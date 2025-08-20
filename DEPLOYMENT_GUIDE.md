# dxID Layer0 Deployment Guide 🚀

## Overview

This guide covers deploying dxID Layer0 across multiple cloud providers for optimal cost and reliability:

- **Validators/Bootstrap Nodes**: Oracle Cloud Free Tier (Always-on)
- **Auxiliary Services**: Render/Railway (Cost-effective)

## 🏗️ Architecture

```
┌─────────────────┐    ┌─────────────────┐    ┌─────────────────┐
│   Oracle Cloud  │    │   Oracle Cloud  │    │   Render/Railway│
│   Free Tier     │    │   Free Tier     │    │   (Auxiliary)   │
│                 │    │                 │    │                 │
│ ┌─────────────┐ │    │ ┌─────────────┐ │    │ ┌─────────────┐ │
│ │ Validator 1 │ │◄──►│ │ Validator 2 │ │◄──►│ │   Explorer  │ │
│ │ Bootstrap   │ │    │ │ Bootstrap   │ │    │ │   Faucet    │ │
│ │ Always-on   │ │    │ │ Always-on   │ │    │ │ API Gateway │ │
│ └─────────────┘ │    │ └─────────────┘ │    │ └─────────────┘ │
└─────────────────┘    └─────────────────┘    └─────────────────┘
         │                       │                       │
         └───────────────────────┼───────────────────────┘
                                 │
                    ┌─────────────┴─────────────┐
                    │      P2P Network          │
                    │   (Auto-discovery)        │
                    └───────────────────────────┘
```

## 🟠 Oracle Cloud Free Tier Setup

### Prerequisites
- Oracle Cloud Free Tier account
- 2 ARM-based VMs (Always Free)
- Ubuntu 22.04 LTS

### VM Specifications (Free Tier)
- **CPU**: 4 ARM-based cores
- **RAM**: 24 GB
- **Storage**: 200 GB
- **Network**: 1 Gbps
- **Always Free**: ✅ Never expires

### Step 1: Create Oracle Cloud VMs

#### VM 1: Primary Validator
```bash
# Create VM
Name: dxid-validator-1
Shape: VM.Standard.A1.Flex
CPU: 4 cores
RAM: 24 GB
OS: Ubuntu 22.04
Public IP: Yes
```

#### VM 2: Secondary Validator
```bash
# Create VM
Name: dxid-validator-2
Shape: VM.Standard.A1.Flex
CPU: 4 cores
RAM: 24 GB
OS: Ubuntu 22.04
Public IP: Yes
```

### Step 2: Configure Security Lists

#### Inbound Rules
```
Source          Port    Protocol    Description
0.0.0.0/0      22      TCP         SSH
0.0.0.0/0      7000    TCP         P2P Communication
0.0.0.0/0      7001    UDP         P2P Discovery
0.0.0.0/0      8545    TCP         RPC API
0.0.0.0/0      80      TCP         HTTP (Explorer)
0.0.0.0/0      443     TCP         HTTPS (Explorer)
```

### Step 3: Deploy Validator Nodes

#### On Both VMs:
```bash
# Update system
sudo apt update && sudo apt upgrade -y

# Install Rust
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh
source ~/.cargo/env

# Install build dependencies
sudo apt install -y build-essential pkg-config libssl-dev

# Clone repository
git clone https://github.com/yourusername/dxid-layer0.git
cd dxid-layer0

# Build optimized release
cargo build --release

# Create systemd service
sudo tee /etc/systemd/system/dxid-validator.service > /dev/null <<EOF
[Unit]
Description=dxID Layer0 Validator Node
After=network.target

[Service]
Type=simple
User=ubuntu
WorkingDirectory=/home/ubuntu/dxid-layer0
ExecStart=/home/ubuntu/dxid-layer0/target/release/dxid-node \\
    --p2p-listen 0.0.0.0:7000 \\
    --rpc-listen 0.0.0.0:8545 \\
    --validator \\
    --bootstrap-peer YOUR_VM2_IP:7000
Restart=always
RestartSec=10
StandardOutput=journal
StandardError=journal

[Install]
WantedBy=multi-user.target
EOF

# Enable and start service
sudo systemctl daemon-reload
sudo systemctl enable dxid-validator
sudo systemctl start dxid-validator

# Check status
sudo systemctl status dxid-validator
```

