//! Zcash Key Generation and Wallet Utility
//!
//! Commands:
//! - generate: Create new spending key, viewing key, and unified address
//! - info: Check lightwalletd connection and chain status
//! - address: Show address from FVK
//! - export: Convert hex FVK to UFVK format for wallets

use anyhow::Result;
use clap::{Parser, Subcommand};
use orchard::keys::{FullViewingKey, IncomingViewingKey, Scope, SpendingKey};
use rand::RngCore;
use tonic::transport::Channel;
use zcash_address::unified::{self, Container, Encoding, Fvk, Ufvk};
use zcash_protocol::consensus::NetworkType;

// Include the generated protobuf code
pub mod proto {
    tonic::include_proto!("cash.z.wallet.sdk.rpc");
}

use proto::compact_tx_streamer_client::CompactTxStreamerClient;
use proto::{ChainSpec, Empty};

#[derive(Parser)]
#[command(name = "zcash-keygen")]
#[command(about = "Zcash testnet key generation and wallet utility")]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    /// Generate new spending key, viewing key, and unified address
    Generate,

    /// Check lightwalletd connection and chain status
    Info {
        /// Lightwalletd server URL
        #[arg(short, long, default_value = "https://lightwalletd.testnet.electriccoin.co:9067")]
        server: String,
    },

    /// Show address from a Full Viewing Key
    Address {
        /// Orchard Full Viewing Key (hex encoded, 96 bytes)
        #[arg(short, long)]
        fvk: String,
    },

    /// Export FVK to UFVK format (for importing into wallets like Zingo/YWallet)
    Export {
        /// Orchard Full Viewing Key (hex encoded, 96 bytes)
        #[arg(short, long)]
        fvk: String,
    },

    /// Decode a UFVK back to its component keys
    Decode {
        /// Unified Full Viewing Key (uviewtest1... or uview1...)
        ufvk: String,
    },
}

#[tokio::main]
async fn main() -> Result<()> {
    let cli = Cli::parse();

    match cli.command {
        Commands::Generate => generate_keys()?,
        Commands::Info { server } => info(&server).await?,
        Commands::Address { fvk } => show_address(&fvk)?,
        Commands::Export { fvk } => export_ufvk(&fvk)?,
        Commands::Decode { ufvk } => decode_ufvk(&ufvk)?,
    }

    Ok(())
}

fn generate_keys() -> Result<()> {
    println!("=== Zcash Testnet Key Generator ===\n");

    // Generate random 32 bytes for spending key
    let mut sk_bytes = [0u8; 32];
    rand::thread_rng().fill_bytes(&mut sk_bytes);

    // Create spending key from random bytes
    let sk = SpendingKey::from_bytes(sk_bytes);
    if sk.is_none().into() {
        anyhow::bail!("Failed to generate valid spending key, please try again");
    }
    let sk = sk.unwrap();

    // Derive Full Viewing Key
    let fvk = FullViewingKey::from(&sk);
    let fvk_bytes = fvk.to_bytes();

    // Derive Incoming Viewing Key (for receiving)
    let ivk: IncomingViewingKey = fvk.to_ivk(Scope::External);

    // Generate a default address
    let orchard_address = fvk.address_at(0u64, Scope::External);
    let orchard_raw = orchard_address.to_raw_address_bytes();

    // Create Unified Address with just Orchard receiver
    let ua = unified::Address::try_from_items(vec![unified::Receiver::Orchard(orchard_raw)])
        .map_err(|e| anyhow::anyhow!("Failed to create unified address: {}", e))?;

    // Encode as testnet address
    let ua_encoded = ua.encode(&NetworkType::Test);

    println!("SPENDING KEY (keep secret!):");
    println!("  {}", hex::encode(sk_bytes));
    println!();

    println!("FULL VIEWING KEY (for zcash-backend .env):");
    println!("  {}", hex::encode(fvk_bytes));
    println!();

    println!("INCOMING VIEWING KEY:");
    println!("  {}", hex::encode(ivk.to_bytes()));
    println!();

    println!("UNIFIED ADDRESS (testnet):");
    println!("  {}", ua_encoded);
    println!();

    println!("=== Copy to .env file ===");
    println!("ORCHARD_FVK={}", hex::encode(fvk_bytes));
    println!("PAYMENT_ADDRESS={}", ua_encoded);

    Ok(())
}

async fn info(server: &str) -> Result<()> {
    println!("=== Lightwalletd Info ===\n");
    println!("Connecting to {}...", server);

    // For HTTPS, tonic should auto-negotiate TLS
    let channel = Channel::from_shared(server.to_string())?
        .connect()
        .await?;

    let mut client = CompactTxStreamerClient::new(channel);

    // Get server info
    let info = client.get_lightd_info(Empty {}).await?.into_inner();

    println!();
    println!("Server Version:    {}", info.version);
    println!("Chain:             {}", info.chain_name);
    println!("Block Height:      {}", info.block_height);
    println!("Sapling Activated: {}", info.sapling_activation_height);
    println!("Consensus Branch:  {}", info.consensus_branch_id);

    // Get latest block
    let latest = client.get_latest_block(ChainSpec {}).await?.into_inner();
    println!();
    println!("Latest Block:");
    println!("  Height: {}", latest.height);
    println!("  Hash:   {}", hex::encode(&latest.hash));

    Ok(())
}

