# dxID Network Discovery System

## 🌟 Automatic Network Discovery

The dxID blockchain now features **automatic network discovery** where the first node to start becomes the network host, and all subsequent nodes automatically connect to the same network.

## 🚀 How It Works

### **First Node (Network Host)**
When you run the first node:
```bash
cargo run --bin dxid-node
```

The system will:
1. 🌟 **Detect** it's the first node
2. 📡 **Become** the network host
3. 💾 **Save** network host information to `dxid-data/network_host.json`
4. 🔄 **Start** heartbeat updates to keep host info fresh

You'll see: `🌟 First node detected - becoming network host`

### **Subsequent Nodes**
When others run nodes:
```bash
cargo run --bin dxid-node
```

The system will:
1. 🔍 **Discover** existing network host
2. 🔗 **Connect** automatically to the host
3. 📊 **Sync** blockchain state
4. 🌐 **Join** the shared network

You'll see: `🔗 Connecting to existing network host: 192.168.1.100:7000`

## 🎯 Network Status

Check network status via RPC:
```bash
curl http://127.0.0.1:8545/network
```

Response:
```json
{
  "auto_discovery_enabled": true,
  "is_host": true,
  "host_info": {
    "address": "192.168.1.100",
    "port": 7000,
    "created_at": 1755523837,
    "last_seen": 1755523900,
    "is_stale": false
  },
  "chain_id": 1337
}
```

## 🔧 Advanced Options

### **Force Host Mode**
Make a specific node the host:
```bash
cargo run --bin dxid-node -- --force-host
```

### **Disable Auto-Discovery**
Use manual bootstrap peers:
```bash
cargo run --bin dxid-node -- --auto-discovery false --p2p-bootstrap 192.168.1.100:7000
```

### **Custom P2P Port**
```bash
cargo run --bin dxid-node -- --p2p-listen 0.0.0.0:8080
```

## 🌐 Network Requirements

### **Local Network**
- ✅ Works automatically on same WiFi/LAN
- ✅ Nodes discover each other via `network_host.json`

### **Internet/WAN**
- 🌐 Requires port forwarding (port 7000)
- 🌐 Or use VPN for secure connectivity
- 🌐 Public IP address needed for host

### **Firewall**
- 🔥 Open port 7000 (or your chosen port)
- 🔥 Allow UDP for peer discovery

## 📁 Network Files

The system creates these files in `dxid-data/`:
- `network_host.json` - Network host information
- `state.json` - Blockchain state (persistent)
- `blocks/` - Block data
- `checkpoints/` - State checkpoints
- `backups/` - Automatic backups

## 🔄 Network Recovery

### **Host Goes Down**
If the host node goes offline:
1. Other nodes will detect stale host info
2. After timeout, they can become new host
3. Or manually specify new bootstrap peer

### **Network Split**
If network splits:
- Each partition may elect new host
- When reconnected, they'll sync automatically
- Longest chain wins (standard blockchain rules)

## 🧪 Testing

### **Local Testing**
1. Start first node: `cargo run --bin dxid-node`
2. Start second node: `cargo run --bin dxid-node`
3. Check both show same block height
4. Submit transaction on one, see it on both

### **Network Testing**
```bash
# Check connected peers
curl http://127.0.0.1:8545/peers

# Check network status
curl http://127.0.0.1:8545/network

# Check blockchain status
curl http://127.0.0.1:8545/status
```

## 🎉 Benefits

- ✅ **Zero Configuration** - Just run the node
- ✅ **Automatic Discovery** - No manual peer setup
- ✅ **Persistent State** - Data survives restarts
- ✅ **Fault Tolerant** - Handles host failures
- ✅ **Scalable** - Easy to add more nodes

## 🚨 Troubleshooting

### **Nodes Not Connecting**
1. Check firewall settings
2. Verify port 7000 is open
3. Check `network_host.json` exists and is valid
4. Try `--force-host` on one node

### **Stale Host Info**
```bash
# Remove stale host info
rm dxid-data/network_host.json
# Restart nodes
```

### **Network Split**
```bash
# Force new host
cargo run --bin dxid-node -- --force-host
```

---

**Now anyone can run `cargo run --bin dxid-node` and automatically join your shared blockchain network!** 🎯

