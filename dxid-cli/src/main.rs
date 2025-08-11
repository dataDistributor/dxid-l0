use anyhow::Result;
use clap::{Parser, Subcommand};
use dxid_crypto::{SecretKey, StarkSignEngine, StarkSignature, ENGINE as STARK};
use dxid_runtime::CHAIN_ID;
use inquire::{Confirm, CustomType, Select, Text};
use serde::{Deserialize, Serialize};
use std::{fs, io, path::PathBuf, time::Duration};

#[derive(Parser)]
#[command(name = "dxid-cli", version, about = "dxID L0 Interactive CLI")]
struct Cli {
    #[arg(long)]
    base: Option<String>,
    #[arg(long)]
    rpc: Option<String>,
    #[command(subcommand)]
    cmd: Option<Cmd>,
}

#[derive(Subcommand)]
enum Cmd {
    Tui,
    Keygen,
    Transfer {
        #[arg(long)]
        secret_hex: String,
        #[arg(long)]
        from: String,
        #[arg(long)]
        to: String,
        #[arg(long)]
        amount: u128,
        #[arg(long, default_value_t = 10u128)]
        fee: u128,
        #[arg(long, default_value_t = 0u64)]
        nonce: u64,
        #[arg(long)]
        rpc: Option<String>,
    },
}

#[derive(Clone, Serialize, Deserialize)]
struct Config {
    base: String,
    rpc_url: String,
    admin_token: String,
}
impl Default for Config {
    fn default() -> Self {
        Self {
            base: "./dxid-data".to_string(),
            rpc_url: "http://127.0.0.1:8545".to_string(),
            admin_token: "".to_string(),
        }
    }
}

#[derive(Clone, Serialize, Deserialize, Default)]
struct Keystore {
    keys: Vec<WalletEntry>,
}
#[derive(Clone, Serialize, Deserialize)]
struct WalletEntry {
    name: String,
    pubkey: [u8; 32],
    secret: [u8; 32],
    local_nonce: u64,
}

#[derive(Clone, Serialize, Deserialize, Default)]
struct Contacts {
    entries: Vec<Contact>,
}
#[derive(Clone, Serialize, Deserialize)]
struct Contact {
    name: String,
    pubkey: [u8; 32],
}

#[derive(Clone, Serialize, Deserialize)]
struct TxFile {
    pub from: [u8; 32],
    pub to: [u8; 32],
    pub amount: u128,
    pub fee: u128,
    pub signature: StarkSignature,
}

/* ---------- API types ---------- */
#[derive(Deserialize)]
struct StatusResp {
    height: u64,
    last_block_hash: String,
    state_root: String,
    chain_id: u32,
}
#[derive(Deserialize)]
struct BalanceResp {
    exists: bool,
    balance: String,
    nonce: u64,
}
#[derive(Deserialize)]
struct AdminListKeysResp {
    keys: Vec<ApiKey>,
}
#[derive(Deserialize, Serialize, Clone)]
struct ApiKey {
    id: String,
    name: String,
    secret: String,
    created_at: u64,
    enabled: bool,
}
#[derive(Deserialize)]
struct AdminAddWebhookResp {
    id: String,
}
#[derive(Deserialize)]
struct AdminListWebhooksResp {
    hooks: Vec<Webhook>,
}
#[derive(Deserialize, Serialize, Clone)]
struct Webhook {
    id: String,
    api_key_id: String,
    url: String,
    events: Vec<String>,
    created_at: u64,
    enabled: bool,
}

fn main() -> Result<()> {
    let cli = Cli::parse();
    fs::create_dir_all(dxid_dir())?;
    let mut cfg = load_config().unwrap_or_default();

    if let Some(base) = &cli.base {
        cfg.base = base.clone();
    }
    if let Some(rpc) = &cli.rpc {
        cfg.rpc_url = rpc.clone();
    }
    save_config(&cfg)?;

    match cli.cmd {
        None | Some(Cmd::Tui) => run_tui(cfg),
        Some(Cmd::Keygen) => cmd_keygen(),
        Some(Cmd::Transfer {
            secret_hex,
            from,
            to,
            amount,
            fee,
            nonce,
            rpc,
        }) => {
            let mut cfg2 = cfg.clone();
            if let Some(r) = rpc {
                cfg2.rpc_url = r;
            }
            cmd_transfer_noninteractive(&cfg2, secret_hex, from, to, amount, fee, nonce)
        }
    }
}

