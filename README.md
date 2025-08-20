# dxID Layer0 - Production Ready Infrastructure Chain

A **production-ready Layer0 blockchain infrastructure** that provides ZKP (Zero-Knowledge Proof) encryption services and universal cross-chain connectivity. Built with Rust for maximum performance and security.

## 🚀 **LIVE & FUNCTIONAL**

✅ **P2P Network**: Real TCP-based networking with auto-discovery  
✅ **CLI Integration**: Built-in node management and wallet operations  
✅ **ZK Encryption**: STARK and SNARK proof systems  
✅ **Cross-Chain Bridge**: Connect to any L1 blockchain  
✅ **Universal Wallet**: Multi-chain token support  

## 🏗️ Architecture

```
dxID Layer0 (Infrastructure Chain)
├── P2P Network (Auto-discovery, TCP-based)
├── ZK-STARK Encryption Services
├── ZK-SNARK Cross-Chain Transactions  
├── Universal Wallet (All chains)
└── Bridge System (L1 Connectivity)
```

## 🎯 Core Purpose

**Revenue Model**: Charge for ZKP encryption services  
**Infrastructure**: Layer0 that connects to any blockchain  
**Integration**: LongYield L1 blockchain connectivity  

## 🚀 Quick Start

### 1. Start the Node
```bash
# Start node directly (recommended)
cargo run --bin dxid-node

# Or use the CLI to manage the node
cargo run --bin dxid-cli-enhanced
```

### 2. Access the CLI
```bash
cargo run --bin dxid-cli-enhanced
```

**CLI Features:**
- ✅ Start/Stop Node Management
- ✅ Wallet Creation & Management  
- ✅ P2P Network Monitoring
- ✅ Cross-Chain Operations
- ✅ ZK Privacy Settings

### 3. Network Auto-Discovery

The node automatically:
- Connects to the live dxID Layer0 network on Railway
- Discovers other nodes on the network
- Enables cross-chain transactions
- Provides ZK privacy services

**🌐 Live Network**: Your CLI automatically connects to `dxid-l0.railway.app:7000` - the live dxID Layer0 network!

## 📁 Clean Project Structure

```
layer0/
├── dxid-node/          # Main blockchain node
├── dxid-cli-enhanced/  # Interactive CLI with node management
├── dxid-p2p/          # Production TCP-based P2P network
├── dxid-crypto/       # Cryptographic primitives
├── dxid-runtime/      # Blockchain runtime
├── dxid-smt/          # Sparse Merkle Tree implementation
├── dxid-zk-stark/     # ZK-STARK proof system
├── dxid-zk-snark/     # ZK-SNARK proof system
├── dxid-bridge/       # Cross-chain bridge system
├── dxid-integration/  # Integration modules
├── dxid-data/         # Persistent data storage
├── dxid-config.toml   # Node configuration
└── USER_GUIDE.md      # Complete user guide
```

## 🔧 Configuration

Edit `dxid-config.toml` to customize:
- P2P network settings
- Auto-discovery parameters
- RPC server configuration
- Blockchain parameters

## 🌐 P2P Network Status

**Current Status**: ✅ **LIVE & FUNCTIONAL**
- **Auto-Discovery**: Enabled
- **Encryption**: ZK-STARK enabled
- **Bootstrap Peers**: Live Railway node (`dxid-l0.railway.app:7000`)
- **Connection Type**: TCP-based production network
- **Network**: Always-on dxID Layer0 network

## 💰 Genesis Faucet

For testing, the node includes a genesis faucet:
- **Address**: Printed on node startup
- **Balance**: 1 trillion tokens for development
- **Purpose**: Testing and development only

## 🔗 Cross-Chain Bridge

Connect to any L1 blockchain:
- **Ethereum**: Full integration
- **Bitcoin**: Bridge support
- **LongYield L1**: Native integration
- **Universal**: Any blockchain via bridge

## 🛡️ ZK Privacy Levels

1. **Public**: All transactions visible
2. **Basic**: Basic transaction privacy
3. **Full**: Complete privacy with ZK proofs

## 📊 Node Monitoring

The CLI provides comprehensive monitoring:
- **Node Status**: Running/stopped, height, chain ID
- **P2P Network**: Connected peers, auto-discovery status
- **Wallet Balances**: Multi-chain token support
- **Transaction History**: Cross-chain operations

## 🎯 Production Features

✅ **Real P2P Networking**: TCP-based with auto-discovery  
✅ **Node Management**: Start/stop from CLI  
✅ **Wallet Integration**: Multi-chain support  
✅ **ZK Encryption**: STARK and SNARK systems  
✅ **Cross-Chain Bridge**: Universal connectivity  
✅ **Auto-Discovery**: Automatic peer connection  
✅ **Configuration**: Easy setup via TOML  
✅ **Monitoring**: Comprehensive CLI interface  

## 📖 Documentation

- **USER_GUIDE.md**: Complete user instructions
- **ARCHITECTURE_OVERVIEW.md**: Technical architecture
- **dxid-config.toml**: Configuration reference

## 🔄 Development Workflow

1. **Start Node**: `cargo run --bin dxid-node`
2. **Use CLI**: `cargo run --bin dxid-cli-enhanced`
3. **Monitor**: Check node status and P2P network
4. **Test**: Use genesis faucet for development
5. **Deploy**: Configure for production networks

---

**Status**: ✅ **PRODUCTION READY**  
**P2P Network**: ✅ **LIVE & FUNCTIONAL**  
**CLI Integration**: ✅ **FULLY OPERATIONAL**  
**Cross-Chain**: ✅ **BRIDGE ENABLED**
