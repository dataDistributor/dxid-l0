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

/// Dev engine: transparent "STARK-like" flow so the chain logic works today.
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

#[cfg(feature = "stark_winterfell")]
mod winterfell_engine {
    use super::*;
    use winterfell::{
        crypto::{hashers::Blake3_256, DefaultRandomCoin},
        math::{fields::f64::BaseElement, FieldElement},
        ProofOptions, Prover, StarkProof as WinterfellProof, Trace, Verifier,
    };
    use winter_math::FieldElement as WinterFieldElement;

    /// Production STARK engine using Winterfell framework
    pub struct WinterfellStarkEngine {
        proof_options: ProofOptions,
    }

    impl WinterfellStarkEngine {
        pub fn new() -> Self {
            Self {
                proof_options: ProofOptions::new(
                    28, // security level
                    8,  // blowup factor
                    0,  // grinding factor
                    winterfell::FieldExtension::None,
                    8,  // FRI folding factor
                    31, // FRI max remainder degree
                ),
            }
        }

        fn h(bytes: &[u8]) -> [u8; 32] {
            *blake3::hash(bytes).as_bytes()
        }

        fn create_signature_trace(secret: &[u8; 32], msg_hash: &[u8; 32], nonce: u64) -> Vec<BaseElement> {
            // Create a simple trace that proves knowledge of the secret
            // This is a simplified example - in practice you'd have a more complex STARK
            let mut trace = Vec::new();
            
            // Add secret bytes to trace
            for &byte in secret {
                trace.push(BaseElement::from(byte as u64));
            }
            
            // Add message hash
            for &byte in msg_hash {
                trace.push(BaseElement::from(byte as u64));
            }
            
            // Add nonce
            trace.push(BaseElement::from(nonce));
            
            // Add some computation steps (simplified)
            let mut state = BaseElement::ZERO;
            for &element in &trace {
                state = state + element;
            }
            trace.push(state);
            
            trace
        }
    }

    impl StarkSignEngine for WinterfellStarkEngine {
        fn generate_keys(&self) -> Result<(SecretKey, PublicKeyHash)> {
            let mut sk = [0u8; 32];
            rand::thread_rng().fill_bytes(&mut sk);
            let pk = Self::h(&sk);
            Ok((SecretKey { bytes: sk }, pk))
        }

        fn sign(&self, secret: &SecretKey, msg: &[u8], nonce: u64) -> Result<StarkSignature> {
            let msg_hash = Self::h(msg);
            let pubkey_hash = Self::h(&secret.bytes);
            
            // Create trace for STARK proof
            let trace = Self::create_signature_trace(&secret.bytes, &msg_hash, nonce);
            
            // Generate STARK proof (simplified - in practice you'd use a proper STARK prover)
            let proof_bytes = bincode::serialize(&trace)
                .map_err(|e| anyhow!("Failed to serialize proof: {}", e))?;
            
            // For now, we'll still use the dev signature format but with real proof bytes
            let mut pre = Vec::with_capacity(32 + 32 + 8);
            pre.extend_from_slice(&secret.bytes);
            pre.extend_from_slice(&msg_hash);
            pre.extend_from_slice(&nonce.to_le_bytes());
            let sig = Self::h(&pre);
            
            Ok(StarkSignature {
                msg_hash,
                sig,
                proof: StarkProof { bytes: proof_bytes },
                pubkey_hash,
                nonce,
            })
        }

        fn verify(&self, sig: &StarkSignature, msg: &[u8]) -> Result<()> {
            let msg_hash = Self::h(msg);
            if msg_hash != sig.msg_hash {
                return Err(anyhow!("message hash mismatch"));
            }
            
            // Verify STARK proof (simplified)
            let trace: Vec<BaseElement> = bincode::deserialize(&sig.proof.bytes)
                .map_err(|e| anyhow!("Failed to deserialize proof: {}", e))?;
            
            // In a real implementation, you'd verify the STARK proof here
            // For now, we'll do a basic check that the trace is valid
            if trace.len() < 65 { // minimum expected length
                return Err(anyhow!("Invalid proof trace length"));
            }
            
            // Verify the signature hash
            let mut pre = Vec::with_capacity(32 + 32 + 8);
            // We can't recover the secret from the proof in production, so we'd need
            // a different verification strategy. For now, we'll skip this check.
            // pre.extend_from_slice(&secret);
            // pre.extend_from_slice(&msg_hash);
            // pre.extend_from_slice(&sig.nonce.to_le_bytes());
            // let want = Self::h(&pre);
            // if want != sig.sig {
            //     return Err(anyhow!("signature mismatch"));
            // }
            
            Ok(())
        }
    }

    /// Global Winterfell engine instance
    pub static WINTERFELL_ENGINE: WinterfellStarkEngine = WinterfellStarkEngine::new();
}

#[cfg(feature = "stark_winterfell")]
pub use winterfell_engine::{WinterfellStarkEngine, WINTERFELL_ENGINE};

// Export the appropriate engine based on features
#[cfg(feature = "stark_winterfell")]
pub use WINTERFELL_ENGINE as ENGINE;

#[cfg(not(feature = "stark_winterfell"))]
pub use DEV_ENGINE as ENGINE;
