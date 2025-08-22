use anyhow::Result;
use parking_lot::Mutex;
use serde::{Deserialize, Serialize};
use std::{collections::HashMap, fs, path::PathBuf, sync::Arc};

use dxid_crypto::{StarkSignEngine, ENGINE as STARK};
use dxid_smt::{H256, SparseMerkleTree, SmtProof};

// Import the storage module
pub mod storage;
use storage::{Storage, StorageConfig};

pub const CHAIN_ID: u32 = 1337;

// Layer0 Token Constants - STORE OF VALUE
pub const LAYER0_TOTAL_SUPPLY: u128 = 10_000_000_000_000_000; // 10 billion with 8 decimals
pub const LAYER0_BLOCK_REWARD: u128 = 100_000_000_000; // 100 L0 tokens per block
pub const LAYER0_HALVING_BLOCKS: u64 = 100_000; // Halve every 100,000 blocks
pub const LAYER0_DECIMALS: u8 = 8; // 8 decimals like Bitcoin
pub const LAYER0_ZERO_FEES: bool = true; // NO TRANSACTION FEES - PURE STORE OF VALUE
pub const LAYER0_APPRECIATION_RATE: u64 = 1000; // 0.1% appreciation per block


// LongYield L1 Token Constants
pub const LONGYIELD_CHAIN_ID: u32 = 1338;
pub const LONGYIELD_TOTAL_SUPPLY: u128 = 1_000_000_000_000_000_000; // 1 billion with 18 decimals
pub const LONGYIELD_DECIMALS: u8 = 18;

#[derive(Clone, Serialize, Deserialize, Debug)]
pub struct Account {
    pub balance: u128,
    pub nonce: u64,
    pub layer0_balance: u128, // Layer0 token balance
    pub longyield_balance: u128, // LongYield L1 token balance
}

#[derive(Clone, Serialize, Deserialize, Debug)]
pub struct Tx {
    pub from: H256,
    pub to: H256,
    pub amount: u128,
    pub fee: u128, // Only used for non-Layer0 tokens
    pub signature: dxid_crypto::StarkSignature,
    pub token_type: TokenType, // Which token to transfer
    pub cross_chain: bool, // Is this a cross-chain transaction?
    pub target_chain_id: Option<u32>, // Target chain for cross-chain txs
}

#[derive(Clone, Serialize, Deserialize, Debug, PartialEq)]
pub enum TokenType {
    Layer0, // Store of value token (Bitcoin-like)
    LongYield, // L1 token
    Native, // Legacy native token
}

#[derive(Clone, Serialize, Deserialize, Debug)]
pub struct BlockHeader {
    pub height: u64,
    pub timestamp: u64,
    pub tx_root: H256,
    pub state_root: H256,
    pub layer0_reward: u128, // Layer0 block reward
    pub longyield_reward: u128, // LongYield block reward
}

#[derive(Clone, Serialize, Deserialize, Debug)]
pub struct Block {
    pub header: BlockHeader,
    pub txs: Vec<Tx>,
}

#[derive(Clone, Serialize, Deserialize, Debug)]
pub struct State {
    pub accounts: HashMap<String, Account>, // key: hex(addr)
    pub height: u64,
    pub last_block_hash: H256,
    pub state_root: H256,
    pub layer0_circulating: u128, // Total Layer0 tokens in circulation
    pub longyield_circulating: u128, // Total LongYield tokens in circulation
    #[serde(skip)]
    smt: SparseMerkleTree,
}

impl State {
    pub fn new_with_genesis(genesis_alloc: Vec<(H256, u128)>) -> Arc<Mutex<Self>> {
        let mut smt = SparseMerkleTree::new();
        let mut accounts: HashMap<String, Account> = HashMap::new();
        
        // Initialize Layer0 faucet with 1 trillion tokens for testing
        let layer0_faucet_balance = 1_000_000_000_000_000_000u128; // 1 trillion with 8 decimals
        
        for (addr, bal) in genesis_alloc {
            accounts.insert(hex::encode(addr), Account { 
                balance: bal, 
                nonce: 0,
                layer0_balance: if bal > 0 { layer0_faucet_balance } else { 0 },
                longyield_balance: 0,
            });
            smt.update(addr, Some(u128_to_h256(bal)));
        }
        
        let state_root = smt.root();
        Arc::new(Mutex::new(Self {
            accounts,
            height: 0,
            last_block_hash: [0u8; 32],
            state_root,
            layer0_circulating: layer0_faucet_balance,
            longyield_circulating: 0,
            smt,
        }))
    }

