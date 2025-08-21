use anyhow::Result;
use axum::{
    extract::{Path, State},
    http::{HeaderMap, StatusCode},
    response::sse::{Event, Sse},
    routing::{delete, get, post},
    Json, Router,
};
use clap::Parser;
use dxid_crypto::ENGINE as STARK;
use dxid_crypto::StarkSignEngine;
use dxid_runtime::{Chain, State as ChainState, CHAIN_ID, Account};
use futures_util::{future, stream::Stream, StreamExt};
use hmac::{Hmac, Mac};
use once_cell::sync::OnceCell;
use parking_lot::Mutex;
use rand::RngCore;
use serde::{Deserialize, Serialize};
use sha2::Sha256;
use std::{convert::Infallible, fs, path::PathBuf, sync::Arc, time::Duration};
use tokio::{sync::broadcast, time::sleep};
use tracing::{info, warn};

// P2P
use dxid_p2p::{self, NetConfig, Network};
use dxid_p2p::types::{GossipBlock, GossipTx, Hello};

// Global P2P handle
static P2P_NET: OnceCell<Arc<Network>> = OnceCell::new();

// Network discovery is now handled by the P2P discovery service
// No more file-based discovery needed - it's all automatic!

/* ---------- Admin token & storage paths ---------- */

fn ensure_admin_token(base: &PathBuf) -> Result<String> {
    let p = base.join("admin_token.txt");
    if let Ok(tok) = fs::read_to_string(&p) {
        return Ok(tok.trim().to_string());
    }
    let mut bytes = [0u8; 32];
    rand::thread_rng().fill_bytes(&mut bytes);
    let tok = hex::encode(bytes);
    fs::write(&p, &tok)?;
    Ok(tok)
}
fn apikeys_path(base: &PathBuf) -> PathBuf { base.join("apikeys.json") }
fn webhooks_path(base: &PathBuf) -> PathBuf { base.join("webhooks.json") }

/* ---------- API keys ---------- */

#[derive(Clone, Serialize, Deserialize)]
struct ApiKey {
    id: String,
    name: String,
    secret: String,
    created_at: u64,
    enabled: bool,
}
#[derive(Clone, Default, Serialize, Deserialize)]
struct ApiKeyStore { keys: Vec<ApiKey> }
impl ApiKeyStore {
    fn load(path: &PathBuf) -> Self {
        fs::read_to_string(path)
            .ok()
            .and_then(|s| serde_json::from_str(&s).ok())
            .unwrap_or_default()
    }
    fn save(&self, path: &PathBuf) -> Result<()> {
        fs::write(path, serde_json::to_string_pretty(self)?)?;
        Ok(())
    }
    fn find_by_secret(&self, sec: &str) -> Option<ApiKey> {
        self.keys.iter().find(|k| k.enabled && k.secret == sec).cloned()
    }
}

/* ---------- Network Discovery ---------- */
// All network discovery is now handled automatically by the P2P discovery service
// No manual configuration needed!

/* ---------- Webhooks ---------- */

#[derive(Clone, Serialize, Deserialize)]
struct Webhook {
    id: String,
    api_key_id: String,
    url: String,
    events: Vec<String>,
    secret: String,
    created_at: u64,
    enabled: bool,
}
#[derive(Clone, Default, Serialize, Deserialize)]
struct WebhookStore { hooks: Vec<Webhook> }
impl WebhookStore {
    fn load(path: &PathBuf) -> Self {
        fs::read_to_string(path)
            .ok()
            .and_then(|s| serde_json::from_str(&s).ok())
            .unwrap_or_default()
    }
    fn save(&self, path: &PathBuf) -> Result<()> {
        fs::write(path, serde_json::to_string_pretty(self)?)?;
        Ok(())
    }
}

/* ---------- RPC context ---------- */

#[derive(Clone)]
struct RpcCtx {
    state: Arc<Mutex<ChainState>>,
    mempool_dir: PathBuf,
    blocks_dir: PathBuf,
    base_dir: PathBuf,
    admin_token: String,
    sse_tx: broadcast::Sender<String>, // JSON events
}

/* ---------- CLI opts ---------- */

#[derive(Parser, Debug, Clone)]
#[command(name="dxid-node", version)]
struct Opts {
    /// Enable P2P gossip (default: true)
    #[arg(long, default_value = "true")]
    p2p: bool,

    /// P2P listen address
    #[arg(long, default_value = "0.0.0.0:7000")]
    p2p_listen: String,

    /// Custom bootstrap peers (optional)
    #[arg(long)]
    p2p_bootstrap: Vec<String>,

    /// Disable automatic peer discovery
    #[arg(long)]
    no_discovery: bool,
}

