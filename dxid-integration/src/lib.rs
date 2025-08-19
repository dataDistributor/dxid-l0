//! dxid-integration: Integration layer for P2P, ZK-STARK, and ZK-SNARK components
//! 
//! This module provides:
//! - Unified interface for all ZK components
//! - P2P network integration with ZK encryption
//! - Module and transaction lifecycle management
//! - Cross-module communication with encryption

use anyhow::{anyhow, Result};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::RwLock;
use tracing::{info, warn};

use dxid_p2p::{Network, NetConfig, PeerInfo, ModuleMessage};

#[cfg(feature = "full_zk")]
use dxid_zk_stark::ZkStarkEngine;
#[cfg(feature = "full_zk")]
use dxid_zk_snark::ZkSnarkEngine;

/// Main integration engine for dxID
pub struct DxidIntegration {
    p2p_network: Network,
    #[cfg(feature = "full_zk")]
    zk_stark_engine: ZkStarkEngine,
    #[cfg(feature = "full_zk")]
    zk_snark_engine: ZkSnarkEngine,
    modules: RwLock<HashMap<String, ModuleInfo>>,
    config: IntegrationConfig,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ModuleInfo {
    pub module_id: String,
    pub module_type: String, // "blockchain", "identity", "storage", etc.
    pub is_encrypted: bool,
    pub zk_stark_proof: Option<Vec<u8>>,
    pub capabilities: Vec<String>,
    pub last_updated: u64,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct IntegrationConfig {
    pub enable_zk_stark: bool,
    pub enable_zk_snark: bool,
    pub p2p_config: NetConfig,
    pub module_encryption_level: u32, // 0 = none, 1 = basic, 2 = full
    pub transaction_encryption_level: u32, // 0 = none, 1 = basic, 2 = full
}

impl Default for IntegrationConfig {
    fn default() -> Self {
        Self {
            enable_zk_stark: true,
            enable_zk_snark: true,
            p2p_config: NetConfig {
                chain_id: 1337,
                genesis_hash: "0000000000000000000000000000000000000000000000000000000000000000".to_string(),
                listen_addr: "0.0.0.0:7000".to_string(),
                bootstrap_peers: Vec::new(),
                max_peers: 50,
                heartbeat_interval: 30,
                auto_discovery: true,
                discovery_interval: 60,
                enable_encryption: false,
            },
            module_encryption_level: 2,
            transaction_encryption_level: 2,
        }
    }
}

impl DxidIntegration {
    pub async fn new(config: IntegrationConfig) -> Result<Self> {
        info!("Initializing dxID integration layer...");
        
        // Start P2P network
        let p2p_network = dxid_p2p::start(config.p2p_config.clone()).await?;
        
        #[cfg(feature = "full_zk")]
        let zk_stark_engine = if config.enable_zk_stark {
            info!("Initializing ZK-STARK engine...");
            ZkStarkEngine::new()?
        } else {
            return Err(anyhow!("ZK-STARK is required but not available"));
        };
        
        #[cfg(feature = "full_zk")]
        let zk_snark_engine = if config.enable_zk_snark {
            info!("Initializing ZK-SNARK engine...");
            ZkSnarkEngine::new()?
        } else {
            return Err(anyhow!("ZK-SNARK is required but not available"));
        };

        Ok(Self {
            p2p_network,
            #[cfg(feature = "full_zk")]
            zk_stark_engine,
            #[cfg(feature = "full_zk")]
            zk_snark_engine,
            modules: RwLock::new(HashMap::new()),
            config,
        })
    }

    /// Register a new module with optional ZK-STARK encryption
    pub async fn register_module(&self, module_id: &str, module_type: &str, module_data: &[u8]) -> Result<()> {
        info!("Registering module: {} ({})", module_id, module_type);
        
        #[cfg(feature = "full_zk")]
        let (is_encrypted, zk_stark_proof) = if self.config.module_encryption_level > 0 {
            // Encrypt module with ZK-STARK
            let encrypted_module = self.zk_stark_engine.encrypt_module(module_id, module_data).await?;
            let proof_bytes = bincode::serialize(&encrypted_module.proof)?;
            (true, Some(proof_bytes))
        } else {
            (false, None)
        };

        #[cfg(not(feature = "full_zk"))]
        let (is_encrypted, zk_stark_proof) = (false, None);

        let module_info = ModuleInfo {
            module_id: module_id.to_string(),
            module_type: module_type.to_string(),
            is_encrypted,
            zk_stark_proof: zk_stark_proof.clone(),
            capabilities: vec!["basic".to_string()],
            last_updated: std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)?
                .as_secs(),
        };

        let mut modules = self.modules.write().unwrap();
        modules.insert(module_id.to_string(), module_info);

        // Announce module on P2P network
        let module_msg = ModuleMessage {
            module_id: module_id.to_string(),
            message_type: "module_registered".to_string(),
            data: module_data.to_vec(),
            zk_proof: zk_stark_proof,
        };
        
        self.p2p_network.publish_module_message(module_msg).await?;
        
        info!("Module {} registered successfully", module_id);
        Ok(())
    }

    /// Send a transaction between modules with ZK-SNARK encryption
    pub async fn send_transaction(&self, from_module: &str, to_module: &str, data: &[u8]) -> Result<String> {
        let tx_id = uuid::Uuid::new_v4().to_string();
        info!("Sending transaction {} from {} to {}", tx_id, from_module, to_module);

        #[cfg(feature = "full_zk")]
        {
            if self.config.transaction_encryption_level > 0 {
                // Create transaction with ZK-SNARK encryption
                let tx = dxid_zk_snark::Transaction {
                    id: tx_id.clone(),
                    from_module: from_module.to_string(),
                    to_module: to_module.to_string(),
                    data: data.to_vec(),
                    timestamp: std::time::SystemTime::now()
                        .duration_since(std::time::UNIX_EPOCH)?
                        .as_secs(),
                };

                // Encrypt transaction
                let encrypted_tx = self.zk_snark_engine.encrypt_transaction(&tx).await?;
                
                // Verify cross-module transaction
                let is_valid = self.zk_snark_engine.verify_cross_module_transaction(&tx).await?;
                if !is_valid {
                    return Err(anyhow!("Cross-module transaction validation failed"));
                }

                // Publish encrypted transaction on P2P network
                let module_msg = ModuleMessage {
                    module_id: "transaction".to_string(),
                    message_type: "encrypted_transaction".to_string(),
                    data: bincode::serialize(&encrypted_tx)?,
                    zk_proof: Some(bincode::serialize(&encrypted_tx.proof)?),
                };
                
                self.p2p_network.publish_module_message(module_msg).await?;
            } else {
                // No encryption - send plain transaction
                let tx = dxid_zk_snark::Transaction {
                    id: tx_id.clone(),
                    from_module: from_module.to_string(),
                    to_module: to_module.to_string(),
                    data: data.to_vec(),
                    timestamp: std::time::SystemTime::now()
                        .duration_since(std::time::UNIX_EPOCH)?
                        .as_secs(),
                };

                // Publish plain transaction on P2P network
                let module_msg = ModuleMessage {
                    module_id: "transaction".to_string(),
                    message_type: "plain_transaction".to_string(),
                    data: bincode::serialize(&tx)?,
                    zk_proof: None,
                };
                
                self.p2p_network.publish_module_message(module_msg).await?;
            }
        }

        #[cfg(not(feature = "full_zk"))]
        {
            // No ZK features - send plain transaction
            let module_msg = ModuleMessage {
                module_id: "transaction".to_string(),
                message_type: "plain_transaction".to_string(),
                data: data.to_vec(),
                zk_proof: None,
            };
            
            self.p2p_network.publish_module_message(module_msg).await?;
        }

        info!("Transaction {} sent successfully", tx_id);
        Ok(tx_id)
    }

    /// Get module information
    pub async fn get_module_info(&self, module_id: &str) -> Result<Option<ModuleInfo>> {
        let modules = self.modules.read().unwrap();
        Ok(modules.get(module_id).cloned())
    }

    /// List all registered modules
    pub async fn list_modules(&self) -> Vec<ModuleInfo> {
        let modules = self.modules.read().unwrap();
        modules.values().cloned().collect()
    }

    /// Get P2P network statistics
    pub async fn get_network_stats(&self) -> dxid_p2p::NetworkStats {
        self.p2p_network.get_stats().await
    }

    /// Add a peer with specific capabilities
    pub async fn add_peer(&self, peer_id: &str, address: &str, capabilities: Vec<String>) -> Result<()> {
        let peer = PeerInfo {
            id: peer_id.to_string(),
            address: address.to_string(),
            last_seen: std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)?
                .as_secs(),
            capabilities,
            chain_id: Some(1337),
            is_connected: false,
            connection_attempts: 0,
            last_connection_attempt: 0,
            is_bootstrap: false,
        };
        
        self.p2p_network.add_peer(peer).await;
        Ok(())
    }

