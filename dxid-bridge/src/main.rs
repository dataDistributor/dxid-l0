use clap::Parser;
use dxid_bridge::{dxIDBridge, BridgeConfig};
use std::env;

#[derive(Parser)]
#[command(name = "dxid-bridge")]
#[command(about = "dxID Bridge - Connect Layer0 to Layer1 Blockchains")]
struct Cli {
    /// Network to connect to
    #[arg(short, long, value_name = "NETWORK")]
    network: Option<String>,

    /// Bridge tokens between networks
    #[arg(short, long)]
    bridge: bool,

    /// Source network
    #[arg(long)]
    from: Option<String>,

    /// Destination network
    #[arg(long)]
    to: Option<String>,

    /// Amount to bridge
    #[arg(long)]
    amount: Option<u128>,

    /// Token symbol
    #[arg(long)]
    token: Option<String>,

    /// List connected networks
    #[arg(long)]
    list: bool,

    /// Check bridge status
    #[arg(long)]
    status: Option<String>,
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    env_logger::init();

    let cli = Cli::parse();

    // Load bridge configuration
    let config = BridgeConfig::default();
    let mut bridge = dxIDBridge::new(config).await?;

    match cli {
        Cli { network: Some(network), .. } => {
            println!("Connecting to {} network...", network);
            bridge.connect_to_network(&network).await?;
            println!("Successfully connected to {} network", network);
        },
        Cli { list: true, .. } => {
            let networks = bridge.get_connected_networks();
            if networks.is_empty() {
                println!("No networks connected");
            } else {
                println!("Connected networks:");
                for network in networks {
                    println!("  - {}", network);
                }
            }
        },
        Cli { bridge: true, from: Some(from), to: Some(to), amount: Some(amount), token: Some(token), .. } => {
            println!("Bridging {} {} from {} to {}", amount, token, from, to);
            
            // For demo purposes, use placeholder addresses
            let from_address = "0x742d35Cc6634C0532925a3b8D4C9db96C4b4d8b6";
            let to_address = "0x742d35Cc6634C0532925a3b8D4C9db96C4b4d8b6";
            
            let tx_id = bridge.bridge_tokens(&from, &to, from_address, to_address, amount, &token).await?;
            println!("Bridge transaction created: {}", tx_id);
        },
        Cli { status: Some(tx_id), .. } => {
            if let Some(tx) = bridge.get_bridge_status(&tx_id).await? {
                println!("Bridge transaction status:");
                println!("  ID: {}", tx.id);
                println!("  From: {} ({})", tx.from_network, tx.from_address);
                println!("  To: {} ({})", tx.to_network, tx.to_address);
                println!("  Amount: {} {}", tx.amount, tx.token_symbol);
                println!("  Status: {:?}", tx.status);
                println!("  Fee: {}", tx.fee);
                if let Some(completed_at) = tx.completed_at {
                    println!("  Completed at: {}", completed_at);
                }
            } else {
                println!("Bridge transaction not found: {}", tx_id);
            }
        },
        _ => {
            println!("dxID Bridge - Connect Layer0 to Layer1 Blockchains");
            println!();
            println!("Usage:");
            println!("  dxid-bridge --network <NETWORK>     Connect to a network");
            println!("  dxid-bridge --list                  List connected networks");
            println!("  dxid-bridge --bridge --from <FROM> --to <TO> --amount <AMOUNT> --token <TOKEN>");
            println!("                                      Bridge tokens between networks");
            println!("  dxid-bridge --status <TX_ID>       Check bridge transaction status");
            println!();
            println!("Supported networks:");
            println!("  - ethereum");
            println!("  - bitcoin");
            println!("  - bsc (Binance Smart Chain)");
            println!("  - polygon");
            println!("  - solana");
            println!("  - cardano");
            println!("  - polkadot");
            println!("  - cosmos");
        }
    }

    Ok(())
}