#[tokio::main]
async fn main() -> Result<()> {
    tracing_subscriber::fmt::init();
    let opts = Opts::parse();

    // Genesis faucet
    let (faucet_sk, faucet_pk) = STARK.generate_keys()?;
    println!("GENESIS faucet pubkey: {}", hex::encode(faucet_pk));
    println!(
        "Save this for testing (dev only) faucet secret: {}",
        hex::encode(faucet_sk.bytes)
    );

    // Chain
    let state = ChainState::new_with_genesis(vec![(faucet_pk, 1_000_000_000_000u128)]);
    let base = PathBuf::from("./dxid-data");
    let chain = Chain::new(state, base.clone(), 2000)?;
    let chain = Arc::new(chain);

    // Admin token (persisted)
    let admin_token = ensure_admin_token(&base)?;
    println!("Admin token file: {}/admin_token.txt", base.display());

    // SSE broadcast channel
    let (sse_tx, _sse_rx) = broadcast::channel::<String>(256);

    // RPC ctx
    let ctx = RpcCtx {
        state: chain.state.clone(),
        mempool_dir: chain.mempool_dir.clone(),
        blocks_dir: chain.blocks_dir.clone(),
        base_dir: base.clone(),
        admin_token,
        sse_tx,
    };

    // ===== P2P Network Startup =====
    // Derive a "genesis hash" from current state root + height 0 marker.
    let genesis_hash = {
        let st = ctx.state.lock();
        let seed = serde_json::json!({
            "state_root": hex::encode(st.state_root),
            "height": 0u64,
            "chain_id": CHAIN_ID
        });
        hex::encode(*blake3::hash(seed.to_string().as_bytes()).as_bytes())
    };

    let maybe_net: Option<Arc<Network>> = if opts.p2p {
        println!("🌐 Starting P2P network with automatic peer discovery...");
        
        let mut net = dxid_p2p::start(NetConfig {
            chain_id: CHAIN_ID,
            genesis_hash,
            listen_addr: opts.p2p_listen.clone(),
            bootstrap_peers: opts.p2p_bootstrap.clone(),
            enable_encryption: true,
            max_peers: 50,
            heartbeat_interval: 30,
            auto_discovery: !opts.no_discovery,
            discovery_interval: 60,
        }).await?;
        
        // Start listening
        net.start_listening().await?;
        
        // Spawn P2P listener (handles incoming connections)
        let net_clone = net.clone();
        tokio::spawn(async move {
            if let Err(e) = net_clone.run_listener().await {
                eprintln!("P2P listener error: {}", e);
            }
        });
        
        // Spawn P2P event loop (handles messages and heartbeat)
        let net_clone = net.clone();
        tokio::spawn(async move {
            if let Err(e) = net_clone.run_event_loop().await {
                eprintln!("P2P event loop error: {}", e);
            }
        });
        
        // Spawn P2P discovery (automatically finds and connects to peers)
        let net_clone = net.clone();
        tokio::spawn(async move {
            if let Err(e) = net_clone.run_discovery().await {
                eprintln!("P2P discovery error: {}", e);
            }
        });
        
        let net = Arc::new(net);
        P2P_NET.set(net.clone()).ok();
        println!("✅ P2P network started successfully with automatic peer discovery");

        Some(net)
    } else {
        None
    };

    // ===== HTTP RPC =====
    let app = Router::new()
        // Open endpoints
        .route("/health", get(health))
        .route("/debug", get(debug_info))
        .route("/status", get(status))
        .route("/watch", get(watch))
        .route("/peers", get(peers))
        .route("/network", get(network_status))
        // V1 trust-minimized endpoints (real SMT)
        .route("/v1/proveAccount/:addr", get(v1_prove_account))
        .route("/v1/verifyProof", post(v1_verify_proof))
        // API-key endpoints
        .route("/balance/:addr", get(balance))
        .route("/block/:height", get(block_by_height))
        .route("/submitTx", post(submit_tx))
        .route("/layer0/transfer", post(layer0_transfer))
        .route("/longyield/transfer", post(longyield_transfer))
        // Admin endpoints
        .route("/admin/apikeys", post(admin_create_key).get(admin_list_keys))
        .route("/admin/apikeys/:id", delete(admin_delete_key))
        .route("/admin/webhooks", post(admin_add_webhook).get(admin_list_webhooks))
        .route("/admin/webhooks/:id", delete(admin_delete_webhook))
        .with_state(ctx.clone());

    tokio::spawn({
        let app = app.clone();
        async move {
            // Use Railway's PORT environment variable, fallback to 8545
            let port = std::env::var("PORT").unwrap_or_else(|_| "8545".to_string());
            let addr = format!("0.0.0.0:{}", port);
            let listener = tokio::net::TcpListener::bind(&addr).await.unwrap();
            println!("RPC listening on http://{}", addr);
            if let Err(e) = axum::serve(listener, app).await {
                eprintln!("RPC server error: {e}");
            }
        }
    });

    // ===== P2P inbound consumers =====
    // Note: P2P integration is simplified - no broadcast channels for now
    // TODO: Implement proper P2P message handling when broadcast channels are fixed

    // ===== Block production loop (+ webhooks + SSE + P2P block gossip) =====
    println!("dxID L0 (devnet) running with real SMT account proofs.");
    let http = reqwest::Client::new();
    
    // Performance optimization: track last block time to avoid excessive CPU usage
    let mut last_block_time = std::time::Instant::now();
    let block_interval = Duration::from_millis(2000);

    loop {
        // Check if it's time to produce a block
        let elapsed = last_block_time.elapsed();
        if elapsed < block_interval {
            // Sleep for the remaining time to reduce CPU usage
            let sleep_time = block_interval - elapsed;
            sleep(sleep_time).await;
            continue;
        }
        
        // Try to produce a block
        match chain.make_block_once() {
            Ok(Some(block)) => {
                last_block_time = std::time::Instant::now();
                
                // SSE broadcast
                let evt = serde_json::json!({
                    "type": "block",
                    "height": block.header.height,
                    "txs": block.txs.len(),
                    "tx_root": hex::encode(block.header.tx_root),
                    "state_root": hex::encode(block.header.state_root),
                    "timestamp": block.header.timestamp
                })
                .to_string();
                let _ = ctx.sse_tx.send(evt);

                // Send webhooks (block + transfer_to)
                let hooks = WebhookStore::load(&webhooks_path(&ctx.base_dir));
                for hook in hooks.hooks.into_iter().filter(|h| h.enabled) {
                    for ev in &hook.events {
                        if ev == "block" {
                            let body = serde_json::json!({
                                "event":"block",
                                "height": block.header.height,
                                "txs": block.txs.len(),
                                "state_root": hex::encode(block.header.state_root),
                                "timestamp": block.header.timestamp,
                                "proof": {
                                    "root": hex::encode(block.header.state_root),
                                    "height": block.header.height
                                }
                            });
                            send_signed_webhook(&http, &hook, body).await;
                        } else if let Some(hex_addr) = ev.strip_prefix("transfer_to:") {
                            let want = hex_addr.to_lowercase();
                            for tx in &block.txs {
                                if hex::encode(tx.to) == want {
                                    let body = serde_json::json!({
                                        "event": "transfer_to",
                                        "to": hex_addr,
                                        "amount": tx.amount,
                                        "block_height": block.header.height,
                                        "tx_hash": hex::encode(blake3::hash(&serde_json::to_vec(tx).unwrap_or_default()).as_bytes())
                                    });
                                    send_signed_webhook(&http, &hook, body).await;
                                }
                            }
                        }
                    }
                }

                // Gossip the built block
                if let Some(net) = P2P_NET.get() {
                    let gb = GossipBlock {
                        height: block.header.height,
                        hash: format!("block-{}", block.header.height), // Simplified hash
                        parent_hash: format!("block-{}", block.header.height.saturating_sub(1)), // Simplified parent
                        state_root: hex::encode(block.header.state_root),
                        tx_ids: block.txs.iter().map(|tx| {
                            hex::encode(blake3::hash(&serde_json::to_vec(tx).unwrap_or_default()).as_bytes())
                        }).collect(),
                        body: serde_json::to_value(&block).unwrap_or(serde_json::json!({})),
                    };
                    if let Err(e) = net.publish_block(gb).await {
                        warn!("Failed to publish block: {}", e);
                    }
                }

                println!(
                    "⛓  built block h={} txs={} root={}",
                    block.header.height,
                    block.txs.len(),
                    hex::encode(block.header.tx_root)
                );
            }
            Ok(None) => {
                // No block produced, sleep for a short time
                sleep(Duration::from_millis(100)).await;
            }
            Err(e) => {
                warn!("Error producing block: {}", e);
                sleep(Duration::from_millis(500)).await;
            }
        }
    }
}