fn run_tui(mut cfg: Config) -> Result<()> {
    loop {
        let height = get_status(&cfg).ok().map(|s| s.height).unwrap_or(0);
        let title = format!("dxID CLI  •  rpc: {}  •  height: {}", cfg.rpc_url, height);
        let choice = Select::new(
            &title,
            vec![
                "Wallets",
                "Transfer",
                "Contacts",
                "Developers / API",
                "Node Status",
                "Settings",
                "Quit",
            ],
        )
        .prompt()?;

        match choice {
            "Wallets" => tui_wallets(&cfg)?,
            "Transfer" => tui_transfer(&cfg)?,
            "Contacts" => tui_contacts()?,
            "Developers / API" => tui_developers(&mut cfg)?,
            "Node Status" => {
                print_status(&cfg)?;
                wait_enter()?;
            }
            "Settings" => tui_settings(&mut cfg)?,
            "Quit" => break,
            _ => {}
        }
    }
    Ok(())
}

/* ---------- Developers / API ---------- */
fn tui_developers(cfg: &mut Config) -> Result<()> {
    loop {
        let choice = Select::new(
            "Developers / API",
            vec![
                "Create API Key",
                "List API Keys",
                "Add Webhook",
                "List Webhooks",
                "Delete Webhook",
                "Back",
            ],
        )
        .prompt()?;
        match choice {
            "Create API Key" => {
                ensure_admin(cfg)?;
                let name = Text::new("Key name (for your app):").with_default("default").prompt()?;
                let created = admin_create_key(cfg, &name)?;
                println!("API Key created:");
                println!("  id:     {}", created.id);
                println!("  secret: {}", created.secret);
                println!("(Store this secret in your app; it will authenticate with X-Api-Key)");
                wait_enter()?;
            }
            "List API Keys" => {
                ensure_admin(cfg)?;
                let list = admin_list_keys(cfg)?;
                println!("API Keys:");
                for k in &list.keys {
                    println!(
                        "- {}  id={}  enabled={}  secret={}",
                        k.name, k.id, k.enabled, k.secret
                    );
                }
                wait_enter()?;
            }
            "Add Webhook" => {
                ensure_admin(cfg)?;
                let list = admin_list_keys(cfg)?;
                if list.keys.is_empty() {
                    println!("No API keys yet; create one first.");
                    wait_enter()?;
                    continue;
                }
                let items: Vec<String> = list
                    .keys
                    .iter()
                    .map(|k| format!("{} (id={}…)", k.name, &k.id[..8]))
                    .collect();
                let pick = Select::new("Which API key owns this webhook?", items).prompt()?;
                let idx = list
                    .keys
                    .iter()
                    .position(|k| pick.contains(&k.id[..8]))
                    .unwrap();
                let key = &list.keys[idx];
                let url = Text::new("Webhook URL (https://...):").prompt()?;
                let ev_choice = Select::new(
                    "Event type",
                    vec!["block", "transfer_to:<address-hex>"],
                )
                .prompt()?;
                let mut events = Vec::new();
                if ev_choice.starts_with("transfer_to") {
                    let to_hex = Text::new("Address (32-byte hex):").prompt()?;
                    events.push(format!("transfer_to:{to_hex}"));
                } else {
                    events.push("block".into());
                }
                let id = admin_add_webhook(cfg, &key.id, &url, events)?;
                println!("Webhook created: id={}", id);
                wait_enter()?;
            }
            "List Webhooks" => {
                ensure_admin(cfg)?;
                let w = admin_list_webhooks(cfg)?;
                println!("Webhooks:");
                for h in &w.hooks {
                    println!(
                        "- id={} url={} enabled={} events={:?} key_id={}",
                        h.id, h.url, h.enabled, h.events, h.api_key_id
                    );
                }
                wait_enter()?;
            }
            "Delete Webhook" => {
                ensure_admin(cfg)?;
                let w = admin_list_webhooks(cfg)?;
                if w.hooks.is_empty() {
                    println!("No webhooks.");
                    wait_enter()?;
                    continue;
                }
                let items: Vec<String> = w
                    .hooks
                    .iter()
                    .map(|h| format!("{}  ({})", h.id, h.url))
                    .collect();
                let pick = Select::new("Select webhook to delete", items).prompt()?;
                let chosen = w
                    .hooks
                    .iter()
                    .find(|h| pick.contains(&h.id))
                    .unwrap();
                admin_delete_webhook(cfg, &chosen.id)?;
                println!("Deleted.");
                wait_enter()?;
            }
            "Back" => break,
            _ => {}
        }
    }
    Ok(())
}

