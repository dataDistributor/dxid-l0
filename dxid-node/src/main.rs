use anyhow::Result;
use axum::{
    extract::{Path, State},
    http::{HeaderMap, StatusCode},
    response::sse::{Event, Sse},
    routing::{delete, get, post},
    Json, Router,
};
use dxid_crypto::ENGINE as STARK;
use dxid_crypto::StarkSignEngine;
use dxid_runtime::{Chain, State as ChainState, Tx, CHAIN_ID};
use futures_util::{future, stream::Stream, StreamExt};
use parking_lot::Mutex;
use rand::RngCore;
use serde::{Deserialize, Serialize};
use std::{convert::Infallible, fs, path::PathBuf, sync::Arc, time::Duration};
use tokio::{sync::broadcast, time::sleep};
use tokio_stream::wrappers::BroadcastStream;

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
fn apikeys_path(base: &PathBuf) -> PathBuf {
    base.join("apikeys.json")
}
fn webhooks_path(base: &PathBuf) -> PathBuf {
    base.join("webhooks.json")
}

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
struct ApiKeyStore {
    keys: Vec<ApiKey>,
}
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
        self.keys
            .iter()
            .find(|k| k.enabled && k.secret == sec)
            .cloned()
    }
}

/* ---------- Webhooks ---------- */

#[derive(Clone, Serialize, Deserialize)]
struct Webhook {
    id: String,
    api_key_id: String, // owner
    url: String,
    events: Vec<String>, // e.g., ["block", "transfer_to:deadbeef..."]
    created_at: u64,
    enabled: bool,
}
#[derive(Clone, Default, Serialize, Deserialize)]
struct WebhookStore {
    hooks: Vec<Webhook>,
}
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

#[tokio::main]
async fn main() -> Result<()> {
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

    // RPC
    let ctx = RpcCtx {
        state: chain.state.clone(),
        mempool_dir: chain.mempool_dir.clone(),
        blocks_dir: chain.blocks_dir.clone(),
        base_dir: base.clone(),
        admin_token,
        sse_tx,
    };
    let app = Router::new()
        // Open endpoints
        .route("/health", get(health))
        .route("/status", get(status))
        .route("/watch", get(watch))
        // API-key endpoints
        .route("/balance/:addr", get(balance))
        .route("/block/:height", get(block_by_height))
        .route("/submitTx", post(submit_tx))
        // Admin endpoints
        .route("/admin/apikeys", post(admin_create_key).get(admin_list_keys))
        .route("/admin/apikeys/:id", delete(admin_delete_key))
        .route("/admin/webhooks", post(admin_add_webhook).get(admin_list_webhooks))
        .route("/admin/webhooks/:id", delete(admin_delete_webhook))
        .with_state(ctx.clone());

    tokio::spawn(async move {
        let addr = "127.0.0.1:8545";
        let listener = tokio::net::TcpListener::bind(addr).await.unwrap();
        println!("RPC listening on http://{}", addr);
        if let Err(e) = axum::serve(listener, app).await {
            eprintln!("RPC server error: {e}");
        }
    });

    // Block production loop (+ webhooks + SSE)
    println!("dxID L0 (devnet) running. Apps can use API keys + webhooks now.");
    let http = reqwest::Client::new();

    loop {
        if let Ok(Some(block)) = chain.make_block_once() {
            // broadcast SSE
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
                            "timestamp": block.header.timestamp
                        });
                        let url = hook.url.clone();
                        let http = http.clone();
                        tokio::spawn(async move {
                            let _ = http.post(url).json(&body).send().await;
                        });
                    } else if let Some(hex_addr) = ev.strip_prefix("transfer_to:") {
                        let want = hex_addr.to_lowercase();
                        for tx in &block.txs {
                            if hex::encode(tx.to).to_lowercase() == want {
                                let body = serde_json::json!({
                                    "event":"transfer_to",
                                    "to": hex::encode(tx.to),
                                    "from": hex::encode(tx.from),
                                    "amount": tx.amount,
                                    "fee": tx.fee,
                                    "nonce": tx.signature.nonce,
                                    "height": block.header.height,
                                    "state_root": hex::encode(block.header.state_root),
                                    "timestamp": block.header.timestamp
                                });
                                let url = hook.url.clone();
                                let http = http.clone();
                                tokio::spawn(async move {
                                    let _ = http.post(url).json(&body).send().await;
                                });
                            }
                        }
                    }
                }
            }

            println!(
                "⛓  built block h={} txs={} root={}",
                block.header.height,
                block.txs.len(),
                hex::encode(block.header.tx_root)
            );
        }
        sleep(Duration::from_millis(2000)).await;
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
        if let Ok(tok) = val.to_str() {
            return tok == ctx.admin_token;
        }
    }
    false
}

/* ---------- Open endpoints ---------- */

async fn health() -> Json<serde_json::Value> {
    Json(serde_json::json!({ "ok": true }))
}