/* ---------- Helpers ---------- */

fn require_api(headers: &HeaderMap, ctx: &RpcCtx) -> bool {
    if let Some(val) = headers.get("X-Api-Key") {
        if let Ok(sec) = val.to_str() {
            let store = ApiKeyStore::load(&apikeys_path(&ctx.base_dir));
            return store.find_by_secret(sec).is_some();
        }
    }
    false
}
fn require_admin(headers: &HeaderMap, ctx: &RpcCtx) -> bool {
    if let Some(val) = headers.get("X-Admin-Token") {
        if let Ok(tok) = val.to_str() { return tok == ctx.admin_token; }
    }
    false
}

/* ---------- Open endpoints ---------- */

async fn health() -> Json<serde_json::Value> {
    Json(serde_json::json!({ 
        "ok": true, 
        "version": "1.0.2",
        "deployment": "latest",
        "timestamp": std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs(),
        "message": "Railway deployment test - force redeploy"
    }))
}

async fn debug_info() -> Json<serde_json::Value> {
    Json(serde_json::json!({
        "version": "1.0.1",
        "deployment": "latest",
        "endpoints": [
            "/health",
            "/status", 
            "/peers",
            "/network",
            "/balance/:addr",
            "/admin/apikeys"
        ],
        "timestamp": std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs()
    }))
}