### Step 4: Configure Bootstrap Peers

#### Update dxid-config.toml on both VMs:
```toml
# Primary Validator (VM1)
bootstrap_peers = [
    "YOUR_VM2_IP:7000"  # Secondary validator
]

# Secondary Validator (VM2)  
bootstrap_peers = [
    "YOUR_VM1_IP:7000"  # Primary validator
]
```

## 🔵 Render/Railway Auxiliary Services

### Service 1: Blockchain Explorer

#### Render Deployment
```yaml
# render.yaml
services:
  - type: web
    name: dxid-explorer
    env: node
    buildCommand: npm install
    startCommand: npm start
    envVars:
      - key: NODE_ENV
        value: production
      - key: VALIDATOR_RPC
        value: http://YOUR_VM1_IP:8545
      - key: BACKUP_RPC
        value: http://YOUR_VM2_IP:8545
```

#### Explorer Features
- **Real-time block explorer**
- **Transaction history**
- **Network statistics**
- **Wallet lookup**
- **API endpoints**

### Service 2: Faucet Service

#### Railway Deployment
```json
// railway.json
{
  "build": {
    "builder": "NIXPACKS"
  },
  "deploy": {
    "startCommand": "npm start",
    "healthcheckPath": "/health",
    "healthcheckTimeout": 300,
    "restartPolicyType": "ON_FAILURE"
  }
}
```

#### Faucet Features
- **Test token distribution**
- **Rate limiting**
- **Captcha protection**
- **Wallet verification**

### Service 3: API Gateway

#### Render API Service
```yaml
# api-gateway.yaml
services:
  - type: web
    name: dxid-api-gateway
    env: node
    buildCommand: npm install
    startCommand: npm start
    envVars:
      - key: VALIDATOR_ENDPOINTS
        value: "http://YOUR_VM1_IP:8545,http://YOUR_VM2_IP:8545"
      - key: RATE_LIMIT
        value: "1000"
```

#### API Gateway Features
- **Load balancing** across validators
- **Rate limiting**
- **Caching**
- **Authentication**
- **Metrics collection**

## 🔧 Configuration Management

### Environment Variables
```bash
# Oracle Cloud VMs
export DXID_VALIDATOR_MODE=true
export DXID_BOOTSTRAP_PEERS="YOUR_VM1_IP:7000,YOUR_VM2_IP:7000"
export DXID_RPC_ENDPOINT="0.0.0.0:8545"
export DXID_P2P_ENDPOINT="0.0.0.0:7000"

# Render/Railway Services
export VALIDATOR_RPC_ENDPOINTS="http://YOUR_VM1_IP:8545,http://YOUR_VM2_IP:8545"
export EXPLORER_PORT=3000
export FAUCET_RATE_LIMIT=100
export API_GATEWAY_PORT=8080
```

### Monitoring Setup

#### Oracle Cloud Monitoring
```bash
# Install monitoring tools
sudo apt install -y htop iotop nethogs

# Create monitoring script
cat > /home/ubuntu/monitor.sh << 'EOF'
#!/bin/bash
echo "=== dxID Validator Status ==="
sudo systemctl status dxid-validator --no-pager
echo ""
echo "=== System Resources ==="
free -h
echo ""
echo "=== Disk Usage ==="
df -h
echo ""
echo "=== Network Connections ==="
netstat -tulpn | grep :7000
netstat -tulpn | grep :8545
EOF

chmod +x /home/ubuntu/monitor.sh

# Add to crontab for regular monitoring
echo "*/5 * * * * /home/ubuntu/monitor.sh >> /var/log/dxid-monitor.log 2>&1" | crontab -
```

## 🚀 Deployment Scripts

