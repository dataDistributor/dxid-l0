# Oracle Cloud Free Tier Setup Guide 🟠

## Step-by-Step ARM VM Creation for dxID Layer0

### Prerequisites
- **Credit Card Required**: Oracle Cloud requires a credit card for verification (no charges for free tier)
- **Email Address**: For account creation
- **Phone Number**: For verification

---

## Step 1: Create Oracle Cloud Account

### 1.1 Go to Oracle Cloud Free Tier
1. Visit: https://www.oracle.com/cloud/free/
2. Click **"Start for free"** button
3. You'll see: **"Always Free"** services highlighted

### 1.2 Account Information
Fill in the registration form:
- **Email**: Your email address
- **Country**: Select your country
- **Name**: Your full name
- **Cloud Account Name**: Choose a unique name (e.g., `dxid-layer0-network`)

### 1.3 Address Information
- **Address**: Your billing address
- **City, State, ZIP**: Complete address details
- **Phone**: Your phone number

### 1.4 Payment Method
- **Credit Card**: Required for verification
- **Note**: You won't be charged for free tier services
- **Billing Address**: Same as above

### 1.5 Verification
- **Email Verification**: Check your email and click verification link
- **Phone Verification**: You'll receive a text with verification code
- **Account Review**: Oracle will review your account (usually instant)

---

## Step 2: Access Oracle Cloud Console

### 2.1 Sign In
1. Go to: https://cloud.oracle.com/
2. Click **"Sign In"**
3. Enter your email and password
4. Select your **Cloud Account** (the name you chose)

### 2.2 Navigate to Compute
1. In the Oracle Cloud Console, click the **hamburger menu** (☰) in the top left
2. Go to **"Compute"** → **"Instances"**
3. Click **"Create Instance"**

---

## Step 3: Create First VM (Validator 1)

### 3.1 Basic Information
```
Name: dxid-validator-1
Create in compartment: (root) [default]
Placement: Availability Domain 1 [default]
Fault Domain: [auto-assigned]
```

### 3.2 Image and Shape
```
Image: Canonical Ubuntu 22.04
Image build: 2024.01.23-0
Shape: VM.Standard.A1.Flex
```

**⚠️ IMPORTANT**: Make sure you select **VM.Standard.A1.Flex** (ARM-based) for free tier!

### 3.3 Shape Configuration
```
OCPUs: 4
Memory (GB): 24
Network bandwidth: 1 Gbps
```

### 3.4 Networking
```
Virtual cloud network: Create new VCN
Name: dxid-vcn
CIDR block: 10.0.0.0/16
Subnet: Create new subnet
Subnet name: dxid-subnet
Subnet CIDR: 10.0.0.0/24
```

### 3.5 Public IP
- ✅ **Assign a public IPv4 address**: Check this box
- This gives your VM a public IP for external access

### 3.6 Boot Volume
```
Size (GB): 200
Performance: Balanced
Encryption: Use platform-managed encryption key
```

### 3.7 Advanced Options

#### Instance Metadata Service
```
✅ Enable Instance Metadata Service: YES
```

#### Initialization Script
```
❌ Initialization Script: Leave empty (we'll use deploy-validator.sh)
```

#### Security Attributes
```
✅ Security Attributes: Use platform default security policies
```

#### Availability Configuration
```
❌ Live Migration: Disable
❌ Availability Domain: Use default (AD1)
```

#### Oracle Cloud Agents
```
✅ Oracle Cloud Agent: Enable
✅ Management Agent: Enable  
✅ Monitoring Agent: Enable
✅ Logging Agent: Enable
```

### 3.8 Review and Create
1. Review all settings
2. Click **"Create"**
3. Wait for VM to be created (2-3 minutes)

---

## Step 4: Create Second VM (Validator 2)

### 4.1 Repeat Steps 3.1-3.8
- **Name**: `dxid-validator-2`
- **Same settings** as Validator 1
- **Same VCN and subnet** (reuse the one created)

### 4.2 Note the IP Addresses
After both VMs are created, note their **public IP addresses**:
- **Validator 1 IP**: `xxx.xxx.xxx.xxx`
- **Validator 2 IP**: `yyy.yyy.yyy.yyy`

---

## Step 5: Configure Security Lists (Firewall)

### 5.1 Access Security Lists
1. Go to **"Networking"** → **"Virtual Cloud Networks"**
2. Click on your **dxid-vcn**
3. Click **"Security Lists"** in the left menu
4. Click on the **default security list**

### 5.2 Add Inbound Rules
Click **"Add Ingress Rules"** and add these rules:

#### Rule 1: SSH Access
```
Source Type: CIDR
Source CIDR: 0.0.0.0/0
IP Protocol: TCP
Source Port Range: All
Destination Port Range: 22
Description: SSH access
```

#### Rule 2: P2P Communication
```
Source Type: CIDR
Source CIDR: 0.0.0.0/0
IP Protocol: TCP
Source Port Range: All
Destination Port Range: 7000
Description: P2P communication
```

#### Rule 3: P2P Discovery
```
Source Type: CIDR
Source CIDR: 0.0.0.0/0
IP Protocol: UDP
Source Port Range: All
Destination Port Range: 7001
Description: P2P discovery
```

