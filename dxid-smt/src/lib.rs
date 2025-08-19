use anyhow::Result;
use blake3::Hasher;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// 32-byte array helper
pub type H256 = [u8; 32];

#[inline]
fn h(bytes: &[u8]) -> H256 {
    let mut hasher = Hasher::new();
    hasher.update(bytes);
    let out = hasher.finalize();
    *out.as_bytes()
}

#[inline]
fn h2(a: &H256, b: &H256) -> H256 {
    let mut bytes = [0u8; 64];
    bytes[..32].copy_from_slice(a);
    bytes[32..].copy_from_slice(b);
    h(&bytes)
}

/// Precomputed zero hashes for empty subtrees at each height (0..=256)
fn zero_hashes() -> &'static [H256; 257] {
    static mut Z: Option<[H256; 257]> = None;
    unsafe {
        if Z.is_none() {
            let mut arr = [[0u8; 32]; 257];
            arr[0] = h(&[0u8; 1]);
            for i in 1..=256 {
                arr[i] = h2(&arr[i - 1], &arr[i - 1]);
            }
            Z = Some(arr);
        }
        Z.as_ref().unwrap()
    }
}

/// Optimized hash function for better performance
#[inline]
fn fast_hash(data: &[u8]) -> H256 {
    let mut hasher = Hasher::new();
    hasher.update(data);
    *hasher.finalize().as_bytes()
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct SmtProof {
    /// Sibling hashes from LSB (leaf level) to MSB (root level), length=256
    pub siblings: Vec<H256>,
}

impl SmtProof {
    pub fn empty() -> Self {
        Self { siblings: vec![zero_hashes()[0]; 256] }
    }
}

#[derive(Clone, Debug)]
pub struct SparseMerkleTree {
    /// key -> value map (value is 32 bytes)
    store: HashMap<H256, H256>,
    /// current root
    root: H256,
}

impl Default for SparseMerkleTree {
    fn default() -> Self {
        Self {
            store: HashMap::new(),
            root: zero_hashes()[256],
        }
    }
}

impl SparseMerkleTree {
    pub fn new() -> Self { Self::default() }

    pub fn root(&self) -> H256 { self.root }

    pub fn get(&self, key: &H256) -> Option<H256> {
        self.store.get(key).cloned()
    }

    /// Update a leaf (insert or set). If `value` is None, delete leaf.
    pub fn update(&mut self, key: H256, value: Option<H256>) {
        if let Some(v) = value {
            self.store.insert(key, v);
        } else {
            self.store.remove(&key);
        }
        self.recompute_root();
    }

    fn recompute_root(&mut self) {
        // Note: This naive recomputation is O(n log N) but fine for devnet.
        // For production, switch to a persistent node store.
        let zeros = zero_hashes();
        let mut nodes: HashMap<usize, HashMap<u64, H256>> = HashMap::new();
        // Level 0: leaves
        let mut leaves: HashMap<u64, H256> = HashMap::new();
        for (k, v) in self.store.iter() {
            let idx = Self::key_index(k);
            // leaf hash = H(0x00 || key || value)
            let mut buf = [0u8; 1 + 32 + 32];
            buf[0] = 0x00;
            buf[1..33].copy_from_slice(k);
            buf[33..].copy_from_slice(v);
            leaves.insert(idx, h(&buf));
        }
        nodes.insert(0, leaves);

        for level in 0..256 {
            let cur = nodes.get(&level).cloned().unwrap_or_default();
            let mut next: HashMap<u64, H256> = HashMap::new();
            if cur.is_empty() {
                // all-zero subtree
                break;
            }
            for (idx, val) in cur.iter() {
                let sib_idx = idx ^ 1;
                let left_is_me = idx & 1 == 0;
                let left = if left_is_me { *val } else { cur.get(&sib_idx).cloned().unwrap_or(zeros[level]) };
                let right = if left_is_me { cur.get(&sib_idx).cloned().unwrap_or(zeros[level]) } else { *val };
                let parent = h2(&left, &right);
                next.insert(idx >> 1, parent);
            }
            nodes.insert(level + 1, next);
        }
        self.root = nodes.get(&256).and_then(|m| m.get(&0).cloned()).unwrap_or(zeros[256]);
    }

    /// Build a Merkle proof for `key` with respect to current tree.
    pub fn prove(&self, key: &H256) -> (Option<H256>, SmtProof) {
        let zeros = zero_hashes();
        let mut siblings = Vec::with_capacity(256);
        let mut idx = Self::key_index(key);
        // For proof construction we need the sibling hash at each level.
        // We recompute per-level nodes on the fly using the set of present leaves.
        let mut level_nodes: HashMap<u64, H256> = HashMap::new();
        // leaf hash
        let leaf_val = self.get(key);
        if let Some(v) = leaf_val {
            let mut buf = [0u8; 1 + 32 + 32];
            buf[0] = 0x00;
            buf[1..33].copy_from_slice(key);
            buf[33..].copy_from_slice(&v);
            level_nodes.insert(Self::key_index(key), h(&buf));
        }
        // Build sparse presence map for all leaves
        let mut present: HashMap<u64, H256> = HashMap::new();
        for (k, v) in self.store.iter() {
            let kidx = Self::key_index(k);
            let mut buf = [0u8; 1 + 32 + 32];
            buf[0] = 0x00;
            buf[1..33].copy_from_slice(k);
            buf[33..].copy_from_slice(v);
            present.insert(kidx, h(&buf));
        }

        for level in 0..256 {
            let sib_idx = idx ^ 1;
            // sibling at this level
            let sib = present.get(&sib_idx).cloned().unwrap_or(zeros[level]);
            siblings.push(sib);

            // propagate present map up a level
            let mut next: HashMap<u64, H256> = HashMap::new();
            if present.is_empty() {
                // nothing to propagate
            } else {
                for (i, val) in present.iter() {
                    let si = i ^ 1;
                    let left_is_me = i & 1 == 0;
                    let left = if left_is_me { *val } else { present.get(&si).cloned().unwrap_or(zeros[level]) };
                    let right = if left_is_me { present.get(&si).cloned().unwrap_or(zeros[level]) } else { *val };
                    next.insert(i >> 1, h2(&left, &right));
                }
            }
            present = next;
            idx >>= 1;
        }

        (leaf_val, SmtProof { siblings })
    }

    /// Verify an inclusion (or non-inclusion) proof.
    /// If `value` is Some, verify inclusion of (key,value). If None, prove absence.
    pub fn verify(root: &H256, key: &H256, value: Option<&H256>, proof: &SmtProof) -> bool {
        let zeros = zero_hashes();
        if proof.siblings.len() != 256 { return false; }

        let mut cur = if let Some(v) = value {
            let mut buf = [0u8; 1 + 32 + 32];
            buf[0] = 0x00;
            buf[1..33].copy_from_slice(key);
            buf[33..].copy_from_slice(v);
            h(&buf)
        } else {
            zeros[0]
        };

        let mut idx = Self::key_index(key);
        for (level, sib) in proof.siblings.iter().enumerate() {
            let left_is_me = idx & 1 == 0;
            let left = if left_is_me { cur } else { *sib };
            let right = if left_is_me { *sib } else { cur };
            cur = h2(&left, &right);
            idx >>= 1;
        }
        &cur == root
    }

    #[inline]
    fn key_index(key: &H256) -> u64 {
        // use the lowest 64 bits of the 256-bit key as binary path index
        // (devnet-friendly; can be extended to full 256-bit if desired)
        let mut arr = [0u8; 8];
        arr.copy_from_slice(&key[24..32]);
        u64::from_be_bytes(arr)
    }
}