fn ensure_admin(cfg: &mut Config) -> Result<()> {
    if cfg.admin_token.trim().is_empty() {
        println!("Admin token required (printed once when the node first started).");
        cfg.admin_token = Text::new("Enter admin token:").prompt()?;
        save_config(cfg)?;
    }
    Ok(())
}

/* ---------- Wallets / Contacts / Transfer ---------- */

fn tui_wallets(cfg: &Config) -> Result<()> {
    loop {
        let ks = load_keystore().unwrap_or_default();
        let mut items: Vec<String> = Vec::new();
        for k in &ks.keys {
            let remote = get_balance(cfg, &k.pubkey).ok();
            let rn = remote.as_ref().map(|b| b.nonce).unwrap_or(k.local_nonce);
            items.push(format!(
                "{}  ({}…)  [nonce={}]",
                k.name,
                hex::encode(&k.pubkey)[..8].to_string(),
                rn
            ));
        }
        items.push("➕  Create new".into());
        items.push("📥  Import existing (dev)".into());
        items.push("⬅️  Back".into());

        let choice = Select::new("Wallets", items).prompt()?;
        if choice == "⬅️  Back" {
            break;
        } else if choice.starts_with('➕') {
            let name = Text::new("Name for wallet:").with_default("default").prompt()?;
            let (sk, pk) = STARK.generate_keys()?;
            let mut keystore = load_keystore().unwrap_or_default();
            keystore.keys.push(WalletEntry {
                name,
                pubkey: pk,
                secret: sk.bytes,
                local_nonce: 0,
            });
            save_keystore(&keystore)?;
            println!("Created wallet. Address: {}", hex::encode(pk));
            wait_enter()?;
        } else if choice.starts_with('📥') {
            let name = Text::new("Name for wallet:").prompt()?;
            let secret_hex = Text::new("Secret (hex, 32 bytes) [dev]:").prompt()?;
            let pubkey_hex = Text::new("Address/pubkey (hex, 32 bytes):").prompt()?;
            let nonce: u64 =
                CustomType::new("Starting nonce? (usually 0)").with_default(0u64).prompt()?;

            let secret = hex32(&secret_hex)?;
            let pubkey = hex32(&pubkey_hex)?;
            let mut keystore = load_keystore().unwrap_or_default();
            keystore.keys.push(WalletEntry {
                name,
                pubkey,
                secret,
                local_nonce: nonce,
            });
            save_keystore(&keystore)?;
            println!("Imported wallet {}", hex::encode(pubkey));
            wait_enter()?;
        } else {
            let idx = wallet_index_from_label(&ks, &choice);
            if let Some(i) = idx {
                wallet_actions(i)?;
            }
        }
    }
    Ok(())
}

