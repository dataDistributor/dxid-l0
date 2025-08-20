# Oracle Cloud Quick Reference Card 🟠

## Critical Settings Checklist

### ✅ VM Configuration
```
Shape: VM.Standard.A1.Flex (ARM-based)
OCPUs: 4
Memory: 24 GB
Storage: 200 GB
OS: Ubuntu 22.04
Public IP: ✅ YES
Instance Metadata: ✅ ENABLE
Cloud Agents: ✅ ENABLE (all)
Live Migration: ❌ DISABLE
```

### ✅ Required Ports (Security List)
```
22   - SSH
7000 - P2P Communication (TCP)
7001 - P2P Discovery (UDP)  
8545 - RPC API (TCP)
80   - HTTP (optional)
443  - HTTPS (optional)
```

### ✅ Network Configuration
```
VCN: dxid-vcn (10.0.0.0/16)
Subnet: dxid-subnet (10.0.0.0/24)
Security List: All ports open (0.0.0.0/0)
```

### ✅ VM Names
```
Validator 1: dxid-validator-1
Validator 2: dxid-validator-2
```

---

## Quick Commands

### SSH Connection
```bash
ssh ubuntu@YOUR_VM1_IP
ssh ubuntu@YOUR_VM2_IP
```

### System Setup
```bash
sudo apt update && sudo apt upgrade -y
sudo apt install -y build-essential pkg-config libssl-dev curl wget git
```

### Rust Installation
```bash
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh
source ~/.cargo/env
rustc --version
```

### Network Test
```bash
ping -c 3 google.com
netstat -tulpn | grep :7000
netstat -tulpn | grep :8545
```

---

## Important Notes

### ⚠️ Critical Points
- **Must use VM.Standard.A1.Flex** (ARM) for free tier
- **Credit card required** for verification (no charges)
- **Always Free** - never expires
- **2 VMs maximum** in free tier

### 🔗 Useful Links
- Oracle Cloud Free Tier: https://www.oracle.com/cloud/free/
- Oracle Cloud Console: https://cloud.oracle.com/
- Free Tier Documentation: https://docs.oracle.com/en-us/iaas/Content/FreeTier/freetier_topic-Always_Free_Resources.htm

### 📞 Support
- Oracle Cloud Support: Available in console
- Free Tier FAQ: https://www.oracle.com/cloud/free/faq.html

---

## Cost Breakdown
```
Oracle Cloud Free Tier: $0/month
├── 2 ARM VMs (4 OCPUs, 24GB each): $0
├── 200 GB storage: $0  
├── 1 Gbps network: $0
└── Always Free - never expires
```

**Total: $0/month forever!** 🎉
