//! dxid-p2p discovery: Simplified peer discovery and network management
//! 
//! This module provides:
//! - Local network discovery via UDP broadcast
//! - Bootstrap peer management
//! - Peer health monitoring

use anyhow::Result;
use serde::{Deserialize, Serialize};
use std::{
    collections::HashMap,
    net::{IpAddr, Ipv4Addr, SocketAddr, UdpSocket},
    sync::Arc,
    time::{Duration, Instant, SystemTime, UNIX_EPOCH},
};
use tokio::{
    sync::{mpsc, Mutex, RwLock},
    time::{interval, sleep},
};
use tracing::{debug, info, warn};

use crate::{NetConfig, PeerInfo};

// Discovery configuration
const DISCOVERY_PORT: u16 = 7001;
const DISCOVERY_MAGIC: &[u8; 4] = b"DXID";
const DISCOVERY_VERSION: u8 = 1;
const MAX_PEER_AGE: Duration = Duration::from_secs(300); // 5 minutes
const PEER_CLEANUP_INTERVAL: Duration = Duration::from_secs(60);
const DISCOVERY_BROADCAST_INTERVAL: Duration = Duration::from_secs(30);

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DiscoveryMessage {
    pub magic: [u8; 4],
    pub version: u8,
    pub message_type: DiscoveryMessageType,
    pub peer_id: String,
    pub chain_id: u32,
    pub listen_addr: String,
    pub capabilities: Vec<String>,
    pub timestamp: u64,
    pub ttl: u8,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum DiscoveryMessageType {
    Ping,
    Pong,
    Announce,
    Query,
    Response,
}

#[derive(Debug, Clone)]
pub struct DiscoveryPeer {
    pub info: PeerInfo,
    pub last_seen: Instant,
    pub is_reachable: bool,
}

impl DiscoveryPeer {
    fn new(info: PeerInfo) -> Self {
        Self {
            info,
            last_seen: Instant::now(),
            is_reachable: true,
        }
    }

    fn update_last_seen(&mut self) {
        self.last_seen = Instant::now();
    }

    fn is_stale(&self) -> bool {
        self.last_seen.elapsed() > MAX_PEER_AGE
    }
}

pub struct DiscoveryService {
    config: NetConfig,
    peer_id: String,
    peers: Arc<RwLock<HashMap<String, DiscoveryPeer>>>,
    running: Arc<Mutex<bool>>,
    network_tx: mpsc::Sender<PeerInfo>,
}

impl DiscoveryService {
    pub fn new(
        config: NetConfig,
        peer_id: String,
        network_tx: mpsc::Sender<PeerInfo>,
    ) -> Result<Self> {
        Ok(Self {
            config,
            peer_id,
            peers: Arc::new(RwLock::new(HashMap::new())),
            running: Arc::new(Mutex::new(false)),
            network_tx,
        })
    }

    pub async fn start(&mut self) -> Result<()> {
        let mut running = self.running.lock().await;
        if *running {
            return Ok(());
        }
        *running = true;
        drop(running);

        // Start discovery tasks
        self.spawn_discovery_tasks().await;

        info!("Discovery service started");
        Ok(())
    }

    pub async fn stop(&mut self) {
        let mut running = self.running.lock().await;
        *running = false;
        drop(running);
        
        info!("Discovery service stopped");
    }

    async fn spawn_discovery_tasks(&self) {
        let peers = self.peers.clone();
        let running = self.running.clone();
        let network_tx = self.network_tx.clone();
        let config = self.config.clone();
        let peer_id = self.peer_id.clone();

        // Task 1: Discovery broadcast
        let running_1 = running.clone();
        let config_1 = config.clone();
        let peer_id_1 = peer_id.clone();
        tokio::spawn(async move {
            let mut interval = interval(DISCOVERY_BROADCAST_INTERVAL);
            while *running_1.lock().await {
                interval.tick().await;
                if let Err(e) = Self::broadcast_announcement(&config_1, &peer_id_1).await {
                    warn!("Failed to broadcast announcement: {}", e);
                }
            }
        });

        // Task 2: Peer cleanup
        let peers_clone = peers.clone();
        let running_clone = running.clone();
        tokio::spawn(async move {
            let mut interval = interval(PEER_CLEANUP_INTERVAL);
            while *running_clone.lock().await {
                interval.tick().await;
                Self::cleanup_stale_peers(&peers_clone).await;
            }
        });

        // Task 3: Bootstrap peer connection
        let peers_clone = peers.clone();
        let running_clone = running.clone();
        let network_tx_clone = network_tx.clone();
        let config_3 = config.clone();
        tokio::spawn(async move {
            Self::connect_to_bootstrap_peers(&config_3, peers_clone, running_clone, network_tx_clone).await;
        });
    }

    async fn broadcast_announcement(config: &NetConfig, peer_id: &str) -> Result<()> {
        let message = DiscoveryMessage {
            magic: *DISCOVERY_MAGIC,
            version: DISCOVERY_VERSION,
            message_type: DiscoveryMessageType::Announce,
            peer_id: peer_id.to_string(),
            chain_id: config.chain_id,
            listen_addr: config.listen_addr.clone(),
            capabilities: vec!["zk-stark".to_string(), "zk-snark".to_string()],
            timestamp: SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap_or_default()
                .as_secs(),
            ttl: 3,
        };

        let message_data = serde_json::to_vec(&message)?;
        
        // Broadcast to local network
        let broadcast_addr = SocketAddr::new(IpAddr::V4(Ipv4Addr::BROADCAST), DISCOVERY_PORT);
        let socket = UdpSocket::bind("0.0.0.0:0")?;
        socket.set_broadcast(true)?;
        socket.send_to(&message_data, broadcast_addr)?;

        // Also send to specific network ranges
        for network in &["192.168.1.255", "10.0.0.255", "172.16.0.255"] {
            if let Ok(addr) = format!("{}:{}", network, DISCOVERY_PORT).parse::<SocketAddr>() {
                let _ = socket.send_to(&message_data, addr);
            }
        }

        Ok(())
    }

    async fn cleanup_stale_peers(peers: &Arc<RwLock<HashMap<String, DiscoveryPeer>>>) {
        let mut peers_guard = peers.write().await;
        let stale_peers: Vec<String> = peers_guard
            .iter()
            .filter(|(_, peer)| peer.is_stale())
            .map(|(id, _)| id.clone())
            .collect();

        for peer_id in stale_peers {
            peers_guard.remove(&peer_id);
            debug!("Removed stale peer: {}", peer_id);
        }
    }

    async fn connect_to_bootstrap_peers(
        config: &NetConfig,
        peers: Arc<RwLock<HashMap<String, DiscoveryPeer>>>,
        running: Arc<Mutex<bool>>,
        network_tx: mpsc::Sender<PeerInfo>,
    ) {
        let mut interval = interval(Duration::from_secs(60)); // Try bootstrap every minute
        
        while *running.lock().await {
            interval.tick().await;
            
            for bootstrap_addr in &config.bootstrap_peers {
                if let Ok(addr) = bootstrap_addr.parse::<SocketAddr>() {
                    let peer_info = PeerInfo {
                        id: format!("bootstrap-{}", addr),
                        address: bootstrap_addr.clone(),
                        last_seen: SystemTime::now()
                            .duration_since(UNIX_EPOCH)
                            .unwrap_or_default()
                            .as_secs(),
                        capabilities: vec!["bootstrap".to_string()],
                        chain_id: Some(config.chain_id),
                        is_connected: false,
                        connection_attempts: 0,
                        last_connection_attempt: 0,
                        is_bootstrap: true,
                    };

                    if let Err(e) = network_tx.send(peer_info).await {
                        warn!("Failed to send bootstrap peer: {}", e);
                    }
                }
            }
        }
    }

    pub async fn get_peers(&self) -> Vec<PeerInfo> {
        let peers = self.peers.read().await;
        peers.values().map(|dp| dp.info.clone()).collect()
    }

    pub async fn add_peer(&self, peer: PeerInfo) {
        let mut peers = self.peers.write().await;
        let discovery_peer = DiscoveryPeer::new(peer);
        peers.insert(discovery_peer.info.id.clone(), discovery_peer);
    }

    pub async fn remove_peer(&self, peer_id: &str) {
        let mut peers = self.peers.write().await;
        peers.remove(peer_id);
    }

    pub async fn update_peer_health(&self, peer_id: &str, is_reachable: bool) {
        let mut peers = self.peers.write().await;
        if let Some(peer) = peers.get_mut(peer_id) {
            peer.is_reachable = is_reachable;
            peer.update_last_seen();
        }
    }
}
