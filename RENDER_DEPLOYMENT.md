# Render.com Deployment Guide

## Quick Setup

1. **Connect Repository**
   - Go to [render.com](https://render.com)
   - Sign up/Login with GitHub
   - Click "New +" → "Web Service"
   - Connect your GitHub repository: `datadistributor/dxid-l0`

2. **Configure Service**
   - **Name**: `dxid-layer0`
   - **Environment**: `Rust`
   - **Build Command**: `cargo build --release --bin dxid-node`
   - **Start Command**: `./target/release/dxid-node`
   - **Health Check Path**: `/health`

3. **Environment Variables**
   - `RUST_VERSION`: `1.82.0`
   - `PORT`: `8080`

4. **Deploy**
   - Click "Create Web Service"
   - Render will automatically build and deploy

## Alternative: Use render.yaml

If you prefer, you can use the included `render.yaml` file:

1. Push the `render.yaml` file to your repository
2. In Render dashboard, choose "Blueprint" instead of "Web Service"
3. Select your repository
4. Render will automatically configure everything from the YAML

## Benefits of Render over Railway

- ✅ More reliable deployment
- ✅ Better build logs and debugging
- ✅ Automatic HTTPS
- ✅ Custom domains
- ✅ Better monitoring
- ✅ Free tier available

## Testing After Deployment

Once deployed, test the endpoints:

```bash
# Health check
curl https://your-app-name.onrender.com/health

# Status
curl https://your-app-name.onrender.com/status

# Balance (with API key)
curl -H "X-Api-Key: your-api-key" https://your-app-name.onrender.com/balance/your-address
```

## Update CLI Configuration

After deployment, update `dxid-config.toml`:

```toml
rpc = "https://your-app-name.onrender.com"
```

## Troubleshooting

- Check build logs in Render dashboard
- Verify all dependencies are in Cargo.toml
- Ensure PORT environment variable is set
- Check health check endpoint is working