#[derive(Serialize)]
struct StatusResp {
    height: u64,
    last_block_hash: String,
    state_root: String,
    chain_id: u32,
}
async fn status(State(ctx): State<RpcCtx>) -> Json<StatusResp> {
    let st = ctx.state.lock();
    Json(StatusResp {
        height: st.height,
        last_block_hash: hex::encode(st.last_block_hash),
        state_root: hex::encode(st.state_root),
        chain_id: CHAIN_ID,
    })
}

async fn watch(State(ctx): State<RpcCtx>) -> Sse<impl Stream<Item = Result<Event, Infallible>>> {
    let rx = ctx.sse_tx.subscribe();

    // FIX: make the closure async-compliant using `future::ready`
    let stream = BroadcastStream::new(rx)
        .filter_map(|msg| future::ready(msg.ok()))
        .map(|json| Ok(Event::default().json_data(json).unwrap()));

    Sse::new(stream)
}

/* ---------- API-key endpoints ---------- */

#[derive(Serialize)]
struct BalanceResp {
    exists: bool,
    balance: String,
    nonce: u64,
}
async fn balance(
    State(ctx): State<RpcCtx>,
    headers: HeaderMap,
    Path(addr_hex): Path<String>,
) -> (StatusCode, Json<BalanceResp>) {
    if !require_api(&headers, &ctx) {
        return (
            StatusCode::UNAUTHORIZED,
            Json(BalanceResp {
                exists: false,
                balance: "0".into(),
                nonce: 0,
            }),
        );
    }
    let st = ctx.state.lock();
    let key = addr_hex.to_lowercase();
    if let Some(acct) = st.accounts.get(&key) {
        (
            StatusCode::OK,
            Json(BalanceResp {
                exists: true,
                balance: acct.balance.to_string(),
                nonce: acct.nonce,
            }),
        )
    } else {
        (
            StatusCode::NOT_FOUND,
            Json(BalanceResp {
                exists: false,
                balance: "0".into(),
                nonce: 0,
            }),
        )
    }
}

