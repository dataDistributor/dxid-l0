use std::collections::HashMap;
use dxid_runtime::{State, Account, SparseMerkleTree};
use blake3;

fn main() {
    println!("Testing State Root Uniqueness...");
    
    // Create a test state
    let mut state = State {
        accounts: HashMap::new(),
        height: 0,
        last_block_hash: [0u8; 32],
        state_root: [0u8; 32],
        layer0_circulating: 0,
        longyield_circulating: 0,
        smt: SparseMerkleTree::new(),
    };
    
    // Test multiple blocks and ensure state roots are different
    let mut previous_roots = Vec::new();
    
    for block_height in 1..=10 {
        state.height = block_height;
        
        // Simulate the empty block state root calculation
        let empty_block_key = [0u8; 32];
        let mut empty_block_data = Vec::new();
        empty_block_data.extend_from_slice(&state.height.to_le_bytes());
        empty_block_data.extend_from_slice(&(block_height * 1000).to_le_bytes()); // Simulate timestamp
        
        let empty_block_hash = blake3::hash(&empty_block_data);
        state.smt.update(empty_block_key, Some(*empty_block_hash.as_bytes()));
        state.state_root = state.smt.root();
        
        let root_hex = hex::encode(state.state_root);
        println!("Block {}: State Root = {}", block_height, root_hex);
        
        // Check if this root is unique
        if previous_roots.contains(&root_hex) {
            println!("❌ ERROR: Duplicate state root found!");
            return;
        }
        previous_roots.push(root_hex);
    }
    
    println!("✅ SUCCESS: All state roots are unique!");
}
