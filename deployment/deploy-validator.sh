#!/bin/bash
# deploy-validator.sh - Automated dxID Layer0 validator deployment for Oracle Cloud

set -e

# Colors for output
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
NC='\033[0m' # No Color

# Function to print colored output
print_status() {
    echo -e "${BLUE}[INFO]${NC} $1"
}

print_success() {
    echo -e "${GREEN}[SUCCESS]${NC} $1"
}

print_warning() {
    echo -e "${YELLOW}[WARNING]${NC} $1"
}

print_error() {
    echo -e "${RED}[ERROR]${NC} $1"
}

# Check arguments
VM_IP=$1
VM_NAME=$2
BOOTSTRAP_PEER=$3

if [ -z "$VM_IP" ] || [ -z "$VM_NAME" ]; then
    echo "Usage: $0 <VM_IP> <VM_NAME> [BOOTSTRAP_PEER]"
    echo ""
    echo "Example:"
    echo "  $0 192.168.1.100 validator-1 192.168.1.101"
    echo "  $0 192.168.1.101 validator-2 192.168.1.100"
    exit 1
fi

print_status "Deploying dxID Layer0 validator to $VM_NAME ($VM_IP)..."

# Check if target/release/dxid-node exists
if [ ! -f "target/release/dxid-node" ]; then
    print_error "dxid-node binary not found. Please run 'cargo build --release' first."
    exit 1
fi

# Check if deployment directory exists
if [ ! -d "deployment" ]; then
    print_error "deployment directory not found. Please ensure deployment files are present."
    exit 1
fi

# Test SSH connection
print_status "Testing SSH connection to $VM_IP..."
if ! ssh -o ConnectTimeout=10 -o BatchMode=yes ubuntu@$VM_IP exit 2>/dev/null; then
    print_error "Cannot connect to $VM_IP via SSH. Please ensure:"
    print_error "  1. SSH key is configured"
    print_error "  2. VM is running"
    print_error "  3. Security lists allow SSH (port 22)"
    exit 1
fi

print_success "SSH connection successful"