async fn block_by_height(
    State(ctx): State<RpcCtx>,
    headers: HeaderMap,
    Path(height): Path<u64>,
) -> (StatusCode, String) {
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
struct SubmitTxResp {
    queued: bool,
    file: String,
}

async fn submit_tx(
    State(ctx): State<RpcCtx>,
    headers: HeaderMap,
    Json(body): Json<SubmitTxReq>,
) -> (StatusCode, Json<SubmitTxResp>) {
    if !require_api(&headers, &ctx) {
        return (
            StatusCode::UNAUTHORIZED,
            Json(SubmitTxResp {
                queued: false,
                file: "".into(),
            }),
        );
    }
    fn hex32(s: &str) -> Option<[u8; 32]> {
        let v = hex::decode(s).ok()?;
        if v.len() != 32 {
            return None;
        }
        let mut out = [0u8; 32];
        out.copy_from_slice(&v);
        Some(out)
    }
    let Some(from) = hex32(&body.from) else {
        return (
            StatusCode::BAD_REQUEST,
            Json(SubmitTxResp {
                queued: false,
                file: "".into(),
            }),
        );
    };
    let Some(to) = hex32(&body.to) else {
        return (
            StatusCode::BAD_REQUEST,
            Json(SubmitTxResp {
                queued: false,
                file: "".into(),
            }),
        );
    };

    if body.signature.pubkey_hash != from {
        return (
            StatusCode::BAD_REQUEST,
            Json(SubmitTxResp {
                queued: false,
                file: "".into(),
            }),
        );
    }
    let msg =
        serde_json::to_vec(&(from, to, body.amount, body.fee, body.signature.nonce, CHAIN_ID))
            .expect("serialize");
    if let Err(_) = STARK.verify(&body.signature, &msg) {
        return (
            StatusCode::BAD_REQUEST,
            Json(SubmitTxResp {
                queued: false,
                file: "".into(),
            }),
        );
    }

    let tx = Tx {
        from,
        to,
        amount: body.amount,
        fee: body.fee,
        signature: body.signature,
    };
    let fname = format!("{}.json", uuid::Uuid::new_v4());
    let path = ctx.mempool_dir.join(&fname);
    if let Err(_) =
        std::fs::write(&path, serde_json::to_string_pretty(&tx).unwrap())
    {
        return (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(SubmitTxResp {
                queued: false,
                file: "".into(),
            }),
        );
    }

    (
        StatusCode::OK,
        Json(SubmitTxResp {
            queued: true,
            file: path.to_string_lossy().to_string(),
        }),
    )
}

/* ---------- Admin endpoints ---------- */

#[derive(Deserialize)]
struct AdminCreateKeyReq {
    name: String,
}
#[derive(Serialize)]
struct AdminCreateKeyResp {
    id: String,
    secret: String,
    created_at: u64,
    enabled: bool,
}
async fn admin_create_key(
    State(ctx): State<RpcCtx>,
    headers: HeaderMap,
    Json(body): Json<AdminCreateKeyReq>,
) -> (StatusCode, Json<AdminCreateKeyResp>) {
    if !require_admin(&headers, &ctx) {
        return (
            StatusCode::UNAUTHORIZED,
            Json(AdminCreateKeyResp {
                id: "".into(),
                secret: "".into(),
                created_at: 0,
                enabled: false,
            }),
        );
    }
    let mut store = ApiKeyStore::load(&apikeys_path(&ctx.base_dir));
    let mut idb = [0u8; 8];
    rand::thread_rng().fill_bytes(&mut idb);
    let id = hex::encode(idb);
    let mut secb = [0u8; 32];
    rand::thread_rng().fill_bytes(&mut secb);
    let secret = hex::encode(secb);
    let key = ApiKey {
        id: id.clone(),
        name: body.name,
        secret: secret.clone(),
        created_at: now_ts(),
        enabled: true,
    };
    store.keys.push(key);
    let _ = store.save(&apikeys_path(&ctx.base_dir));
    (
        StatusCode::OK,
        Json(AdminCreateKeyResp {
            id,
            secret,
            created_at: now_ts(),
            enabled: true,
        }),
    )
}

#[derive(Serialize)]
struct AdminListKeysResp {
    keys: Vec<ApiKey>,
}
async fn admin_list_keys(
    State(ctx): State<RpcCtx>,
    headers: HeaderMap,
) -> (StatusCode, Json<AdminListKeysResp>) {
    if !require_admin(&headers, &ctx) {
        return (StatusCode::UNAUTHORIZED, Json(AdminListKeysResp { keys: vec![] }));
    }
    let store = ApiKeyStore::load(&apikeys_path(&ctx.base_dir));
    (StatusCode::OK, Json(AdminListKeysResp { keys: store.keys }))
}

async fn admin_delete_key(
    State(ctx): State<RpcCtx>,
    headers: HeaderMap,
    Path(id): Path<String>,
) -> StatusCode {
    if !require_admin(&headers, &ctx) {
        return StatusCode::UNAUTHORIZED;
    }
    let mut store = ApiKeyStore::load(&apikeys_path(&ctx.base_dir));
    if let Some(k) = store.keys.iter_mut().find(|k| k.id == id) {
        k.enabled = false;
    }
    let _ = store.save(&apikeys_path(&ctx.base_dir));
    StatusCode::OK
}

#[derive(Deserialize)]
struct AdminAddWebhookReq {
    api_key_id: String,
    url: String,
    events: Vec<String>,
}
#[derive(Serialize)]
struct AdminAddWebhookResp {
    id: String,
}
async fn admin_add_webhook(
    State(ctx): State<RpcCtx>,
    headers: HeaderMap,
    Json(body): Json<AdminAddWebhookReq>,
) -> (StatusCode, Json<AdminAddWebhookResp>) {
    if !require_admin(&headers, &ctx) {
        return (
            StatusCode::UNAUTHORIZED,
            Json(AdminAddWebhookResp { id: "".into() }),
        );
    }
    let mut store = WebhookStore::load(&webhooks_path(&ctx.base_dir));
    let id = uuid::Uuid::new_v4().to_string();
    let hook = Webhook {
        id: id.clone(),
        api_key_id: body.api_key_id,
        url: body.url,
        events: body.events,
        created_at: now_ts(),
        enabled: true,
    };
    store.hooks.push(hook);
    let _ = store.save(&webhooks_path(&ctx.base_dir));
    (StatusCode::OK, Json(AdminAddWebhookResp { id }))
}

#[derive(Serialize)]
struct AdminListWebhooksResp {
    hooks: Vec<Webhook>,
}
async fn admin_list_webhooks(
    State(ctx): State<RpcCtx>,
    headers: HeaderMap,
) -> (StatusCode, Json<AdminListWebhooksResp>) {
    if !require_admin(&headers, &ctx) {
        return (StatusCode::UNAUTHORIZED, Json(AdminListWebhooksResp { hooks: vec![] }));
    }
    let store = WebhookStore::load(&webhooks_path(&ctx.base_dir));
    (StatusCode::OK, Json(AdminListWebhooksResp { hooks: store.hooks }))
}

async fn admin_delete_webhook(
    State(ctx): State<RpcCtx>,
    headers: HeaderMap,
    Path(id): Path<String>,
) -> StatusCode {
    if !require_admin(&headers, &ctx) {
        return StatusCode::UNAUTHORIZED;
    }
    let mut store = WebhookStore::load(&webhooks_path(&ctx.base_dir));
    if let Some(h) = store.hooks.iter_mut().find(|h| h.id == id) {
        h.enabled = false;
    }
    let _ = store.save(&webhooks_path(&ctx.base_dir));
    StatusCode::OK
}

/* ---------- util ---------- */

fn now_ts() -> u64 {
    use std::time::{SystemTime, UNIX_EPOCH};
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs()
}