#[derive(Serialize)]
struct StatusResp {
    height: u64,
    last_block_hash: String,
    state_root: String,
    chain_id: u32,
}
async fn status(State(ctx): State<RpcCtx>) -> Json<StatusResp> {
    // Try to access state with timeout to prevent deadlocks
    let result = tokio::time::timeout(Duration::from_secs(5), async {
        let st = ctx.state.lock();
        (st.height, st.last_block_hash, st.state_root)
    }).await;
    
    match result {
        Ok((height, last_block_hash, state_root)) => {
            Json(StatusResp {
                height,
                last_block_hash: hex::encode(last_block_hash),
                state_root: hex::encode(state_root),
                chain_id: CHAIN_ID,
            })
        }
        Err(_) => {
            // Timeout or error, return basic status
            Json(StatusResp {
                height: 0,
                last_block_hash: "00000000000000000000000000000000".to_string(),
                state_root: "00000000000000000000000000000000".to_string(),
                chain_id: CHAIN_ID,
            })
        }
    }
}

async fn watch(State(_ctx): State<RpcCtx>) -> Sse<impl Stream<Item = Result<Event, Infallible>>> {
    // TODO: Fix SSE stream after resolving broadcast issues
    let stream = futures_util::stream::empty();
    Sse::new(stream)
}

async fn peers(State(_ctx): State<RpcCtx>) -> Json<serde_json::Value> {
    if let Some(net) = P2P_NET.get() {
        let peers = net.connected_peers().await;
        let stats = net.get_stats().await;
        Json(serde_json::json!({
            "peers": peers,
            "stats": {
                "total_peers": stats.total_peers,
                "connected_peers": stats.connected_peers,
                "peers_with_zk_stark": stats.peers_with_zk_stark,
                "peers_with_zk_snark": stats.peers_with_zk_snark,
                "messages_sent": stats.messages_sent,
                "messages_received": stats.messages_received
            }
        }))
    } else {
        Json(serde_json::json!({ 
            "peers": Vec::<String>::new(),
            "stats": {
                "total_peers": 0,
                "connected_peers": 0,
                "peers_with_zk_stark": 0,
                "peers_with_zk_snark": 0,
                "messages_sent": 0,
                "messages_received": 0
            }
        }))
    }
}

async fn network_status(State(_ctx): State<RpcCtx>) -> Json<serde_json::Value> {
    let mut status = serde_json::json!({
        "auto_discovery_enabled": true,
        "p2p_enabled": false,
        "chain_id": CHAIN_ID,
        "peer_count": 0,
        "discovery_active": false,
    });

    if let Some(net) = P2P_NET.get() {
        let stats = net.get_stats().await;
        status["p2p_enabled"] = serde_json::Value::Bool(true);
        status["peer_count"] = serde_json::Value::Number(serde_json::Number::from(stats.connected_peers));
        status["discovery_active"] = serde_json::Value::Bool(stats.discovery_enabled);
        status["total_peers"] = serde_json::Value::Number(serde_json::Number::from(stats.total_peers));
        status["bootstrap_peers"] = serde_json::Value::Number(serde_json::Number::from(stats.bootstrap_peers));
    }

    Json(status)
}

/* ---------- v1 trust-minimized endpoints (real SMT) ---------- */

#[derive(Serialize, Deserialize)]
struct AccountLeaf {
    addr: String,
    balance: String,
    nonce: u64,
}

#[derive(Serialize, Deserialize)]
struct AccountProof {
    root: String,
    height: u64,
    leaf: AccountLeaf,
    /// Siblings from LSB to MSB (256 entries) — hex-encoded
    path: Vec<String>,
}

#[derive(Serialize, Deserialize)]
struct VerifyReq { proof: AccountProof }
#[derive(Serialize, Deserialize)]
struct VerifyResp { ok: bool, reason: Option<String> }

async fn v1_prove_account(State(ctx): State<RpcCtx>, Path(addr_hex): Path<String>) -> (StatusCode, Json<AccountProof>) {
    let st = ctx.state.lock();
    let (acct_opt, proof) = st.prove_account(&addr_hex);
    let acct = acct_opt.unwrap_or(dxid_runtime::Account { balance: 0, nonce: 0, layer0_balance: 0, longyield_balance: 0 });

    let path: Vec<String> = proof.siblings.iter().map(|s| hex::encode(s)).collect();

    let out = AccountProof {
        root: hex::encode(st.state_root),
        height: st.height,
        leaf: AccountLeaf {
            addr: addr_hex.to_lowercase(),
            balance: acct.balance.to_string(),
            nonce: acct.nonce,
        },
        path,
    };
    (StatusCode::OK, Json(out))
}

