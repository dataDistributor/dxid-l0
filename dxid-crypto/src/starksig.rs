use anyhow::{anyhow, Result};
use rand::RngCore;
use serde::{Deserialize, Serialize};

/// Public key representation = BLAKE3 hash of secret (dev) — post-quantum hash-based auth shape.
/// In prod we prove knowledge of the preimage via STARK.
pub type PublicKeyHash = [u8; 32];

/// Secret key material (dev). In prod, this is just the preimage witness.
#[derive(Clone, Serialize, Deserialize)]
pub struct SecretKey {
    /// 32 bytes of secret entropy (dev). In a proper STARK scheme this is the witness.
    pub bytes: [u8; 32],
}

#[derive(Clone, Serialize, Deserialize, Debug)]
pub struct StarkSignature {
    /// Message hash (blake3 of message bytes).
    pub msg_hash: [u8; 32],
    /// "Signature" = blake3(secret || msg_hash) in dev engine.
    /// In prod, this is *not* used; the STARK proof attests the relation.
    pub sig: [u8; 32],
    /// STARK proof bytes. In the dev engine this encodes the raw secret so the verifier
    /// can emulate a proof check. In prod, this holds actual STARK proof bytes.
    pub proof: StarkProof,
    /// Public key commitment (hash of secret).
    pub pubkey_hash: PublicKeyHash,
    /// Nonce to avoid replay.
    pub nonce: u64,
}

#[derive(Clone, Serialize, Deserialize, Debug)]
pub struct StarkProof {
    /// Opaque proof bytes. Dev engine encodes the raw secret; replace with real proof later.
    pub bytes: Vec<u8>,
}

pub trait StarkSignEngine: Send + Sync + 'static {
    /// Generate dev/prod "secret" and its public commitment (hash root).
    fn generate_keys(&self) -> Result<(SecretKey, PublicKeyHash)>;

    /// Sign a message with nonce, producing a STARK-backed signature/proof tuple.
    fn sign(&self, secret: &SecretKey, msg: &[u8], nonce: u64) -> Result<StarkSignature>;

    /// Verify a STARK-backed signature/proof for the given message and pubkey hash.
    fn verify(&self, sig: &StarkSignature, msg: &[u8]) -> Result<()>;
}

/// Dev engine: transparent “STARK-like” flow so the chain logic works today.
/// - pubkey = H(sk)
/// - msg_hash = H(msg)
/// - sig = H(sk || msg_hash || nonce)
/// - proof.bytes = sk (so verifier recomputes and checks H(sk)=pubkey, H(sk||msg_hash)=sig)
/// Replace this with a real Winterfell proof without changing node/CLI/runtime code.
pub struct DevStarkEngine;

impl DevStarkEngine {
    fn h(bytes: &[u8]) -> [u8; 32] {
        *blake3::hash(bytes).as_bytes()
    }
}

impl StarkSignEngine for DevStarkEngine {
    fn generate_keys(&self) -> Result<(SecretKey, PublicKeyHash)> {
        let mut sk = [0u8; 32];
        rand::thread_rng().fill_bytes(&mut sk);
        let pk = Self::h(&sk);
        Ok((SecretKey { bytes: sk }, pk))
    }

    fn sign(&self, secret: &SecretKey, msg: &[u8], nonce: u64) -> Result<StarkSignature> {
        let msg_hash = Self::h(msg);
        let mut pre = Vec::with_capacity(32 + 32 + 8);
        pre.extend_from_slice(&secret.bytes);
        pre.extend_from_slice(&msg_hash);
        pre.extend_from_slice(&nonce.to_le_bytes());
        let sig = Self::h(&pre);
        let pubkey_hash = Self::h(&secret.bytes);
        let proof = StarkProof {
            bytes: secret.bytes.to_vec(), // dev only
        };
        Ok(StarkSignature {
            msg_hash,
            sig,
            proof,
            pubkey_hash,
            nonce,
        })
    }

    fn verify(&self, sig: &StarkSignature, msg: &[u8]) -> Result<()> {
        let msg_hash = Self::h(msg);
        if msg_hash != sig.msg_hash {
            return Err(anyhow!("message hash mismatch"));
        }
        // Dev "proof": recover secret from proof bytes and recompute relations.
        if sig.proof.bytes.len() != 32 {
            return Err(anyhow!("invalid dev proof encoding"));
        }
        let mut sk = [0u8; 32];
        sk.copy_from_slice(&sig.proof.bytes);
        let pk = Self::h(&sk);
        if pk != sig.pubkey_hash {
            return Err(anyhow!("pubkey hash mismatch"));
        }
        let mut pre = Vec::with_capacity(32 + 32 + 8);
        pre.extend_from_slice(&sk);
        pre.extend_from_slice(&msg_hash);
        pre.extend_from_slice(&sig.nonce.to_le_bytes());
        let want = Self::h(&pre);
        if want != sig.sig {
            return Err(anyhow!("signature mismatch"));
        }
        Ok(())
    }
}

/// Global dev engine instance.
pub static DEV_ENGINE: DevStarkEngine = DevStarkEngine;