fn show_address(fvk_hex: &str) -> Result<()> {
    println!("=== Address from FVK ===\n");

    let fvk = parse_orchard_fvk(fvk_hex)?;

    // Get the address
    let address = fvk.address_at(0u64, Scope::External);
    let orchard_raw = address.to_raw_address_bytes();
    let ua = unified::Address::try_from_items(vec![unified::Receiver::Orchard(orchard_raw)])?;
    let ua_encoded = ua.encode(&NetworkType::Test);

    println!("Unified Address (testnet):");
    println!("  {}", ua_encoded);
    println!();
    println!("To check balance, use a wallet like Zingo or YWallet,");
    println!("or view on https://testnet.zcashblockexplorer.com/");

    Ok(())
}

fn export_ufvk(fvk_hex: &str) -> Result<()> {
    println!("=== Export to UFVK Format ===\n");

    let orchard_fvk = parse_orchard_fvk(fvk_hex)?;

    // Get the FVK bytes for encoding
    let fvk_bytes = orchard_fvk.to_bytes();

    // Create UFVK with just Orchard receiver
    let ufvk = Ufvk::try_from_items(vec![Fvk::Orchard(fvk_bytes)])
        .map_err(|e| anyhow::anyhow!("Failed to create UFVK: {}", e))?;

    // Encode for testnet
    let ufvk_encoded = ufvk.encode(&NetworkType::Test);

    // Also show the address
    let address = orchard_fvk.address_at(0u64, Scope::External);
    let orchard_raw = address.to_raw_address_bytes();
    let ua = unified::Address::try_from_items(vec![unified::Receiver::Orchard(orchard_raw)])?;
    let ua_encoded = ua.encode(&NetworkType::Test);

    println!("UNIFIED FULL VIEWING KEY (UFVK):");
    println!("  {}", ufvk_encoded);
    println!();
    println!("UNIFIED ADDRESS:");
    println!("  {}", ua_encoded);
    println!();
    println!("=== Import Instructions ===");
    println!();
    println!("Zingo CLI:");
    println!("  ./zingo-cli --server <lightwalletd> importufvk \"{}\"", ufvk_encoded);
    println!();
    println!("YWallet:");
    println!("  Settings > Accounts > Import > Paste the UFVK above");

    Ok(())
}

/// Parse hex-encoded Orchard FVK
fn parse_orchard_fvk(fvk_hex: &str) -> Result<FullViewingKey> {
    let fvk_bytes = hex::decode(fvk_hex)?;
    if fvk_bytes.len() != 96 {
        anyhow::bail!("FVK must be 96 bytes, got {}", fvk_bytes.len());
    }
    let mut fvk_arr = [0u8; 96];
    fvk_arr.copy_from_slice(&fvk_bytes);

    let fvk = FullViewingKey::from_bytes(&fvk_arr);
    if fvk.is_none().into() {
        anyhow::bail!("Invalid Full Viewing Key");
    }
    Ok(fvk.unwrap())
}

fn decode_ufvk(ufvk_str: &str) -> Result<()> {
    println!("=== Decode UFVK ===\n");

    // Detect network from prefix
    let (ufvk, network) = if ufvk_str.starts_with("uviewtest") {
        let (net, ufvk) = Ufvk::decode(ufvk_str)
            .map_err(|e| anyhow::anyhow!("Failed to decode UFVK: {:?}", e))?;
        (ufvk, format!("{:?}", net))
    } else if ufvk_str.starts_with("uview") {
        let (net, ufvk) = Ufvk::decode(ufvk_str)
            .map_err(|e| anyhow::anyhow!("Failed to decode UFVK: {:?}", e))?;
        (ufvk, format!("{:?}", net))
    } else {
        anyhow::bail!("Invalid UFVK: must start with 'uview' or 'uviewtest'");
    };

    println!("Network: {}", network);
    println!();

    // Extract components
    let items = ufvk.items();
    println!("Components ({}):", items.len());

    for item in items {
        match item {
            Fvk::Orchard(bytes) => {
                println!();
                println!("  ORCHARD FVK (96 bytes):");
                println!("    {}", hex::encode(bytes));

                // Try to derive address from it
                let fvk_opt: Option<FullViewingKey> = FullViewingKey::from_bytes(&bytes).into();
                if let Some(fvk) = fvk_opt {
                    let address = fvk.address_at(0u64, Scope::External);
                    let orchard_raw = address.to_raw_address_bytes();
                    let ua = unified::Address::try_from_items(vec![unified::Receiver::Orchard(orchard_raw)]);
                    if let Ok(ua) = ua {
                        let ua_encoded = ua.encode(&NetworkType::Test);
                        println!();
                        println!("  Derived Address:");
                        println!("    {}", ua_encoded);
                    }
                }
            }
            Fvk::Sapling(bytes) => {
                println!();
                println!("  SAPLING FVK (128 bytes):");
                println!("    {}", hex::encode(bytes));
            }
            Fvk::P2pkh(bytes) => {
                println!();
                println!("  TRANSPARENT P2PKH (65 bytes):");
                println!("    {}", hex::encode(bytes));
            }
            Fvk::Unknown { typecode, data } => {
                println!();
                println!("  UNKNOWN (typecode {}):", typecode);
                println!("    {}", hex::encode(data));
            }
        }
    }

    println!();
    println!("=== For .env file ===");
    for item in ufvk.items() {
        if let Fvk::Orchard(bytes) = item {
            println!("ORCHARD_FVK={}", hex::encode(bytes));
        }
    }

    Ok(())
}