async fn v1_verify_proof(Json(req): Json<VerifyReq>) -> (StatusCode, Json<VerifyResp>) {
    use dxid_smt::{SparseMerkleTree, H256};
    fn dehex32(s: &str) -> Option<H256> {
        let v = hex::decode(s).ok()?; if v.len() != 32 { return None; }
        let mut o = [0u8; 32]; o.copy_from_slice(&v); Some(o)
    }
    fn u128_to_h256(x: u128) -> H256 {
        let mut out = [0u8; 32];
        out[16..].copy_from_slice(&x.to_be_bytes());
        out
    }

    let p = req.proof;
    let Some(root) = dehex32(&p.root) else {
        return (StatusCode::BAD_REQUEST, Json(VerifyResp { ok: false, reason: Some("bad root".into()) }));
    };
    let Some(addr) = dehex32(&p.leaf.addr) else {
        return (StatusCode::BAD_REQUEST, Json(VerifyResp { ok: false, reason: Some("bad addr".into()) }));
    };
    let bal = p.leaf.balance.parse::<u128>().unwrap_or(u128::MAX);

    // decode path
    if p.path.len() != 256 {
        return (StatusCode::BAD_REQUEST, Json(VerifyResp { ok: false, reason: Some("bad path len".into()) }));
    }
    let mut siblings = Vec::with_capacity(256);
    for s in p.path.iter() {
        let Some(x) = dehex32(s) else {
            return (StatusCode::BAD_REQUEST, Json(VerifyResp { ok: false, reason: Some("bad sibling".into()) }));
        };
        siblings.push(x);
    }
    let proof = dxid_smt::SmtProof { siblings };

    let ok = SparseMerkleTree::verify(&root, &addr, Some(&u128_to_h256(bal)), &proof);
    let resp = if ok {
        VerifyResp { ok: true, reason: None }
    } else {
        VerifyResp { ok: false, reason: Some("verification failed".into()) }
    };
    (if ok { StatusCode::OK } else { StatusCode::BAD_REQUEST }, Json(resp))
}

/* ---------- API-key endpoints ---------- */

#[derive(Serialize)]
struct BalanceResp {
    exists: bool,
    balance: String,
    nonce: u64,
    layer0_balance: String,
    longyield_balance: String,
}

async fn balance(State(ctx): State<RpcCtx>, headers: HeaderMap, Path(addr_hex): Path<String>)
-> (StatusCode, Json<BalanceResp>) {
    if !require_api(&headers, &ctx) {
        return (StatusCode::UNAUTHORIZED, Json(BalanceResp { 
            exists:false, 
            balance:"0".into(), 
            nonce:0,
            layer0_balance: "0".into(),
            longyield_balance: "0".into(),
        }));
    }
    let st = ctx.state.lock();
    let key = addr_hex.to_lowercase();
    if let Some(acct) = st.accounts.get(&key) {
        (StatusCode::OK, Json(BalanceResp {
            exists: true, 
            balance: acct.balance.to_string(), 
            nonce: acct.nonce,
            layer0_balance: acct.layer0_balance.to_string(),
            longyield_balance: acct.longyield_balance.to_string(),
        }))
    } else {
        (StatusCode::NOT_FOUND, Json(BalanceResp { 
            exists:false, 
            balance:"0".into(), 
            nonce:0,
            layer0_balance: "0".into(),
            longyield_balance: "0".into(),
        }))
    }
}

async fn block_by_height(State(ctx): State<RpcCtx>, headers: HeaderMap, Path(height): Path<u64>)
-> (StatusCode, String) {
    if !require_api(&headers, &ctx) {
        return (StatusCode::UNAUTHORIZED, "{\"error\":\"unauthorized\"}".into());
    }
    let fname = format!("{:016x}.json", height);
    let path = ctx.blocks_dir.join(fname);
    match std::fs::read_to_string(&path) {
        Ok(s) => (StatusCode::OK, s),
        Err(_) => (StatusCode::NOT_FOUND, "{\"error\":\"not found\"}".into()),
    }
}

#[derive(Deserialize)]
struct SubmitTxReq {
    from: String, // hex(32)
    to: String,   // hex(32)
    amount: u128,
    fee: u128,
    signature: dxid_crypto::StarkSignature,
}
#[derive(Serialize)]
struct SubmitTxResp { queued: bool, file: String }

