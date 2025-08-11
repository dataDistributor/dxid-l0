use anyhow::{anyhow, Result};
use parking_lot::Mutex;
use serde::{Deserialize, Serialize};
use std::{collections::HashMap, fs, path::PathBuf, sync::Arc};
use uuid::Uuid;

use dxid_crypto::{PublicKeyHash, StarkSignature, ENGINE as STARK};
use dxid_crypto::StarkSignEngine; // trait in scope

mod smt;
pub const CHAIN_ID: u32 = 7777;

#[derive(Clone, Serialize, Deserialize, Debug)]
pub struct Account {
    pub pubkey_hash: PublicKeyHash,
    pub nonce: u64,
    pub balance: u128,
}

#[derive(Clone, Serialize, Deserialize, Debug)]
pub struct Tx {
    pub from: PublicKeyHash,
    pub to: PublicKeyHash,
    pub amount: u128,
    pub fee: u128,
    pub signature: StarkSignature,
}

#[derive(Clone, Serialize, Deserialize, Debug, Default)]
pub struct State {
    pub accounts: HashMap<String, Account>, // key = hex(pubkey_hash)
    pub height: u64,
    pub last_block_hash: [u8; 32],
    pub state_root: [u8; 32],
}

#[derive(Clone, Serialize, Deserialize, Debug)]
pub struct Block {
    pub header: BlockHeader,
    pub txs: Vec<Tx>,
}

#[derive(Clone, Serialize, Deserialize, Debug)]
pub struct BlockHeader {
    pub height: u64,
    pub prev_hash: [u8; 32],
    pub tx_root: [u8; 32],
    /// Previous and new state roots (post-apply, SMT)
    pub prev_state_root: [u8; 32],
    pub state_root: [u8; 32],
    /// Transition commitment placeholder (swap for STARK of state transition)
    pub transition_commitment: [u8; 32],
    /// Block timestamp (unix seconds)
    pub timestamp: u64,
    /// Chain id to domain-separate signables
    pub chain_id: u32,
    pub id: Uuid,
}

fn h(bytes: &[u8]) -> [u8; 32] {
    *blake3::hash(bytes).as_bytes()
}
fn hexify(bytes: &[u8]) -> String {
    hex::encode(bytes)
}

impl State {
    pub fn new_with_genesis(funded: Vec<(PublicKeyHash, u128)>) -> Self {
        let mut accounts = HashMap::new();
        for (pk, bal) in funded {
            accounts.insert(
                hexify(&pk),
                Account {
                    pubkey_hash: pk,
                    nonce: 0,
                    balance: bal,
                },
            );
        }
        let state_root = smt::state_root_from_accounts(&accounts);
        State {
            accounts,
            height: 0,
            last_block_hash: [0u8; 32],
            state_root,
        }
    }

    pub fn apply_block(&mut self, b: &Block) -> Result<()> {
        // Verify linkage
        if b.header.prev_hash != self.last_block_hash {
            return Err(anyhow!("prev_hash mismatch at height {}", b.header.height));
        }
        if b.header.prev_state_root != self.state_root {
            return Err(anyhow!(
                "prev_state_root mismatch at height {}",
                b.header.height
            ));
        }
        // Verify tx_root matches
        let want_root = merkleize_txs(&b.txs);
        if want_root != b.header.tx_root {
            return Err(anyhow!("tx_root mismatch"));
        }
        // Apply txs
        for tx in &b.txs {
            apply_tx(self, tx)?;
        }
        // Compute new SMT root and check header
        let new_root = smt::state_root_from_accounts(&self.accounts);
        if new_root != b.header.state_root {
            return Err(anyhow!("state_root mismatch after apply"));
        }
        // Update height & last hash
        self.height = b.header.height;
        self.last_block_hash = block_hash(&b.header);
        self.state_root = new_root;
        Ok(())
    }
}

fn apply_tx(st: &mut State, tx: &Tx) -> Result<()> {
    // Signable message (deterministic JSON tuple) with nonce & chain bound
    let msg = serde_json::to_vec(&(
        tx.from,
        tx.to,
        tx.amount,
        tx.fee,
        tx.signature.nonce,
        CHAIN_ID,
    ))?;
    // Signature must match "from"
    if tx.signature.pubkey_hash != tx.from {
        return Err(anyhow!("signature pubkey mismatch"));
    }
    // Verify STARK-backed signature
    STARK.verify(&tx.signature, &msg)?;

    // Nonce & balances
    let from_key = hexify(&tx.from);
    let to_key = hexify(&tx.to);

    let from = st
        .accounts
        .get_mut(&from_key)
        .ok_or_else(|| anyhow!("sender not found"))?;

    if from.nonce != tx.signature.nonce {
        return Err(anyhow!(
            "nonce mismatch: expected {}, got {}",
            from.nonce,
            tx.signature.nonce
        ));
    }

    let total = tx
        .amount
        .checked_add(tx.fee)
        .ok_or_else(|| anyhow!("overflow"))?;
    if from.balance < total {
        return Err(anyhow!("insufficient balance"));
    }

    from.balance -= total;
    from.nonce += 1;

    let to = st.accounts.entry(to_key).or_insert(Account {
        pubkey_hash: tx.to,
        nonce: 0,
        balance: 0,
    });
    to.balance = to
        .balance
        .checked_add(tx.amount)
        .ok_or_else(|| anyhow!("overflow"))?;

    Ok(())
}

