# Railway Free Deployment Guide 🚂

## Deploy dxID Layer0 Node on Railway (FREE)

### ✅ **Why Railway is Perfect:**
- **Free tier available** - 500 hours/month
- **Simple deployment** - just connect your GitHub repo
- **Automatic builds** - no manual setup needed
- **Public URLs** - automatically discoverable
- **No credit card required** for free tier

---

## Step 1: Prepare Your Repository

### 1.1 Push to GitHub
```bash
# Make sure your code is on GitHub
git add .
git commit -m "Ready for Railway deployment"
git push origin main
```

### 1.2 Create Railway Configuration
Create a new file: `railway.toml` in your project root:

```toml
[build]
builder = "nixpacks"

[deploy]
startCommand = "cargo run --bin dxid-node -- --p2p-listen 0.0.0.0:7000 --rpc-listen 0.0.0.0:8545"
healthcheckPath = "/status"
healthcheckTimeout = 300
restartPolicyType = "on_failure"
restartPolicyMaxRetries = 10

[[services]]
name = "dxid-node"
```

---

## Step 2: Create Railway Account

### 2.1 Sign Up
1. **Go to**: https://railway.app/
2. **Click "Start a New Project"**
3. **Sign up with GitHub** (recommended)
4. **Verify your email**

### 2.2 Free Tier Setup
```
✅ Select "Free" plan
✅ 500 hours/month included
✅ No credit card required
```

---

## Step 3: Deploy Your Node

### 3.1 Connect Repository
1. **Click "Deploy from GitHub repo"**
2. **Select your dxID repository**
3. **Choose the main branch**

### 3.2 Configure Build Settings
```
✅ Builder: Nixpacks (automatic)
✅ Root Directory: / (leave empty)
✅ Branch: main
```

### 3.3 Environment Variables
Add these environment variables in Railway dashboard:

```bash
# Node Configuration
NODE_ENV=production
RUST_VERSION=1.70.0

# Network Configuration  
P2P_LISTEN_ADDR=0.0.0.0:7000
RPC_LISTEN_ADDR=0.0.0.0:8545

# Optional: Custom settings
LOG_LEVEL=info
```

---

## Step 4: Deploy and Configure

### 4.1 Start Deployment
1. **Click "Deploy Now"**
2. **Wait for build** (5-10 minutes)
3. **Check build logs** for any errors

### 4.2 Get Your Public URL
After deployment, Railway will give you:
```
✅ Public URL: https://your-app-name.railway.app
✅ Internal URL: http://localhost:8545
```

### 4.3 Test Your Node
```bash
# Test RPC endpoint
curl https://your-app-name.railway.app/status

# Test P2P endpoint  
curl https://your-app-name.railway.app:7000
```

---

## Step 5: Configure Your Local Node

### 5.1 Update Your Local Configuration
Edit your `dxid-config.toml`:

```toml
# Add your Railway node as a bootstrap peer
rpc = "http://127.0.0.1:8545"
bootstrap_peers = ["your-app-name.railway.app:7000"]
```

### 5.2 Connect Local to Railway
Your local node will now:
1. **Connect to Railway node** automatically
2. **Share the network** with Railway
3. **Be discoverable** through Railway

---

## Step 6: Monitor and Manage

### 6.1 Railway Dashboard
```
✅ View logs in real-time
✅ Monitor resource usage
✅ Restart if needed
✅ Scale if required
```

### 6.2 Health Checks
Railway will automatically:
- **Check `/status` endpoint**
- **Restart on failures**
- **Monitor uptime**

---

## Troubleshooting

### ❌ Build Fails
```bash
# Check Railway logs
# Common issues:
# 1. Missing Cargo.toml
# 2. Rust version issues
# 3. Dependencies not found
```

### ❌ Node Won't Start
```bash
# Check environment variables
# Verify ports are correct
# Check Railway logs for errors
```

### ❌ Connection Issues
```bash
# Verify public URL is correct
# Check firewall settings
# Test with curl first
```

---

## Cost Breakdown

### 💰 **Free Tier Limits:**
```
✅ 500 hours/month (FREE)
✅ 1GB RAM
✅ Shared CPU
✅ 1GB storage
✅ Perfect for dxID node
```

### 💰 **If You Exceed Free Tier:**
```
⚠️ $5/month for additional usage
⚠️ $0.000463 per GB-hour
⚠️ Very affordable scaling
```

---

## Next Steps

### 🎯 **After Deployment:**
1. **Test your Railway node** is working
2. **Connect your local node** to Railway
3. **Share your Railway URL** with others
4. **Monitor performance** in Railway dashboard

### 🚀 **Your Network is Now:**
```
✅ Publicly discoverable
✅ Always online (Railway)
✅ Free to run
✅ Scalable if needed
```

**Your dxID Layer0 network is now live on Railway!** 🎉
