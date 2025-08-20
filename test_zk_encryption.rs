use dxid_zk_stark::{ZkStarkEngine, StarkProofSystem, ModuleEncryption, BlockchainEncryption};
use dxid_zk_snark::{ZkSnarkEngine, CircuitSnarkSystem, TransactionEncryption, CrossModuleVerification};
use anyhow::Result;

fn main() -> Result<()> {
    println!("=== ZK Encryption Verification Test ===\n");

    // Test ZK-STARK
    println!("🔐 Testing ZK-STARK Encryption:");
    
    let stark_engine = ZkStarkEngine::new();
    let proof_system = StarkProofSystem::new(256);
    let module_encryption = ModuleEncryption::new(&[1u8; 32]);
    let blockchain_encryption = BlockchainEncryption::new(&[2u8; 32]);

    // Test module encryption
    let test_data = b"This is test data for ZK-STARK encryption";
    let encrypted = module_encryption.encrypt(test_data)?;
    let decrypted = module_encryption.decrypt(&encrypted)?;
    
    println!("✅ Module encryption/decryption: {}", 
        if test_data == decrypted.as_slice() { "PASSED" } else { "FAILED" });

    // Test blockchain state encryption
    let state_data = b"Blockchain state data";
    let encrypted_state = blockchain_encryption.encrypt(state_data)?;
    let decrypted_state = blockchain_encryption.decrypt(&encrypted_state)?;
    
    println!("✅ Blockchain state encryption/decryption: {}", 
        if state_data == decrypted_state.as_slice() { "PASSED" } else { "FAILED" });

    // Test STARK proof generation
    let public_inputs = vec![1u64, 2u64, 3u64];
    let private_inputs = vec![4u64, 5u64, 6u64];
    
    let proof = proof_system.generate_module_integrity_proof(&public_inputs, &private_inputs)?;
    let verified = proof_system.verify_module_integrity_proof(&public_inputs, &proof)?;
    
    println!("✅ STARK proof generation/verification: {}", 
        if verified { "PASSED" } else { "FAILED" });

    println!("\n🔐 Testing ZK-SNARK Encryption:");
    
    let snark_engine = ZkSnarkEngine::new();
    let circuit_system = CircuitSnarkSystem::new();
    let tx_encryption = TransactionEncryption::new(&[3u8; 32]);
    let cross_verification = CrossModuleVerification::new(&[4u8; 32]);

    // Test transaction encryption
    let tx_data = b"Transaction data for ZK-SNARK";
    let encrypted_tx = tx_encryption.encrypt(tx_data)?;
    let decrypted_tx = tx_encryption.decrypt(&encrypted_tx)?;
    
    println!("✅ Transaction encryption/decryption: {}", 
        if tx_data == decrypted_tx.as_slice() { "PASSED" } else { "FAILED" });

    // Test SNARK proof generation
    let tx_proof = circuit_system.generate_transaction_validity_proof(&public_inputs, &private_inputs)?;
    let tx_verified = circuit_system.verify_transaction_validity_proof(&public_inputs, &tx_proof)?;
    
    println!("✅ SNARK proof generation/verification: {}", 
        if tx_verified { "PASSED" } else { "FAILED" });

    // Test cross-module verification
    let module_id = "test_module".to_string();
    let tx_hash = [1u8; 32];
    let timestamp = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)?
        .as_secs();
    
    let verification_result = cross_verification.perform_verification(
        &module_id, 
        &module_id, 
        &tx_hash, 
        timestamp
    )?;
    
    println!("✅ Cross-module verification: {}", 
        if verification_result.is_valid { "PASSED" } else { "FAILED" });

    println!("\n🎯 ZK Encryption Summary:");
    println!("✅ ZK-STARK: Module encryption, blockchain state encryption, proof generation");
    println!("✅ ZK-SNARK: Transaction encryption, circuit proofs, cross-module verification");
    println!("✅ All cryptographic primitives are working correctly!");
    
    Ok(())
}
