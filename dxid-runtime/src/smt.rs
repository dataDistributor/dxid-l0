//! Simple 256-bit Sparse Merkle Tree (SMT) for account state.
//! - Key = 32-byte pubkey hash (we treat it as a 256-bit little-endian index).
//! - Leaf hash = H(0x00 || H(balance_le || nonce_le))  (domain-separated)
//! - Inner hash = H(0x01 || left || right)
//! - Default/empty subtree hashes are precomputed ZERO[i] where i is the tree level.
//!
//! This is a dev-friendly, deterministic SMT to get a real, verifiable `state_root`.
//! It rebuilds from the full account map each block (fine for small N).

use crate::Account;
use blake3;
use std::collections::HashMap;

fn h(bytes: &[u8]) -> [u8; 32] {
    *blake3::hash(bytes).as_bytes()
}

fn hash_leaf(value_hash: &[u8; 32]) -> [u8; 32] {
    let mut v = Vec::with_capacity(1 + 32);
    v.push(0x00);
    v.extend_from_slice(value_hash);
    h(&v)
}
fn hash_node(left: &[u8; 32], right: &[u8; 32]) -> [u8; 32] {
    let mut v = Vec::with_capacity(1 + 32 + 32);
    v.push(0x01);
    v.extend_from_slice(left);
    v.extend_from_slice(right);
    h(&v)
}

/// Precompute ZERO hashes for all levels (0..=256).
fn zero_hashes() -> Vec<[u8; 32]> {
    let mut z = Vec::with_capacity(257);
    // ZERO leaf = hash of zero value
    let zero_val = [0u8; 32];
    let mut cur = hash_leaf(&zero_val);
    z.push(cur);
    for _ in 0..256 {
        cur = hash_node(&cur, &cur);
        z.push(cur);
    }
    z
}

/// Right-shift a 256-bit little-endian integer by 1 bit.
fn shr1_le(x: &[u8; 32]) -> [u8; 32] {
    let mut out = [0u8; 32];
    for i in 0..32 {
        let mut v = x[i] >> 1;
        if i + 1 < 32 {
            v |= (x[i + 1] & 0x01) << 7;
        }
        out[i] = v;
    }
    out
}

/// Compute the SMT root for the current accounts map.
pub fn state_root_from_accounts(accounts: &HashMap<String, Account>) -> [u8; 32] {
    let zeros = zero_hashes();

    // Build leaves at level 0: key = 32-byte LE index, value = leaf hash
    // We use pubkey_hash bytes as-is (consistent, deterministic).
    let mut level_map: HashMap<[u8; 32], [u8; 32]> = HashMap::new();
    for acct in accounts.values() {
        // Commit to (balance, nonce). (Key is already the address path.)
        let mut v = Vec::with_capacity(16 + 8);
        v.extend_from_slice(&acct.balance.to_le_bytes());
        v.extend_from_slice(&acct.nonce.to_le_bytes());
        let val_h = h(&v);
        let leaf = hash_leaf(&val_h);
        level_map.insert(acct.pubkey_hash, leaf);
    }

    // Empty tree?
    if level_map.is_empty() {
        return zeros[256];
    }

    // Fold up through 256 levels.
    for level in 0..256 {
        let mut parent_map: HashMap<[u8; 32], ([u8; 32], [u8; 32])> = HashMap::new();

        for (key, hash) in level_map.into_iter() {
            let bit = key[0] & 1;
            let pkey = shr1_le(&key);

            let entry = parent_map
                .entry(pkey)
                .or_insert((zeros[level], zeros[level]));
            if bit == 0 {
                entry.0 = hash; // left
            } else {
                entry.1 = hash; // right
            }
        }

        // Compute parent hashes for next level
        let mut next_level: HashMap<[u8; 32], [u8; 32]> = HashMap::with_capacity(parent_map.len());
        for (pkey, (left, right)) in parent_map.into_iter() {
            next_level.insert(pkey, hash_node(&left, &right));
        }
        level_map = next_level;
    }

    // After 256 folds, only one entry should remain (the root at key all-zero).
    // If for any reason it's empty (shouldn't happen), return ZERO[256].
    level_map
        .into_values()
        .next()
        .unwrap_or_else(|| zeros[256])
}
