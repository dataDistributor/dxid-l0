# dxID Layer0 Quick Setup Guide 🚀

## 🎯 **Perfect Deployment Strategy**

Your dxID Layer0 network is now configured for **optimal cost and reliability**:

- **Validators**: Oracle Cloud Free Tier (Always-on, $0/month)
- **Auxiliary Services**: Render/Railway (Cost-effective)

## 🟠 **Step 1: Oracle Cloud Free Tier Setup**

### Create Oracle Cloud Account
1. Go to [Oracle Cloud Free Tier](https://www.oracle.com/cloud/free/)
2. Sign up for **Always Free** account
3. Get **2 ARM VMs** (4 cores, 24GB RAM each) - **$0/month forever**

### Create VMs
```bash
# VM 1: Primary Validator
Name: dxid-validator-1
Shape: VM.Standard.A1.Flex
CPU: 4 cores
RAM: 24 GB
OS: Ubuntu 22.04
Public IP: Yes

# VM 2: Secondary Validator  
Name: dxid-validator-2
Shape: VM.Standard.A1.Flex
CPU: 4 cores
RAM: 24 GB
OS: Ubuntu 22.04
Public IP: Yes
```

### Configure Security Lists
Open these ports on both VMs:
- **22**: SSH
- **7000**: P2P Communication
- **7001**: P2P Discovery (UDP)
- **8545**: RPC API

## 🚀 **Step 2: Deploy Validators**

### Build Release Binary
```bash
# On your local machine
cargo build --release
```

### Deploy to Oracle Cloud VMs
```bash
# Deploy to first validator
./deployment/deploy-validator.sh YOUR_VM1_IP validator-1 YOUR_VM2_IP

# Deploy to second validator
./deployment/deploy-validator.sh YOUR_VM2_IP validator-2 YOUR_VM1_IP
```

### Verify Deployment
```bash
# Check validator 1
ssh ubuntu@YOUR_VM1_IP "sudo systemctl status dxid-validator"

# Check validator 2
ssh ubuntu@YOUR_VM2_IP "sudo systemctl status dxid-validator"

# Test RPC endpoints
curl http://YOUR_VM1_IP:8545/status
curl http://YOUR_VM2_IP:8545/status
```

## 🔵 **Step 3: Deploy Auxiliary Services**

### Blockchain Explorer (Render)
1. Go to [Render.com](https://render.com)
2. Connect your GitHub repository
3. Deploy from `auxiliary-services/explorer/`
4. Set environment variables:
   - `VALIDATOR_RPC`: `http://YOUR_VM1_IP:8545`
   - `BACKUP_RPC`: `http://YOUR_VM2_IP:8545`

### Faucet Service (Railway)
1. Go to [Railway.app](https://railway.app)
2. Connect your GitHub repository
3. Deploy from `auxiliary-services/faucet/`
4. Set environment variables:
   - `VALIDATOR_RPC`: `http://YOUR_VM1_IP:8545`
   - `BACKUP_RPC`: `http://YOUR_VM2_IP:8545`

## 📊 **Cost Breakdown**

### Oracle Cloud Free Tier
- **2 ARM VMs**: $0/month (Always Free)
- **200 GB Storage**: $0/month
- **1 Gbps Network**: $0/month
- **Total**: $0/month

### Render/Railway
- **Explorer**: $7/month (or free tier)
- **Faucet**: $7/month (or free tier)
- **Total**: $14/month (or $0 with free tiers)

### **Total Network Cost**: $14/month (or $0) 🎉

## 🌐 **Network Architecture**

```
┌─────────────────┐    ┌─────────────────┐    ┌─────────────────┐
│   Oracle Cloud  │    │   Oracle Cloud  │    │   Render/Railway│
│   Free Tier     │    │   Free Tier     │    │   (Auxiliary)   │
│                 │    │                 │    │                 │
│ ┌─────────────┐ │    │ ┌─────────────┐ │    │ ┌─────────────┐ │
│ │ Validator 1 │ │◄──►│ │ Validator 2 │ │◄──►│ │   Explorer  │ │
│ │ Bootstrap   │ │    │ │ Bootstrap   │ │    │ │   Faucet    │ │
│ │ Always-on   │ │    │ │ Always-on   │ │    │ │             │ │
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

## ✅ **Benefits of This Setup**

### **Cost Efficiency**
- ✅ **$0 validators** (Oracle Cloud Free Tier)
- ✅ **Minimal auxiliary costs** (Render/Railway)
- ✅ **Always-on infrastructure** without recurring costs

### **Reliability**
- ✅ **Always-free Oracle Cloud** never expires
- ✅ **Redundant validators** for high availability
- ✅ **Geographic distribution** for resilience

### **Scalability**
- ✅ **Easy to add more validators**
- ✅ **Modular auxiliary services**
- ✅ **Cloud-native architecture**

## 🎯 **Next Steps**

1. **Deploy validators** to Oracle Cloud
2. **Deploy auxiliary services** to Render/Railway
3. **Test network connectivity**
4. **Share your GitHub repository**
5. **Let the community join!**

## 📞 **Support**

- **Oracle Cloud**: [Documentation](https://docs.oracle.com/en-us/iaas/Content/home.htm)
- **Render**: [Documentation](https://render.com/docs)
- **Railway**: [Documentation](https://docs.railway.app)

---

**Your dxID Layer0 network is now ready for production deployment!** 🚀✨

**Total cost: $14/month (or $0 with free tiers) for a production-ready blockchain network!** 💰