fn wallet_actions(index: usize) -> Result<()> {
    let ks = load_keystore().unwrap_or_default();
    let Some(entry) = ks.keys.get(index).cloned() else {
        return Ok(());
    };
    loop {
        let choice = Select::new(
            &format!("Wallet: {} ({})", entry.name, hex::encode(entry.pubkey)),
            vec![
                "Show address",
                "Show dev secret (hex)",
                "Set local nonce",
                "Delete wallet",
                "Back",
            ],
        )
        .prompt()?;
        match choice {
            "Show address" => {
                println!("Address: {}", hex::encode(entry.pubkey));
                wait_enter()?;
            }
            "Show dev secret (hex)" => {
                println!("Secret [dev]: {}", hex::encode(entry.secret));
                wait_enter()?;
            }
            "Set local nonce" => {
                let new_nonce: u64 = CustomType::new("New nonce:").prompt()?;
                let mut ks2 = load_keystore().unwrap_or_default();
                ks2.keys[index].local_nonce = new_nonce;
                save_keystore(&ks2)?;
                println!("Updated local nonce to {}", new_nonce);
                wait_enter()?;
            }
            "Delete wallet" => {
                let ok = Confirm::new("Really delete this wallet?").with_default(false).prompt()?;
                if ok {
                    let mut ks2 = load_keystore().unwrap_or_default();
                    ks2.keys.remove(index);
                    save_keystore(&ks2)?;
                    println!("Deleted.");
                    wait_enter()?;
                    break;
                }
            }
            "Back" => break,
            _ => {}
        }
    }
    Ok(())
}

fn tui_contacts() -> Result<()> {
    loop {
        let book = load_contacts().unwrap_or_default();
        let mut items: Vec<String> = book
            .entries
            .iter()
            .map(|c| format!("{}  ({}…)", c.name, hex::encode(&c.pubkey)[..8].to_string()))
            .collect();
        items.push("➕  Add contact".into());
        items.push("⬅️  Back".into());

        let choice = Select::new("Contacts", items).prompt()?;
        if choice == "⬅️  Back" {
            break;
        } else if choice.starts_with('➕') {
            let name = Text::new("Contact name:").prompt()?;
            let addr_hex = Text::new("Address (hex, 32 bytes):").prompt()?;
            let addr = hex32(&addr_hex)?;
            let mut book = load_contacts().unwrap_or_default();
            book.entries.push(Contact { name, pubkey: addr });
            save_contacts(&book)?;
            println!("Added.");
            wait_enter()?;
        } else {
            let idx = contact_index_from_label(&book, &choice);
            if let Some(i) = idx {
                let ok = Confirm::new("Delete this contact?").with_default(false).prompt()?;
                if ok {
                    let mut book = load_contacts().unwrap_or_default();
                    book.entries.remove(i);
                    save_contacts(&book)?;
                    println!("Deleted.");
                    wait_enter()?;
                }
            }
        }
    }
    Ok(())
}

