# dxID Layer0 - User Guide

## 🚀 **Quick Start - Just Run and Connect!**

### **For End Users (Super Easy)**

1. **Start the Node**: Double-click `start-node.bat` or run:
   ```bash
   cargo run --bin dxid-node
   ```

2. **Use the CLI**: Run the enhanced CLI:
   ```bash
   cargo run --bin dxid-cli-enhanced
   ```

3. **That's it!** Your node will automatically:
   - Connect to the dxID network
   - Discover other peers
   - Enable cross-chain transactions
   - Provide ZK privacy services

---

## 🌐 **Automatic Network Discovery**

### **What Happens When You Start**

1. **Bootstrap Connection**: Your node automatically connects to main network nodes
2. **Peer Discovery**: Discovers other nodes on the network
3. **Cross-Chain Ready**: Can connect to Ethereum, Bitcoin, and other chains
4. **ZK Services**: Provides privacy and encryption services

### **Network Features**

- ✅ **Auto-Discovery**: Finds peers automatically
- ✅ **Bootstrap Nodes**: Connects to main network nodes
- ✅ **Reconnection**: Automatically reconnects if connection lost
- ✅ **Encryption**: All communications are encrypted
- ✅ **Cross-Chain**: Ready for multi-chain transactions

---

## 💰 **Getting Started with Wallets**

### **Create Your First Wallet**

1. Run the CLI: `cargo run --bin dxid-cli-enhanced`
2. Choose option `4` (Manage wallets)
3. Choose option `1` (Create New Wallet)
4. Give it a name (e.g., "my-wallet")
5. Save the secret key securely!

### **Fund Your Wallet**

Your wallet starts with 0 balance. To get tokens:

1. **Use the Genesis Faucet**: The node creates a faucet with 1 trillion tokens
2. **Check the Node Logs**: Look for the faucet secret key when starting the node
3. **Import Faucet Wallet**: Use the faucet secret to create a faucet wallet
4. **Send to Your Wallet**: Transfer tokens from faucet to your wallet

---

## 🔧 **CLI Features**

### **Main Menu Options**

- **`1`** - Check node status and network connectivity
- **`2`** - View wallet balances
- **`3`** - Send transactions
- **`4`** - Manage wallets (create, import, list)
- **`5`** - API key management
- **`6`** - Layer0 token operations
- **`7`** - Multi-chain token operations
- **`8`** - ZK privacy settings
- **`9`** - Node management (start/stop)

### **Network Status**

When you check node status (option `1`), you'll see:

```
P2P Network Status:
  Auto-Discovery: ✅ Enabled
  Connected Peers: 3
  Total Peers: 5
  Bootstrap Peers: 2
  ZK-STARK Peers: 3
  ZK-SNARK Peers: 2
```

---

## 🌍 **Cross-Chain Features**

### **Supported Chains**

- **Layer0**: dxID infrastructure chain
- **Ethereum**: Mainnet and testnets
- **Bitcoin**: Mainnet and testnet
- **LongYield**: L1 blockchain (coming soon)

### **Cross-Chain Transactions**

1. **Bridge Setup**: Automatic bridge configuration
2. **Token Transfer**: Move tokens between chains
3. **ZK Privacy**: Private cross-chain transactions
4. **Fee Optimization**: Best route selection

---

## 🔐 **ZK Privacy Features**

### **Privacy Levels**

- **Public**: All transactions visible (default)
- **Basic**: Basic transaction privacy
- **Full**: Complete privacy with ZK proofs

### **ZK-STARK vs ZK-SNARK**

- **ZK-STARK**: Post-quantum secure, no trusted setup
- **ZK-SNARK**: Faster proofs, smaller size
- **Auto-Selection**: System chooses best for your use case

---

## ⚙️ **Configuration**

### **Network Settings**

Edit `dxid-config.toml` to customize:

```toml
[network]
enable_p2p = true
listen_addr = "0.0.0.0:7000"
auto_discovery = true
max_peers = 50
```

### **Bootstrap Nodes**

Add your own bootstrap nodes:

```toml
bootstrap_peers = [
    "your-node.com:7000",
    "friend-node.com:7000",
]
```

---

## 🛠️ **Troubleshooting**

### **Common Issues**

**"No peers connected"**
- This is normal for new nodes
- Auto-discovery will find peers automatically
- Check your firewall settings

**"Account doesn't exist yet"**
- Create a wallet first
- Use the genesis faucet to get initial tokens
- Send a transaction to activate the account

**"Node won't start"**
- Check if port 8545 is available
- Kill any existing dxid-node processes
- Check firewall settings

### **Getting Help**

1. **Check Logs**: Look at node output for error messages
2. **Network Status**: Use CLI option `1` to check connectivity
3. **Restart Node**: Use CLI option `9` to restart the node

---

## 🎯 **Advanced Usage**

### **Running Multiple Nodes**

1. **Different Ports**: Change ports in config
2. **Different Data Directories**: Use `--data-dir` flag
3. **Network Segregation**: Use different chain IDs

### **Custom Networks**

1. **Private Network**: Set `chain_id` to unique value
2. **Custom Bootstrap**: Add your own bootstrap nodes
3. **Genesis Configuration**: Customize initial token distribution

---

## 🔗 **Integration**

### **API Access**

- **RPC Endpoint**: `http://localhost:8545`
- **API Keys**: Use CLI option `5` to manage
- **WebSocket**: Available for real-time updates

### **SDK Integration**

```javascript
// Connect to dxID node
const dxid = new DxidClient('http://localhost:8545', apiKey);

// Send transaction
await dxid.sendTransaction({
    from: walletAddress,
    to: recipientAddress,
    amount: 1000,
    zkPrivacy: 'full'
});
```

---

## 🚀 **What's Next**

### **Upcoming Features**

- **GUI Wallet**: Beautiful web interface
- **Mobile App**: iOS and Android wallets
- **DeFi Integration**: DEX and lending protocols
- **NFT Support**: Cross-chain NFT transfers
- **Enterprise Features**: Multi-sig and compliance tools

### **Community**

- **Discord**: Join our community
- **GitHub**: Contribute to development
- **Documentation**: Full API documentation
- **Tutorials**: Step-by-step guides

---

## 🎉 **You're Ready!**

Your dxID Layer0 node is now:
- ✅ Connected to the network
- ✅ Ready for cross-chain transactions
- ✅ Providing ZK privacy services
- ✅ Auto-discovering peers

**Start exploring the future of decentralized infrastructure!**