async fn submit_tx(State(ctx): State<RpcCtx>, headers: HeaderMap, Json(body): Json<SubmitTxReq>)
-> (StatusCode, Json<SubmitTxResp>) {
    if !require_api(&headers, &ctx) {
        return (StatusCode::UNAUTHORIZED, Json(SubmitTxResp { queued:false, file:"".into() }));
    }
    fn hex32(s: &str) -> Option<[u8; 32]> {
        let v = hex::decode(s).ok()?;
        if v.len() != 32 { return None; }
        let mut out = [0u8; 32];
        out.copy_from_slice(&v);
        Some(out)
    }
    let Some(from) = hex32(&body.from) else {
        return (StatusCode::BAD_REQUEST, Json(SubmitTxResp { queued:false, file:"".into() }));
    };
    let Some(to) = hex32(&body.to) else {
        return (StatusCode::BAD_REQUEST, Json(SubmitTxResp { queued:false, file:"".into() }));
    };

    if body.signature.pubkey_hash != from {
        return (StatusCode::BAD_REQUEST, Json(SubmitTxResp { queued:false, file:"".into() }));
    }
    let msg = serde_json::to_vec(&(from, to, body.amount, body.fee, body.signature.nonce, CHAIN_ID)).unwrap();
    if STARK.verify(&body.signature, &msg).is_err() {
        return (StatusCode::BAD_REQUEST, Json(SubmitTxResp { queued:false, file:"".into() }));
    }

    let signature = body.signature.clone();
    let tx = dxid_runtime::Tx { 
        from, 
        to, 
        amount: body.amount, 
        fee: body.fee, 
        signature,
        token_type: dxid_runtime::TokenType::Native,
        cross_chain: false,
        target_chain_id: None,
    };
    let fname = format!("{}.json", uuid::Uuid::new_v4());
    let path = ctx.mempool_dir.join(&fname);
    if std::fs::write(&path, serde_json::to_string_pretty(&tx).unwrap()).is_err() {
        return (StatusCode::INTERNAL_SERVER_ERROR, Json(SubmitTxResp { queued:false, file:"".into() }));
    }

    // Gossip the tx
    if let Some(net) = P2P_NET.get() {
        let id = blake3::hash(serde_json::to_string(&tx).unwrap().as_bytes());
        let wire = GossipTx {
            id: hex::encode(id.as_bytes()),
            body: serde_json::json!({
                "from": body.from,
                "to": body.to,
                "amount": body.amount,
                "fee": body.fee,
                "signature": body.signature.clone(),
            }),
        };
        if let Err(e) = net.publish_tx(wire).await {
            warn!("Failed to publish transaction: {}", e);
        }
    }

    (StatusCode::OK, Json(SubmitTxResp { queued:true, file: path.to_string_lossy().to_string() }))
}

// Layer0 Token Transfer Endpoint
#[derive(Deserialize)]
struct Layer0TransferReq {
    from: String, // hex(32)
    to: String,   // hex(32)
    amount: u128,
    fee: u128,
    signature: dxid_crypto::StarkSignature,
}

async fn layer0_transfer(State(ctx): State<RpcCtx>, headers: HeaderMap, Json(body): Json<Layer0TransferReq>)
-> (StatusCode, Json<SubmitTxResp>) {
    if !require_api(&headers, &ctx) {
        return (StatusCode::UNAUTHORIZED, Json(SubmitTxResp { queued:false, file:"".into() }));
    }
    
    fn hex32(s: &str) -> Option<[u8; 32]> {
        let v = hex::decode(s).ok()?;
        if v.len() != 32 { return None; }
        let mut out = [0u8; 32];
        out.copy_from_slice(&v);
        Some(out)
    }
    
    let Some(from) = hex32(&body.from) else {
        return (StatusCode::BAD_REQUEST, Json(SubmitTxResp { queued:false, file:"".into() }));
    };
    let Some(to) = hex32(&body.to) else {
        return (StatusCode::BAD_REQUEST, Json(SubmitTxResp { queued:false, file:"".into() }));
    };

    if body.signature.pubkey_hash != from {
        return (StatusCode::BAD_REQUEST, Json(SubmitTxResp { queued:false, file:"".into() }));
    }
    
    let msg = serde_json::to_vec(&(from, to, body.amount, body.fee, body.signature.nonce, CHAIN_ID)).unwrap();
    if STARK.verify(&body.signature, &msg).is_err() {
        return (StatusCode::BAD_REQUEST, Json(SubmitTxResp { queued:false, file:"".into() }));
    }

    let signature = body.signature.clone();
    let tx = dxid_runtime::Tx { 
        from, 
        to, 
        amount: body.amount, 
        fee: body.fee, 
        signature,
        token_type: dxid_runtime::TokenType::Layer0,
        cross_chain: false,
        target_chain_id: None,
    };
    
    let fname = format!("{}.json", uuid::Uuid::new_v4());
    let path = ctx.mempool_dir.join(&fname);
    if std::fs::write(&path, serde_json::to_string_pretty(&tx).unwrap()).is_err() {
        return (StatusCode::INTERNAL_SERVER_ERROR, Json(SubmitTxResp { queued:false, file:"".into() }));
    }

    // Gossip the tx
    if let Some(net) = P2P_NET.get() {
        let id = blake3::hash(serde_json::to_string(&tx).unwrap().as_bytes());
        let wire = GossipTx {
            id: hex::encode(id.as_bytes()),
            body: serde_json::json!({
                "from": body.from,
                "to": body.to,
                "amount": body.amount,
                "fee": body.fee,
                "signature": body.signature.clone(),
                "token_type": "Layer0",
            }),
        };
        if let Err(e) = net.publish_tx(wire).await {
            warn!("Failed to publish Layer0 transaction: {}", e);
        }
    }

    (StatusCode::OK, Json(SubmitTxResp { queued:true, file: path.to_string_lossy().to_string() }))
}