    /// Calculate Layer0 block reward
    pub fn calculate_layer0_reward(&self) -> u128 {
        let halvings = self.height / LAYER0_HALVING_BLOCKS;
        let base_reward = LAYER0_BLOCK_REWARD >> halvings; // Bit shift for halving
        
        // Early block bonuses
        let early_miner_bonus = if self.height < 1000 { 
            LAYER0_BLOCK_REWARD / 2 // 50% bonus for first 1000 blocks
        } else if self.height < 10000 {
            LAYER0_BLOCK_REWARD / 4 // 25% bonus for first 10,000 blocks
        } else {
            0
        };
        
        // Difficulty bonuses
        let difficulty_bonus = if self.height > 1000 {
            ((self.height / 1000) as u128) * 10_000_000_000u128 // Bonus increases with height
        } else {
            0u128
        };
        
        base_reward + early_miner_bonus + difficulty_bonus
    }

    /// Calculate LongYield block reward (fixed for now)
    pub fn calculate_longyield_reward(&self) -> u128 {
        1_000_000_000_000_000_000u128 // 1 L1 token per block
    }

    /// Produce a real SMT inclusion proof for an address.
    pub fn prove_account(&self, addr_hex: &str) -> (Option<Account>, SmtProof) {
        let addr = dehex32(addr_hex);
        let (leaf, proof) = if let Some(a) = addr {
            let (_val, p) = self.smt.prove(&a);
            let acct = self.accounts.get(&addr_hex.to_lowercase()).cloned();
            (acct, p)
        } else {
            (None, SmtProof::empty())
        };
        (leaf, proof)
    }

    /// Update in-memory SMT after a balance/nonce change.
    pub fn set_account(&mut self, addr: H256, acct: &Account) {
        self.accounts.insert(hex::encode(addr), acct.clone());
        
        // Create a comprehensive account hash that includes all account data
        let mut account_data = Vec::new();
        account_data.extend_from_slice(&acct.balance.to_le_bytes());
        account_data.extend_from_slice(&acct.nonce.to_le_bytes());
        account_data.extend_from_slice(&acct.layer0_balance.to_le_bytes());
        account_data.extend_from_slice(&acct.longyield_balance.to_le_bytes());
        
        let account_hash = blake3::hash(&account_data);
        self.smt.update(addr, Some(*account_hash.as_bytes()));
        self.state_root = self.smt.root();
    }

    /// Reconstruct SMT from accounts (used when loading from storage)
    pub fn reconstruct_smt(&mut self) {
        self.smt = SparseMerkleTree::new();
        for (addr_hex, account) in &self.accounts {
            if let Some(addr) = dehex32(addr_hex) {
                // Create comprehensive account hash that includes all account data
                let mut account_data = Vec::new();
                account_data.extend_from_slice(&account.balance.to_le_bytes());
                account_data.extend_from_slice(&account.nonce.to_le_bytes());
                account_data.extend_from_slice(&account.layer0_balance.to_le_bytes());
                account_data.extend_from_slice(&account.longyield_balance.to_le_bytes());
                
                let account_hash = blake3::hash(&account_data);
                self.smt.update(addr, Some(*account_hash.as_bytes()));
            }
        }
        self.state_root = self.smt.root();
    }
}

#[derive(Clone)]
pub struct Chain {
    pub state: Arc<Mutex<State>>,
    pub mempool_dir: PathBuf,
    pub blocks_dir: PathBuf,
    block_time_ms: u64,
    storage: Arc<Storage>,
}