# Create deployment package
print_status "Creating deployment package..."
DEPLOY_DIR="/tmp/dxid-deploy-$$"
mkdir -p $DEPLOY_DIR
cp target/release/dxid-node $DEPLOY_DIR/
cp -r deployment/* $DEPLOY_DIR/
cp dxid-config.toml $DEPLOY_DIR/

# Copy deployment files
print_status "Copying deployment files to $VM_IP..."
scp -r $DEPLOY_DIR/* ubuntu@$VM_IP:~/

# Clean up local deployment directory
rm -rf $DEPLOY_DIR

# Execute deployment
print_status "Executing deployment on $VM_IP..."
ssh ubuntu@$VM_IP << EOF
    set -e
    
    echo "=== Starting dxID Layer0 validator deployment ==="
    
    # Update system
    echo "Updating system packages..."
    sudo apt update && sudo apt upgrade -y
    
    # Install dependencies
    echo "Installing build dependencies..."
    sudo apt install -y build-essential pkg-config libssl-dev curl
    
    # Install Rust if not present
    if ! command -v cargo &> /dev/null; then
        echo "Installing Rust..."
        curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh -s -- -y
        source ~/.cargo/env
    fi
    
    # Create dxID directory structure
    echo "Setting up directory structure..."
    mkdir -p ~/dxid-layer0/data
    mkdir -p ~/dxid-layer0/logs
    
    # Move files to proper location
    mv ~/dxid-node ~/dxid-layer0/target/release/
    mv ~/dxid-validator.service ~/dxid-layer0/
    mv ~/dxid-config.toml ~/dxid-layer0/
    chmod +x ~/dxid-layer0/target/release/dxid-node
    
    # Configure bootstrap peer if provided
    if [ ! -z "$BOOTSTRAP_PEER" ]; then
        echo "Configuring bootstrap peer: $BOOTSTRAP_PEER"
        sed -i "s/YOUR_VM2_IP/$BOOTSTRAP_PEER/g" ~/dxid-layer0/dxid-validator.service
    fi
    
    # Setup systemd service
    echo "Configuring systemd service..."
    sudo cp ~/dxid-layer0/dxid-validator.service /etc/systemd/system/
    sudo systemctl daemon-reload
    
    # Configure firewall
    echo "Configuring firewall..."
    sudo ufw --force enable
    sudo ufw default deny incoming
    sudo ufw default allow outgoing
    sudo ufw allow ssh
    sudo ufw allow 7000/tcp
    sudo ufw allow 7001/udp
    sudo ufw allow 8545/tcp
    
    # Create monitoring script
    echo "Setting up monitoring..."
    cat > ~/dxid-layer0/monitor.sh << 'MONITOR_EOF'
#!/bin/bash
echo "=== dxID Layer0 Validator Status ==="
sudo systemctl status dxid-validator --no-pager
echo ""
echo "=== System Resources ==="
free -h
echo ""
echo "=== Disk Usage ==="
df -h
echo ""
echo "=== Network Connections ==="
netstat -tulpn | grep :7000 || echo "No P2P connections"
netstat -tulpn | grep :8545 || echo "No RPC connections"
echo ""
echo "=== Recent Logs ==="
sudo journalctl -u dxid-validator --no-pager -n 20
MONITOR_EOF
    
    chmod +x ~/dxid-layer0/monitor.sh
    
    # Setup log rotation
    echo "Setting up log rotation..."
    sudo tee /etc/logrotate.d/dxid-validator > /dev/null << 'LOGROTATE_EOF'
/home/ubuntu/dxid-layer0/logs/*.log {
    daily
    missingok
    rotate 7
    compress
    delaycompress
    notifempty
    create 644 ubuntu ubuntu
    postrotate
        systemctl reload dxid-validator
    endscript
}
LOGROTATE_EOF
    
    # Start service
    echo "Starting dxID validator service..."
    sudo systemctl enable dxid-validator
    sudo systemctl start dxid-validator
    
    # Wait for service to start
    echo "Waiting for service to start..."
    sleep 10
    
    # Check service status
    echo "Checking service status..."
    if sudo systemctl is-active --quiet dxid-validator; then
        echo "✅ dxID validator service is running"
    else
        echo "❌ dxID validator service failed to start"
        sudo systemctl status dxid-validator --no-pager
        exit 1
    fi
    
    # Test RPC endpoint
    echo "Testing RPC endpoint..."
    if curl -s http://localhost:8545/status > /dev/null; then
        echo "✅ RPC endpoint is responding"
    else
        echo "⚠️  RPC endpoint not responding yet (may need more time)"
    fi
    
    echo "=== Deployment completed successfully ==="
    echo ""
    echo "Useful commands:"
    echo "  Check status: sudo systemctl status dxid-validator"
    echo "  View logs: sudo journalctl -u dxid-validator -f"
    echo "  Monitor: ~/dxid-layer0/monitor.sh"
    echo "  Restart: sudo systemctl restart dxid-validator"
    echo "  Stop: sudo systemctl stop dxid-validator"
EOF

if [ $? -eq 0 ]; then
    print_success "Deployment completed successfully for $VM_NAME!"
    print_status "Validator is now running on $VM_IP"
    print_status "RPC endpoint: http://$VM_IP:8545"
    print_status "P2P endpoint: $VM_IP:7000"
else
    print_error "Deployment failed for $VM_NAME"
    exit 1
fi

# Final status check
print_status "Performing final status check..."
ssh ubuntu@$VM_IP "sudo systemctl status dxid-validator --no-pager"

print_success "dxID Layer0 validator deployment complete!"
print_status "Next steps:"
print_status "  1. Deploy the second validator with this script"
print_status "  2. Configure auxiliary services (explorer, faucet)"
print_status "  3. Test network connectivity between validators"