fn tui_transfer(cfg: &Config) -> Result<()> {
    let ks = load_keystore().unwrap_or_default();
    if ks.keys.is_empty() {
        println!("No wallets yet. Create one first.");
        wait_enter()?;
        return Ok(());
    }
    let choices: Vec<String> = ks
        .keys
        .iter()
        .map(|k| format!("{}  ({}…)  [local_nonce={}]", k.name, &hex::encode(k.pubkey)[..8], k.local_nonce))
        .collect();
    let sel = Select::new("Choose sender", choices).prompt()?;
    let sidx = wallet_index_from_label(&ks, &sel).unwrap();
    let sender = &ks.keys[sidx];

    let use_contact = Confirm::new("Pick recipient from Contacts?").with_default(true).prompt()?;
    let to_pubkey = if use_contact {
        let book = load_contacts().unwrap_or_default();
        if book.entries.is_empty() {
            println!("No contacts yet; entering address manually.");
            hex32(&Text::new("Recipient address (hex, 32 bytes):").prompt()?)?
        } else {
            let items: Vec<String> = book
                .entries
                .iter()
                .map(|c| format!("{}  ({}…)", c.name, &hex::encode(c.pubkey)[..8]))
                .collect();
            let pick = Select::new("Choose contact", items).prompt()?;
            let idx = contact_index_from_label(&book, &pick).unwrap();
            book.entries[idx].pubkey
        }
    } else {
        hex32(&Text::new("Recipient address (hex, 32 bytes):").prompt()?)?
    };

    let amount: u128 = CustomType::new("Amount:").prompt()?;
    let fee: u128 = CustomType::new("Fee:").with_default(10u128).prompt()?;

    let remote_nonce = get_balance(cfg, &sender.pubkey).ok().map(|b| b.nonce).unwrap_or(sender.local_nonce);
    let mut nonce = remote_nonce;
    if !Confirm::new(&format!("Use network nonce {}?", nonce)).with_default(true).prompt()? {
        nonce = CustomType::new("Enter nonce:").prompt()?;
    }

    let msg = serde_json::to_vec(&(sender.pubkey, to_pubkey, amount, fee, nonce, CHAIN_ID))?;
    let sk = SecretKey { bytes: sender.secret };
    let signature = STARK.sign(&sk, &msg, nonce)?;
    let tx = TxFile {
        from: sender.pubkey,
        to: to_pubkey,
        amount,
        fee,
        signature,
    };

    match submit_tx(cfg, &tx) {
        Ok(path) => {
            println!("✅ Submitted. Queued at: {}", path);
            let mut ks2 = ks.clone();
            ks2.keys[sidx].local_nonce = nonce.saturating_add(1);
            save_keystore(&ks2)?;
        }
        Err(e) => {
            eprintln!("Submit failed: {e}");
        }
    }

    wait_enter()?;
    Ok(())
}

/* ---------- Status & Settings ---------- */

fn print_status(cfg: &Config) -> Result<()> {
    match get_status(cfg) {
        Ok(s) => {
            println!("Node RPC: {}", cfg.rpc_url);
            println!("Height: {}", s.height);
            println!("State root: {}", s.state_root);
            println!("Last block: {}", s.last_block_hash);
            println!("Chain ID: {}", s.chain_id);
        }
        Err(e) => {
            eprintln!("Status error: {e}");
        }
    }
    Ok(())
}

fn tui_settings(cfg: &mut Config) -> Result<()> {
    loop {
        let choice = Select::new(
            "Settings",
            vec![
                format!("Set RPC URL (current: {})", cfg.rpc_url),
                format!("Set node base dir (current: {})", cfg.base),
                format!(
                    "Set admin token (current: {}…)",
                    &cfg.admin_token.chars().take(6).collect::<String>()
                ),
                "Back".into(),
            ],
        )
        .prompt()?;
        match choice.as_str() {
            s if s.starts_with("Set RPC URL") => {
                let newu = Text::new("RPC URL:")
                    .with_default(&cfg.rpc_url)
                    .prompt()?;
                cfg.rpc_url = newu;
                save_config(cfg)?;
                println!("Saved.");
                wait_enter()?;
            }
            s if s.starts_with("Set node base dir") => {
                let newb = Text::new("Path to node base (dxid-data):")
                    .with_default(&cfg.base)
                    .prompt()?;
                cfg.base = newb;
                save_config(cfg)?;
                println!("Saved.");
                wait_enter()?;
            }
            s if s.starts_with("Set admin token") => {
                cfg.admin_token =
                    Text::new("Admin token:")
                        .with_default(&cfg.admin_token)
                        .prompt()?;
                save_config(cfg)?;
                println!("Saved.");
                wait_enter()?;
            }
            "Back" => break,
            _ => {}
        }
    }
    Ok(())
}

/* ---------- HTTP helpers ---------- */