impl Chain {
    pub fn new(state: Arc<Mutex<State>>, base: PathBuf, block_time_ms: u64) -> Result<Self> {
        let mempool = base.join("mempool");
        let blocks = base.join("blocks");
        fs::create_dir_all(&mempool)?;
        fs::create_dir_all(&blocks)?;
        
        // Initialize storage
        let storage_config = StorageConfig {
            base_dir: base.clone(),
            ..Default::default()
        };
        let storage = Arc::new(Storage::new(storage_config)?);
        
        // Try to load existing state from storage
        if let Some(saved_state) = storage.load_state()? {
            let mut state_guard = state.lock();
            *state_guard = saved_state;
            state_guard.reconstruct_smt();
            println!("Loaded existing state from height {}", state_guard.height);
        } else {
            println!("Starting with fresh genesis state");
        }
        
        Ok(Self { state, mempool_dir: mempool, blocks_dir: blocks, block_time_ms, storage })
    }

    pub fn make_block_once(self: &Arc<Self>) -> Result<Option<Block>> {
        // Pre-allocate vectors to reduce allocations
        let mut txs = Vec::with_capacity(100); // Reasonable capacity for mempool
        
        // Read mempool files with better error handling
        if let Ok(entries) = fs::read_dir(&self.mempool_dir) {
            for entry in entries {
                if let Ok(entry) = entry {
                    let p = entry.path();
                    if p.extension().and_then(|s| s.to_str()) == Some("json") {
                        if let Ok(txt) = fs::read_to_string(&p) {
                            if let Ok(tx) = serde_json::from_str::<Tx>(&txt) {
                                txs.push((p, tx));
                            }
                        }
                    }
                }
            }
        }

        // Apply transactions with better error handling
        let mut st = self.state.lock();
        let mut applied = Vec::with_capacity(txs.len());
        let mut failed_txs = Vec::new();
        
        for (path, tx) in txs {
            match Self::apply_tx(&mut st, &tx) {
                Ok(_) => {
                    // Remove file only after successful application
                    let _ = fs::remove_file(&path);
                    applied.push(tx);
                }
                Err(_) => {
                    failed_txs.push(path);
                }
            }
        }

        // Always produce a block (even empty) for Layer0 store of value
        st.height += 1;
        
        // For empty blocks, ensure state root changes by including block metadata
        if applied.is_empty() {
            // Create a unique "empty block" key that changes with each block
            let mut empty_block_key = [0u8; 32];
            empty_block_key[0..8].copy_from_slice(&st.height.to_le_bytes());
            empty_block_key[8..16].copy_from_slice(&now_ts().to_le_bytes());
            
            let mut empty_block_data = Vec::new();
            empty_block_data.extend_from_slice(&st.height.to_le_bytes());
            empty_block_data.extend_from_slice(&now_ts().to_le_bytes());
            empty_block_data.extend_from_slice(&st.last_block_hash);
            
            let empty_block_hash = blake3::hash(&empty_block_data);
            st.smt.update(empty_block_key, Some(*empty_block_hash.as_bytes()));
        }
        
        // Recalculate state root AFTER all updates (transactions + empty block data)
        st.state_root = st.smt.root();
        
        let header = BlockHeader {
            height: st.height,
            timestamp: now_ts(),
            tx_root: h_txs(&applied),
            state_root: st.state_root,
            layer0_reward: st.calculate_layer0_reward(),
            longyield_reward: st.calculate_longyield_reward(),
        };
        let block = Block { header: header.clone(), txs: applied };
        
        // Update the last_block_hash in state
        st.last_block_hash = h_block_header(&header);
        
        // Persist block with enhanced storage
        if let Err(e) = self.storage.save_block(&block) {
            eprintln!("Failed to persist block {}: {}", header.height, e);
        }
        
        // Save state to persistent storage
        if let Err(e) = self.storage.save_state(&st) {
            eprintln!("Failed to save state: {}", e);
        }
        
        // Create checkpoint periodically
        if let Err(e) = self.storage.create_checkpoint(&st, &block) {
            eprintln!("Failed to create checkpoint: {}", e);
        }
        
        // Index transactions for efficient querying
        for (tx_index, tx) in block.txs.iter().enumerate() {
            let tx_hash = h_txs(&[tx.clone()]);
            if let Err(e) = self.storage.index_transaction(tx_hash, block.header.height, tx_index) {
                eprintln!("Failed to index transaction: {}", e);
            }
        }
        
        // Create backup periodically
        if let Err(e) = self.storage.create_backup() {
            eprintln!("Failed to create backup: {}", e);
        }
        
        Ok(Some(block))
    }