// LongYield Token Transfer Endpoint
#[derive(Deserialize)]
struct LongYieldTransferReq {
    from: String, // hex(32)
    to: String,   // hex(32)
    amount: u128,
    fee: u128,
    signature: dxid_crypto::StarkSignature,
}

async fn longyield_transfer(State(ctx): State<RpcCtx>, headers: HeaderMap, Json(body): Json<LongYieldTransferReq>)
-> (StatusCode, Json<SubmitTxResp>) {
    if !require_api(&headers, &ctx) {
        return (StatusCode::UNAUTHORIZED, Json(SubmitTxResp { queued:false, file:"".into() }));
    }
    
    fn hex32(s: &str) -> Option<[u8; 32]> {
        let v = hex::decode(s).ok()?;
        if v.len() != 32 { return None; }
        let mut out = [0u8; 32];
        out.copy_from_slice(&v);
        Some(out)
    }
    
    let Some(from) = hex32(&body.from) else {
        return (StatusCode::BAD_REQUEST, Json(SubmitTxResp { queued:false, file:"".into() }));
    };
    let Some(to) = hex32(&body.to) else {
        return (StatusCode::BAD_REQUEST, Json(SubmitTxResp { queued:false, file:"".into() }));
    };

    if body.signature.pubkey_hash != from {
        return (StatusCode::BAD_REQUEST, Json(SubmitTxResp { queued:false, file:"".into() }));
    }
    
    let msg = serde_json::to_vec(&(from, to, body.amount, body.fee, body.signature.nonce, CHAIN_ID)).unwrap();
    if STARK.verify(&body.signature, &msg).is_err() {
        return (StatusCode::BAD_REQUEST, Json(SubmitTxResp { queued:false, file:"".into() }));
    }

    let signature = body.signature.clone();
        let tx = dxid_runtime::Tx {
        from, 
        to, 
        amount: body.amount, 
        fee: body.fee, 
        signature,
        token_type: dxid_runtime::TokenType::LongYield,
        cross_chain: false,
        target_chain_id: None,
    };
    
    let fname = format!("{}.json", uuid::Uuid::new_v4());
    let path = ctx.mempool_dir.join(&fname);
    if std::fs::write(&path, serde_json::to_string_pretty(&tx).unwrap()).is_err() {
        return (StatusCode::INTERNAL_SERVER_ERROR, Json(SubmitTxResp { queued:false, file:"".into() }));
    }

    // Gossip the tx
    if let Some(net) = P2P_NET.get() {
        let id = blake3::hash(serde_json::to_string(&tx).unwrap().as_bytes());
        let wire = GossipTx {
            id: hex::encode(id.as_bytes()),
            body: serde_json::json!({
                "from": body.from,
                "to": body.to,
                "amount": body.amount,
                "fee": body.fee,
                "signature": body.signature.clone(),
                "token_type": "LongYield",
            }),
        };
        if let Err(e) = net.publish_tx(wire).await {
            warn!("Failed to publish LongYield transaction: {}", e);
        }
    }

    (StatusCode::OK, Json(SubmitTxResp { queued:true, file: path.to_string_lossy().to_string() }))
}

/* ---------- Admin endpoints ---------- */

#[derive(Deserialize)]
struct AdminCreateKeyReq { name: String }
#[derive(Serialize)]
struct AdminCreateKeyResp { id: String, secret: String, created_at: u64, enabled: bool }

async fn admin_create_key(State(ctx): State<RpcCtx>, headers: HeaderMap, Json(body): Json<AdminCreateKeyReq>)
-> (StatusCode, Json<AdminCreateKeyResp>) {
    if !require_admin(&headers, &ctx) {
        return (StatusCode::UNAUTHORIZED, Json(AdminCreateKeyResp { id:"".into(), secret:"".into(), created_at:0, enabled:false }));
    }
    let mut store = ApiKeyStore::load(&apikeys_path(&ctx.base_dir));
    let mut idb = [0u8; 8]; rand::thread_rng().fill_bytes(&mut idb);
    let id = hex::encode(idb);
    let mut secb = [0u8; 32]; rand::thread_rng().fill_bytes(&mut secb);
    let secret = hex::encode(secb);
    let key = ApiKey { id: id.clone(), name: body.name, secret: secret.clone(), created_at: now_ts(), enabled: true };
    store.keys.push(key);
    let _ = store.save(&apikeys_path(&ctx.base_dir));
    (StatusCode::OK, Json(AdminCreateKeyResp { id, secret, created_at: now_ts(), enabled: true }))
}