fn block_hash(hdr: &BlockHeader) -> [u8; 32] {
    // Hash all header fields deterministically
    let mut v = Vec::new();
    v.extend_from_slice(&hdr.prev_hash);
    v.extend_from_slice(&hdr.tx_root);
    v.extend_from_slice(&hdr.prev_state_root);
    v.extend_from_slice(&hdr.state_root);
    v.extend_from_slice(&hdr.transition_commitment);
    v.extend_from_slice(&hdr.height.to_le_bytes());
    v.extend_from_slice(&hdr.timestamp.to_le_bytes());
    v.extend_from_slice(&hdr.chain_id.to_le_bytes());
    v.extend_from_slice(hdr.id.as_bytes());
    h(&v)
}

fn merkleize_txs(txs: &[Tx]) -> [u8; 32] {
    // Flat merkle-ish: hash(concat(hash(tx_i)))
    let mut acc = Vec::with_capacity(32 * txs.len());
    for t in txs {
        acc.extend_from_slice(&h(&serde_json::to_vec(t).unwrap()));
    }
    h(&acc)
}

/// Simple in-process “chain” that produces a block every N ms from tx JSON files in `mempool_dir`.
pub struct Chain {
    pub state: Arc<Mutex<State>>,
    pub mempool_dir: PathBuf,
    pub blocks_dir: PathBuf,
    pub period_ms: u64,
}

impl Chain {
    pub fn new(state: State, base: PathBuf, period_ms: u64) -> Result<Self> {
        let mempool_dir = base.join("mempool");
        let blocks_dir = base.join("blocks");
        fs::create_dir_all(&mempool_dir)?;
        fs::create_dir_all(&blocks_dir)?;
        Ok(Self {
            state: Arc::new(Mutex::new(state)),
            mempool_dir,
            blocks_dir,
            period_ms,
        })
    }

    /// Collect tx files, make a block, apply it, write it to disk.
    pub fn make_block_once(&self) -> Result<Option<Block>> {
        let entries = fs::read_dir(&self.mempool_dir)?;
        let mut txs = Vec::new();
        let mut consumed = Vec::new();

        for e in entries {
            let e = e?;
            if !e.file_type()?.is_file() {
                continue;
            }
            let p = e.path();
            if p.extension().and_then(|s| s.to_str()) != Some("json") {
                continue;
            }
            let txt = fs::read_to_string(&p)?;
            let tx: Tx = serde_json::from_str(&txt)?;
            // Pre-validate signature before including
            let msg = serde_json::to_vec(&(tx.from, tx.to, tx.amount, tx.fee, tx.signature.nonce, CHAIN_ID))?;
            STARK.verify(&tx.signature, &msg)?;
            txs.push(tx);
            consumed.push(p);
        }

        if txs.is_empty() {
            return Ok(None);
        }

        // Build header (need next state root). Clone state & simulate apply.
        let mut st = self.state.lock();
        let height = st.height + 1;
        let tx_root = merkleize_txs(&txs);
        let prev_state_root = st.state_root;

        // Simulate to compute new state root via SMT
        let mut preview = st.clone();
        for tx in &txs {
            apply_tx(&mut preview, tx)?;
        }
        let next_state_root = smt::state_root_from_accounts(&preview.accounts);

        // Transition commitment placeholder
        let mut pre = Vec::new();
        pre.extend_from_slice(&st.last_block_hash);
        pre.extend_from_slice(&tx_root);
        pre.extend_from_slice(&height.to_le_bytes());
        let transition_commitment = h(&pre);

        let header = BlockHeader {
            height,
            prev_hash: st.last_block_hash,
            tx_root,
            prev_state_root,
            state_root: next_state_root,
            transition_commitment,
            timestamp: now_ts(),
            chain_id: CHAIN_ID,
            id: Uuid::new_v4(),
        };
        let block = Block {
            header: header.clone(),
            txs,
        };

        // Apply and persist
        st.apply_block(&block)?;
        let fname = format!("{:016x}.json", height);
        let path = self.blocks_dir.join(fname);
        fs::write(&path, serde_json::to_string_pretty(&block)?)?;

        // Remove consumed txs
        for p in consumed {
            let _ = fs::remove_file(p);
        }

        Ok(Some(block))
    }
}

fn now_ts() -> u64 {
    use std::time::{SystemTime, UNIX_EPOCH};
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs()
}
