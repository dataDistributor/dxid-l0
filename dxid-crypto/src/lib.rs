//! dxid-crypto: STARK-first signature/verification boundary for dxID L0.
//! - Default: DevStarkEngine (transparent toy engine) so you can run end-to-end now.
//! - Prod: switch to a real STARK engine behind the same trait (feature: "stark_winterfell").

pub mod starksig;

pub use starksig::{
    PublicKeyHash, SecretKey, StarkProof, StarkSignEngine, StarkSignature, DEV_ENGINE,
};

/// Re-export a single global engine instance for simplicity in node/runtime/cli.
/// Swap the engine by changing this pub use (or feature-gate).
pub use starksig::DEV_ENGINE as ENGINE;