#[derive(Serialize)]
struct AdminListKeysResp { keys: Vec<ApiKey> }
async fn admin_list_keys(State(ctx): State<RpcCtx>, headers: HeaderMap)
-> (StatusCode, Json<AdminListKeysResp>) {
    if !require_admin(&headers, &ctx) {
        return (StatusCode::UNAUTHORIZED, Json(AdminListKeysResp { keys: vec![] }));
    }
    let store = ApiKeyStore::load(&apikeys_path(&ctx.base_dir));
    (StatusCode::OK, Json(AdminListKeysResp { keys: store.keys }))
}

async fn admin_delete_key(State(ctx): State<RpcCtx>, headers: HeaderMap, Path(id): Path<String>) -> StatusCode {
    if !require_admin(&headers, &ctx) { return StatusCode::UNAUTHORIZED; }
    let mut store = ApiKeyStore::load(&apikeys_path(&ctx.base_dir));
    if let Some(k) = store.keys.iter_mut().find(|k| k.id == id) { k.enabled = false; }
    let _ = store.save(&apikeys_path(&ctx.base_dir));
    StatusCode::OK
}

#[derive(Deserialize)]
struct AdminAddWebhookReq { api_key_id: String, url: String, events: Vec<String>, secret: Option<String> }
#[derive(Serialize)]
struct AdminAddWebhookResp { id: String, secret: String }

async fn admin_add_webhook(State(ctx): State<RpcCtx>, headers: HeaderMap, Json(body): Json<AdminAddWebhookReq>)
-> (StatusCode, Json<AdminAddWebhookResp>) {
    if !require_admin(&headers, &ctx) {
        return (StatusCode::UNAUTHORIZED, Json(AdminAddWebhookResp { id: "".into(), secret: "".into() }));
    }
    let mut store = WebhookStore::load(&webhooks_path(&ctx.base_dir));
    let id = uuid::Uuid::new_v4().to_string();
    let secret = if let Some(s) = &body.secret {
        s.clone()
    } else {
        let mut b = [0u8; 32]; rand::thread_rng().fill_bytes(&mut b); hex::encode(b)
    };
    let hook = Webhook { id: id.clone(), api_key_id: body.api_key_id, url: body.url, events: body.events, secret: secret.clone(), created_at: now_ts(), enabled: true };
    store.hooks.push(hook);
    let _ = store.save(&webhooks_path(&ctx.base_dir));
    (StatusCode::OK, Json(AdminAddWebhookResp { id, secret }))
}

#[derive(Serialize)]
struct AdminListWebhooksResp { hooks: Vec<Webhook> }
async fn admin_list_webhooks(State(ctx): State<RpcCtx>, headers: HeaderMap)
-> (StatusCode, Json<AdminListWebhooksResp>) {
    if !require_admin(&headers, &ctx) {
        return (StatusCode::UNAUTHORIZED, Json(AdminListWebhooksResp { hooks: vec![] }));
    }
    let store = WebhookStore::load(&webhooks_path(&ctx.base_dir));
    (StatusCode::OK, Json(AdminListWebhooksResp { hooks: store.hooks }))
}

async fn admin_delete_webhook(State(ctx): State<RpcCtx>, headers: HeaderMap, Path(id): Path<String>) -> StatusCode {
    if !require_admin(&headers, &ctx) { return StatusCode::UNAUTHORIZED; }
    let mut store = WebhookStore::load(&webhooks_path(&ctx.base_dir));
    if let Some(h) = store.hooks.iter_mut().find(|h| h.id == id) { h.enabled = false; }
    let _ = store.save(&webhooks_path(&ctx.base_dir));
    StatusCode::OK
}

/* ---------- util ---------- */

fn now_ts() -> u64 {
    use std::time::{SystemTime, UNIX_EPOCH};
    SystemTime::now().duration_since(UNIX_EPOCH).unwrap_or_default().as_secs()
}

async fn send_signed_webhook(http: &reqwest::Client, hook: &Webhook, body: serde_json::Value) {
    let ts = now_ts().to_string();
    let body_str = body.to_string();

    // HMAC-SHA256 over "<timestamp>.<body>"
    let mut mac = Hmac::<Sha256>::new_from_slice(&hex::decode(&hook.secret).unwrap_or_default())
        .unwrap_or_else(|_| Hmac::<Sha256>::new_from_slice(&[0u8; 32]).unwrap());
    mac.update(ts.as_bytes());
    mac.update(b".");
    mac.update(body_str.as_bytes());
    let sig = hex::encode(mac.finalize().into_bytes());

    let _ = http
        .post(&hook.url)
        .header("X-Dxid-Timestamp", &ts)
        .header("X-Dxid-Signature", format!("sha256={sig}"))
        .json(&body)
        .send()
        .await;
}