### Automated Deployment Script
```bash
#!/bin/bash
# deploy-validator.sh

set -e

VM_IP=$1
VM_NAME=$2
BOOTSTRAP_PEER=$3

if [ -z "$VM_IP" ] || [ -z "$VM_NAME" ]; then
    echo "Usage: $0 <VM_IP> <VM_NAME> [BOOTSTRAP_PEER]"
    exit 1
fi

echo "Deploying dxID validator to $VM_NAME ($VM_IP)..."

# Copy deployment files
scp -r deployment/ ubuntu@$VM_IP:~/
scp target/release/dxid-node ubuntu@$VM_IP:~/

# Execute deployment
ssh ubuntu@$VM_IP << EOF
    set -e
    
    # Update system
    sudo apt update && sudo apt upgrade -y
    
    # Install dependencies
    sudo apt install -y build-essential pkg-config libssl-dev
    
    # Setup validator
    sudo mkdir -p /opt/dxid
    sudo cp ~/dxid-node /opt/dxid/
    sudo chmod +x /opt/dxid/dxid-node
    
    # Configure service
    sudo cp ~/deployment/dxid-validator.service /etc/systemd/system/
    sudo sed -i "s/YOUR_VM2_IP/$BOOTSTRAP_PEER/g" /etc/systemd/system/dxid-validator.service
    
    # Start service
    sudo systemctl daemon-reload
    sudo systemctl enable dxid-validator
    sudo systemctl start dxid-validator
    
    echo "Validator deployed successfully!"
    sudo systemctl status dxid-validator
EOF

echo "Deployment complete for $VM_NAME"
```

### Usage
```bash
# Deploy to both VMs
./deploy-validator.sh YOUR_VM1_IP validator-1 YOUR_VM2_IP
./deploy-validator.sh YOUR_VM2_IP validator-2 YOUR_VM1_IP
```

## 📊 Cost Analysis

### Oracle Cloud Free Tier
- **2 ARM VMs**: $0/month (Always Free)
- **200 GB Storage**: $0/month
- **1 Gbps Network**: $0/month
- **Total**: $0/month

### Render/Railway
- **Explorer**: $7/month (or free tier)
- **Faucet**: $7/month (or free tier)
- **API Gateway**: $7/month (or free tier)
- **Total**: $21/month (or $0 with free tiers)

### Total Network Cost
- **Validators**: $0/month
- **Auxiliary Services**: $21/month (or $0)
- **Total**: $21/month for production-ready network

## 🔒 Security Considerations

### Oracle Cloud Security
```bash
# Firewall configuration
sudo ufw default deny incoming
sudo ufw default allow outgoing
sudo ufw allow ssh
sudo ufw allow 7000/tcp
sudo ufw allow 7001/udp
sudo ufw allow 8545/tcp
sudo ufw enable

# SSL/TLS certificates
sudo apt install certbot
sudo certbot --nginx -d your-explorer-domain.com
```

### API Security
```javascript
// Rate limiting
const rateLimit = require('express-rate-limit');

const limiter = rateLimit({
    windowMs: 15 * 60 * 1000, // 15 minutes
    max: 100 // limit each IP to 100 requests per windowMs
});

app.use(limiter);
```

## 📈 Scaling Strategy

### Phase 1: Foundation (Current)
- 2 Oracle Cloud validators
- Basic auxiliary services

### Phase 2: Growth
- Add more validators as needed
- Implement sharding
- Add more auxiliary services

### Phase 3: Enterprise
- Multi-region deployment
- Advanced monitoring
- Automated failover

## 🎯 Benefits of This Setup

### Cost Efficiency
- **$0 validators** (Oracle Cloud Free Tier)
- **Minimal auxiliary costs** (Render/Railway free tiers)
- **Always-on infrastructure** without recurring costs

### Reliability
- **Always-free Oracle Cloud** never expires
- **Redundant validators** for high availability
- **Geographic distribution** for resilience

### Scalability
- **Easy to add more validators**
- **Modular auxiliary services**
- **Cloud-native architecture**

---

**This deployment strategy gives you a production-ready dxID Layer0 network for minimal cost!** 🚀✨