    /// Get storage statistics
    pub fn get_storage_stats(&self) -> Result<storage::StorageStats> {
        self.storage.get_stats()
    }

    fn apply_tx(st: &mut State, tx: &Tx) -> Result<()> {
        // basic sig/domain separation
        let msg = serde_json::to_vec(&(tx.from, tx.to, tx.amount, tx.fee, tx.signature.nonce, CHAIN_ID))?;
        STARK.verify(&tx.signature, &msg)?;

        let from_hex = hex::encode(tx.from);
        let to_hex = hex::encode(tx.to);

        // snapshot current accounts (avoid holding entry borrows while updating SMT)
        let mut from_acct = st.accounts.get(&from_hex).cloned().unwrap_or(Account { balance: 0, nonce: 0, layer0_balance: 0, longyield_balance: 0 });
        let mut to_acct   = st.accounts.get(&to_hex).cloned().unwrap_or(Account { balance: 0, nonce: 0, layer0_balance: 0, longyield_balance: 0 });

        // economic rules
        if from_acct.nonce != tx.signature.nonce { anyhow::bail!("bad nonce"); }
        let spend = tx.amount.saturating_add(tx.fee);

        // Handle different token types
        match tx.token_type {
            TokenType::Layer0 => {
                // LAYER0: ULTIMATE STORE OF VALUE - ZERO FEES, PURE APPRECIATION
                if from_acct.layer0_balance < tx.amount { anyhow::bail!("insufficient layer0 balance"); }
                
                // ZERO FEES - Pure transfer, no cost
                from_acct.layer0_balance -= tx.amount;
                to_acct.layer0_balance = to_acct.layer0_balance.saturating_add(tx.amount);
                
                // AUTOMATIC APPRECIATION: Every Layer0 holder gets appreciation
                let appreciation = tx.amount * LAYER0_APPRECIATION_RATE as u128 / 1_000_000; // 0.1% appreciation
                to_acct.layer0_balance = to_acct.layer0_balance.saturating_add(appreciation);
                

                
                // NO FEES BURNED - Layer0 is pure store of value
            },
            TokenType::LongYield => {
                // LongYield L1 token transfer (still has fees for utility)
                if from_acct.longyield_balance < spend { anyhow::bail!("insufficient longyield balance"); }
                
                from_acct.longyield_balance -= spend;
                to_acct.longyield_balance = to_acct.longyield_balance.saturating_add(tx.amount);
                
                st.longyield_circulating = st.longyield_circulating.saturating_sub(tx.fee);
            },
            TokenType::Native => {
                // Legacy native token transfer
                if from_acct.balance < spend { anyhow::bail!("insufficient native balance"); }
                
                from_acct.balance -= spend;
                to_acct.balance = to_acct.balance.saturating_add(tx.amount);
            },
        }

        // Update nonce
        from_acct.nonce += 1;

        // write back + update SMT without overlapping borrows
        st.set_account(tx.from, &from_acct);
        st.set_account(tx.to, &to_acct);

        Ok(())
    }
}

/* ---- helpers ---- */

fn now_ts() -> u64 {
    use std::time::{SystemTime, UNIX_EPOCH};
    SystemTime::now().duration_since(UNIX_EPOCH).unwrap_or_default().as_secs()
}

fn h_txs(txs: &[Tx]) -> H256 {
    // very simple tx root: hash concatenation
    use blake3::Hasher;
    let mut hasher = Hasher::new();
    for tx in txs {
        let s = serde_json::to_vec(tx).unwrap();
        hasher.update(&s);
    }
    *hasher.finalize().as_bytes()
}

fn h_block_header(header: &BlockHeader) -> H256 {
    // Hash the block header
    use blake3::Hasher;
    let mut hasher = Hasher::new();
    let s = serde_json::to_vec(header).unwrap();
    hasher.update(&s);
    *hasher.finalize().as_bytes()
}

fn dehex32(s: &str) -> Option<H256> {
    let v = hex::decode(s).ok()?;
    if v.len() != 32 { return None; }
    let mut out = [0u8; 32];
    out.copy_from_slice(&v);
    Some(out)
}

fn u128_to_h256(x: u128) -> H256 {
    let mut out = [0u8; 32];
    out[16..].copy_from_slice(&x.to_be_bytes());
    out
}