fn get_status(cfg: &Config) -> Result<StatusResp> {
    let url = format!("{}/status", cfg.rpc_url);
    let cli = reqwest::blocking::Client::builder()
        .timeout(Duration::from_secs(5))
        .build()?;
    Ok(cli.get(url).send()?.error_for_status()?.json()?)
}
fn get_balance(cfg: &Config, addr: &[u8; 32]) -> Result<BalanceResp> {
    let url = format!("{}/balance/{}", cfg.rpc_url, hex::encode(addr));
    let cli = reqwest::blocking::Client::builder()
        .timeout(Duration::from_secs(5))
        .build()?;
    Ok(cli
        .get(url)
        .header("X-Api-Key", prompt_api_key()?)
        .send()?
        .error_for_status()?
        .json()?)
}
fn submit_tx(cfg: &Config, tx: &TxFile) -> Result<String> {
    #[derive(Serialize)]
    struct Body<'a> {
        from: String,
        to: String,
        amount: u128,
        fee: u128,
        signature: &'a StarkSignature,
    }
    #[derive(Deserialize)]
    struct Resp {
        queued: bool,
        file: String,
    }
    let body = Body {
        from: hex::encode(tx.from),
        to: hex::encode(tx.to),
        amount: tx.amount,
        fee: tx.fee,
        signature: &tx.signature,
    };
    let url = format!("{}/submitTx", cfg.rpc_url);
    let cli = reqwest::blocking::Client::builder()
        .timeout(Duration::from_secs(10))
        .build()?;
    let resp: Resp = cli
        .post(url)
        .header("X-Api-Key", prompt_api_key()?)
        .json(&body)
        .send()?
        .error_for_status()?
        .json()?;
    if !resp.queued {
        anyhow::bail!("node rejected tx");
    }
    Ok(resp.file)
}

/* ---------- Admin HTTP helpers ---------- */

fn admin_create_key(cfg: &Config, name: &str) -> Result<ApiKey> {
    #[derive(Serialize)]
    struct Body<'a> {
        name: &'a str
    }
    let url = format!("{}/admin/apikeys", cfg.rpc_url);
    let cli = reqwest::blocking::Client::new();
    let k: ApiKey = cli
        .post(url)
        .header("X-Admin-Token", &cfg.admin_token)
        .json(&Body { name })
        .send()?
        .error_for_status()?
        .json()?;
    Ok(k)
}
fn admin_list_keys(cfg: &Config) -> Result<AdminListKeysResp> {
    let url = format!("{}/admin/apikeys", cfg.rpc_url);
    let cli = reqwest::blocking::Client::new();
    Ok(cli
        .get(url)
        .header("X-Admin-Token", &cfg.admin_token)
        .send()?
        .error_for_status()?
        .json()?)
}
fn admin_add_webhook(
    cfg: &Config,
    api_key_id: &str,
    url_str: &str,
    events: Vec<String>,
) -> Result<String> {
    #[derive(Serialize)]
    struct Body<'a> {
        api_key_id: &'a str,
        url: &'a str,
        events: Vec<String>,
    }
    let url = format!("{}/admin/webhooks", cfg.rpc_url);
    let cli = reqwest::blocking::Client::new();
    let resp: AdminAddWebhookResp = cli
        .post(url)
        .header("X-Admin-Token", &cfg.admin_token)
        .json(&Body {
            api_key_id,
            url: url_str,
            events,
        })
        .send()?
        .error_for_status()?
        .json()?;
    Ok(resp.id)
}
fn admin_list_webhooks(cfg: &Config) -> Result<AdminListWebhooksResp> {
    let url = format!("{}/admin/webhooks", cfg.rpc_url);
    let cli = reqwest::blocking::Client::new();
    Ok(cli
        .get(url)
        .header("X-Admin-Token", &cfg.admin_token)
        .send()?
        .error_for_status()?
        .json()?)
}
fn admin_delete_webhook(cfg: &Config, id: &str) -> Result<()> {
    let url = format!("{}/admin/webhooks/{}", cfg.rpc_url, id);
    let cli = reqwest::blocking::Client::new();
    cli.delete(url)
        .header("X-Admin-Token", &cfg.admin_token)
        .send()?
        .error_for_status()?;
    Ok(())
}

/* ---------- Local storage & utils ---------- */

