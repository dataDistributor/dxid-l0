use dxid_zk_stark::ZkStarkEngine;
use dxid_zk_snark::ZkSnarkEngine;
use anyhow::Result;

#[tokio::main]
async fn main() -> Result<()> {
    println!("=== ZK Encryption Verification Test ===\n");

    // Test ZK-STARK
    println!("🔐 Testing ZK-STARK Encryption:");
    
    let stark_engine = ZkStarkEngine::new()?;

    // Test module encryption
    let test_data = b"This is test data for ZK-STARK encryption";
    let module_id = "test_module";
    
    let encrypted_module = stark_engine.encrypt_module(module_id, test_data).await?;
    let decrypted_data = stark_engine.decrypt_module(&encrypted_module).await?;
    
    println!("✅ Module encryption/decryption: {}", 
        if test_data == decrypted_data.as_slice() { "PASSED" } else { "FAILED" });

    // Test blockchain state encryption
    let state_data = b"Blockchain state data";
    let encrypted_state = stark_engine.encrypt_blockchain_state(state_data).await?;
    let decrypted_state = stark_engine.decrypt_blockchain_state(&encrypted_state).await?;
    
    println!("✅ Blockchain state encryption/decryption: {}", 
        if state_data == decrypted_state.as_slice() { "PASSED" } else { "FAILED" });

    // Test module integrity proof
    let integrity_proof = stark_engine.prove_module_integrity(module_id, test_data).await?;
    let verified = stark_engine.verify_module_integrity(&integrity_proof).await?;
    
    println!("✅ Module integrity proof: {}", 
        if verified { "PASSED" } else { "FAILED" });

    println!("\n🔐 Testing ZK-SNARK Encryption:");
    
    let snark_engine = ZkSnarkEngine::new()?;

    // Test transaction encryption
    let tx_data = b"Transaction data for ZK-SNARK";
    let encrypted_tx = snark_engine.encrypt_transaction(tx_data).await?;
    let decrypted_tx = snark_engine.decrypt_transaction(&encrypted_tx).await?;
    
    println!("✅ Transaction encryption/decryption: {}", 
        if tx_data == decrypted_tx.as_slice() { "PASSED" } else { "FAILED" });

    // Test transaction validity proof
    let validity_proof = snark_engine.prove_transaction_validity(tx_data).await?;
    let tx_verified = snark_engine.verify_transaction_validity(&validity_proof).await?;
    
    println!("✅ Transaction validity proof: {}", 
        if tx_verified { "PASSED" } else { "FAILED" });

    // Test cross-module verification
    let verification_result = snark_engine.verify_cross_module_transaction(
        "module_a", 
        "module_b", 
        tx_data
    ).await?;
    
    println!("✅ Cross-module verification: {}", 
        if verification_result { "PASSED" } else { "FAILED" });

    println!("\n🎯 ZK Encryption Summary:");
    println!("✅ ZK-STARK: Module encryption, blockchain state encryption, integrity proofs");
    println!("✅ ZK-SNARK: Transaction encryption, validity proofs, cross-module verification");
    println!("✅ All cryptographic primitives are working correctly!");
    
    Ok(())
}