    /// Find peers with specific capabilities
    pub async fn find_peers_with_capability(&self, capability: &str) -> Vec<PeerInfo> {
        self.p2p_network.find_peers_with_capability(capability).await
    }

    /// Encrypt blockchain state with ZK-STARK
    pub async fn encrypt_blockchain_state(&self, state_data: &[u8]) -> Result<Vec<u8>> {
        #[cfg(feature = "full_zk")]
        {
            let encrypted_state = self.zk_stark_engine.encrypt_blockchain_state(state_data).await?;
            Ok(bincode::serialize(&encrypted_state)?)
        }
        
        #[cfg(not(feature = "full_zk"))]
        {
            Err(anyhow!("ZK-STARK not available"))
        }
    }

    /// Decrypt blockchain state with ZK-STARK
    pub async fn decrypt_blockchain_state(&self, encrypted_state_data: &[u8]) -> Result<Vec<u8>> {
        #[cfg(feature = "full_zk")]
        {
            let encrypted_state: dxid_zk_stark::EncryptedBlockchainState = bincode::deserialize(encrypted_state_data)?;
            self.zk_stark_engine.decrypt_blockchain_state(&encrypted_state).await
        }
        
        #[cfg(not(feature = "full_zk"))]
        {
            Err(anyhow!("ZK-STARK not available"))
        }
    }
}

// Re-export types for convenience (avoiding duplicates)
// pub use dxid_p2p::{Network, NetConfig, PeerInfo, ModuleMessage};

#[cfg(feature = "full_zk")]
pub use dxid_zk_stark::EncryptedBlockchainState;
#[cfg(feature = "full_zk")]
pub use dxid_zk_snark::Transaction;