#### Rule 4: RPC API
```
Source Type: CIDR
Source CIDR: 0.0.0.0/0
IP Protocol: TCP
Source Port Range: All
Destination Port Range: 8545
Description: RPC API
```

#### Rule 5: HTTP (Optional - for Explorer)
```
Source Type: CIDR
Source CIDR: 0.0.0.0/0
IP Protocol: TCP
Source Port Range: All
Destination Port Range: 80
Description: HTTP
```

#### Rule 6: HTTPS (Optional - for Explorer)
```
Source Type: CIDR
Source CIDR: 0.0.0.0/0
IP Protocol: TCP
Source Port Range: All
Destination Port Range: 443
Description: HTTPS
```

### 5.3 Save Rules
Click **"Add Ingress Rules"** to save all rules.

---

## Step 6: Connect to Your VMs

### 6.1 Get SSH Key
1. In Oracle Cloud Console, go to **"Compute"** → **"Instances"**
2. Click on your VM name
3. Click **"Connect"** button
4. Choose **"Cloud Shell"** (easiest method)
5. Copy the SSH command provided

### 6.2 Alternative: Use Your Local Terminal
If you prefer using your local terminal:
1. Generate SSH key: `ssh-keygen -t rsa -b 2048`
2. Copy public key to VM (Oracle will show you how)

### 6.3 Test Connection
```bash
# Connect to Validator 1
ssh ubuntu@YOUR_VM1_IP

# Connect to Validator 2  
ssh ubuntu@YOUR_VM2_IP
```

---

## Step 7: Verify VM Setup

### 7.1 Check System Resources
```bash
# Check CPU and RAM
free -h
nproc

# Check disk space
df -h

# Check network
ip addr show
```

### 7.2 Update System
```bash
sudo apt update && sudo apt upgrade -y
```

### 7.3 Install Basic Tools
```bash
sudo apt install -y curl wget git htop
```

---

## Step 8: Prepare for dxID Deployment

### 8.1 Install Rust
```bash
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh
source ~/.cargo/env
rustc --version
```

### 8.2 Install Build Dependencies
```bash
sudo apt install -y build-essential pkg-config libssl-dev
```

### 8.3 Test Network Connectivity
```bash
# Test internet access
ping -c 3 google.com

# Test port accessibility (from your local machine)
telnet YOUR_VM_IP 22
telnet YOUR_VM_IP 7000
telnet YOUR_VM_IP 8545
```

---

## Step 9: Record Your Configuration

Create a file to record your setup:

```bash
# On your local machine
cat > oracle-cloud-config.txt << EOF
Oracle Cloud Free Tier Configuration
====================================

Validator 1:
- Name: dxid-validator-1
- Public IP: YOUR_VM1_IP
- Private IP: 10.0.0.x
- Shape: VM.Standard.A1.Flex (4 OCPUs, 24GB RAM)

Validator 2:
- Name: dxid-validator-2  
- Public IP: YOUR_VM2_IP
- Private IP: 10.0.0.y
- Shape: VM.Standard.A1.Flex (4 OCPUs, 24GB RAM)

Network:
- VCN: dxid-vcn (10.0.0.0/16)
- Subnet: dxid-subnet (10.0.0.0/24)
- Security List: All required ports open

SSH Commands:
ssh ubuntu@YOUR_VM1_IP
ssh ubuntu@YOUR_VM2_IP

Next Steps:
1. Deploy dxID using deploy-validator.sh script
2. Configure bootstrap peers
3. Deploy auxiliary services
EOF
```

---

## Step 10: Verify Free Tier Status

### 10.1 Check Usage
1. In Oracle Cloud Console, go to **"Billing"** → **"Usage"**
2. Verify you're using **Always Free** resources
3. Check that you haven't exceeded free tier limits

### 10.2 Free Tier Limits
- **2 ARM-based VMs** (VM.Standard.A1.Flex)
- **24 GB total memory**
- **200 GB total storage**
- **Always Free** - never expires

---

## ✅ Success Checklist

- [ ] Oracle Cloud account created and verified
- [ ] 2 ARM VMs created (VM.Standard.A1.Flex)
- [ ] Public IPs assigned to both VMs
- [ ] Security list configured with all required ports
- [ ] SSH access working to both VMs
- [ ] System updated and Rust installed
- [ ] Network connectivity verified
- [ ] Configuration documented

---

## 🚀 Next Steps

Once your Oracle Cloud VMs are set up:

1. **Build dxID binary** on your local machine
2. **Deploy using the script**: `./deployment/deploy-validator.sh`
3. **Configure bootstrap peers** between validators
4. **Deploy auxiliary services** on Render/Railway
5. **Test network connectivity**

---

## 💰 Cost Verification

**Total Cost: $0/month**
- Oracle Cloud Free Tier: Always free
- 2 ARM VMs: Included in free tier
- 200 GB storage: Included in free tier
- Network bandwidth: Included in free tier

**No charges will be applied** as long as you stay within free tier limits!

---

**Your Oracle Cloud infrastructure is now ready for dxID Layer0 deployment!** 🚀