fn dxid_dir() -> PathBuf {
    let home = std::env::var_os("HOME")
        .or_else(|| std::env::var_os("USERPROFILE"))
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from("."));
    home.join(".dxid")
}
fn config_path() -> PathBuf {
    dxid_dir().join("config.json")
}
fn keystore_path() -> PathBuf {
    dxid_dir().join("keystore.json")
}
fn contacts_path() -> PathBuf {
    dxid_dir().join("contacts.json")
}

fn load_config() -> Option<Config> {
    let p = config_path();
    let s = fs::read_to_string(p).ok()?;
    serde_json::from_str(&s).ok()
}
fn save_config(cfg: &Config) -> Result<()> {
    fs::write(config_path(), serde_json::to_string_pretty(cfg)?)?;
    Ok(())
}
fn load_keystore() -> Option<Keystore> {
    let p = keystore_path();
    let s = fs::read_to_string(p).ok()?;
    serde_json::from_str(&s).ok()
}
fn save_keystore(ks: &Keystore) -> Result<()> {
    fs::create_dir_all(dxid_dir())?;
    fs::write(keystore_path(), serde_json::to_string_pretty(ks)?)?;
    Ok(())
}
fn load_contacts() -> Option<Contacts> {
    let p = contacts_path();
    let s = fs::read_to_string(p).ok()?;
    serde_json::from_str(&s).ok()
}
fn save_contacts(c: &Contacts) -> Result<()> {
    fs::create_dir_all(dxid_dir())?;
    fs::write(contacts_path(), serde_json::to_string_pretty(c)?)?;
    Ok(())
}
fn wallet_index_from_label(ks: &Keystore, label: &str) -> Option<usize> {
    ks.keys
        .iter()
        .position(|k| label.contains(&k.name) && label.contains(&hex::encode(k.pubkey)[..8]))
}
fn contact_index_from_label(book: &Contacts, label: &str) -> Option<usize> {
    book.entries
        .iter()
        .position(|c| label.contains(&c.name) && label.contains(&hex::encode(c.pubkey)[..8]))
}
fn hex32(s: &str) -> Result<[u8; 32]> {
    let v = hex::decode(s)?;
    if v.len() != 32 {
        anyhow::bail!("expected 32-byte hex");
    }
    let mut out = [0u8; 32];
    out.copy_from_slice(&v);
    Ok(out)
}
fn wait_enter() -> io::Result<()> {
    use std::io::Write;
    print!("Press ENTER to continue…");
    io::stdout().flush()?;
    let mut buf = String::new();
    io::stdin().read_line(&mut buf)?;
    Ok(())
}
fn prompt_api_key() -> Result<String> {
    let p = dxid_dir().join("apikey.txt");
    if let Ok(s) = fs::read_to_string(&p) {
        let v = s.trim().to_string();
        if !v.is_empty() {
            return Ok(v);
        }
    }
    let key = Text::new("Enter your API key (X-Api-Key):").prompt()?;
    fs::write(p, &key)?;
    Ok(key)
}
fn cmd_keygen() -> Result<()> {
    let (sk, pk) = STARK.generate_keys()?;
    println!("pubkey (address): {}", hex::encode(pk));
    println!("secret (dev only): {}", hex::encode(sk.bytes));
    Ok(())
}
fn cmd_transfer_noninteractive(
    cfg: &Config,
    secret_hex: String,
    from: String,
    to: String,
    amount: u128,
    fee: u128,
    nonce: u64,
) -> Result<()> {
    let from_pk = hex32(&from)?;
    let to_pk = hex32(&to)?;
    let mut skb = [0u8; 32];
    hex::decode_to_slice(&secret_hex, &mut skb)?;
    let sk = SecretKey { bytes: skb };

    let msg = serde_json::to_vec(&(from_pk, to_pk, amount, fee, nonce, CHAIN_ID))?;
    let signature = STARK.sign(&sk, &msg, nonce)?;

    let tx = TxFile {
        from: from_pk,
        to: to_pk,
        amount,
        fee,
        signature,
    };

    let path = submit_tx(cfg, &tx)?;
    println!("queued tx at {}", path);
    Ok(())
}
