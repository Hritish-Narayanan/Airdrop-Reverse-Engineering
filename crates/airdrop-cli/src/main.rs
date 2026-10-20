use airdrop_core::{
    discover_airdrop_peers, register_airdrop_service, run_airdrop_server, AirDropBlePayload,
    AirDropClient, AirDropServerConfig,
};
use clap::{Parser, Subcommand};
use mdns_sd::ServiceDaemon;
use std::net::SocketAddr;
use std::path::PathBuf;
use std::sync::atomic::AtomicBool;
use std::sync::Arc;
use std::time::Duration;
use tracing::error;

#[derive(Parser, Debug)]
#[command(name = "airdrop-cli", version = "0.1.0", about = "AirDrop Sender & Receiver for Android and Linux")]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand, Debug)]
enum Commands {
    /// Scan the local network for AirDrop-enabled Apple devices
    Scan {
        /// Timeout in seconds to listen for mDNS announcements
        #[arg(short, long, default_value_t = 5)]
        timeout: u64,
    },
    /// Send file(s) to a target Apple device via AirDrop
    Send {
        /// Target URL or IP:Port of the Apple AirDrop receiver (e.g. 192.168.1.50:8770)
        #[arg(short, long)]
        target: String,

        /// Path to one or more files to send
        #[arg(required = true)]
        files: Vec<PathBuf>,

        /// Display name for the sending device shown on the Apple device's screen
        #[arg(long, default_value = "Android Phone")]
        name: String,

        /// Device model shown in AirDrop prompt
        #[arg(long, default_value = "Pixel")]
        model: String,
    },
    /// Run an AirDrop receiver daemon on this device to accept files from Apple devices
    Receive {
        /// Port to bind the AirDrop HTTPS listener
        #[arg(short, long, default_value_t = 8770)]
        port: u16,

        /// Directory where incoming files will be stored
        #[arg(short, long, default_value = "airdrop_received")]
        output: PathBuf,

        /// Device name advertised to nearby Apple devices
        #[arg(long, default_value = "Android AirDrop")]
        name: String,

        /// Auto-accept all incoming file transfer requests
        #[arg(long, default_value_t = true)]
        auto_accept: bool,
    },
    /// Generate raw BLE advertisement hex packets for Android Bluetooth LE Advertiser
    BlePayload {
        /// Generate packet for 'Everyone' mode
        #[arg(long, default_value_t = true)]
        everyone: bool,
    },
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    tracing_subscriber::fmt::init();

    let cli = Cli::parse();

    match cli.command {
        Commands::Scan { timeout } => {
            println!("🔍 Scanning for AirDrop devices on local network (timeout: {}s)...", timeout);
            let mdns = ServiceDaemon::new()?;
            let peers = discover_airdrop_peers(&mdns, Duration::from_secs(timeout)).await?;

            if peers.is_empty() {
                println!("No AirDrop peers found on this Wi-Fi network.");
                println!("Tip: Ensure target Apple device has AirDrop set to 'Everyone' or is on the same Wi-Fi.");
            } else {
                println!("\n✨ Found {} AirDrop peer(s):", peers.len());
                for (i, peer) in peers.iter().enumerate() {
                    println!("{}. Instance: {}", i + 1, peer.instance_name);
                    println!("   Host: {}", peer.host_name);
                    println!("   Port: {}", peer.port);
                    println!("   IPs: {:?}", peer.ip_addresses);
                    if let Some(flags) = peer.txt_records.get("flags") {
                        println!("   Flags: {}", flags);
                    }
                    if let Some(first_ip) = peer.ip_addresses.first() {
                        let target_url = if first_ip.is_ipv6() {
                            format!("https://[{}]:{}", first_ip, peer.port)
                        } else {
                            format!("https://{}:{}", first_ip, peer.port)
                        };
                        println!("   Target URL: {}", target_url);
                    }
                    println!();
                }
            }
        }
        Commands::Send {
            target,
            files,
            name,
            model,
        } => {
            let target_url = if target.starts_with("http://") || target.starts_with("https://") {
                target
            } else {
                format!("https://{}", target)
            };

            println!("🚀 Preparing to AirDrop {} file(s) to {}", files.len(), target_url);
            for f in &files {
                if !f.exists() {
                    eprintln!("Error: File not found: {:?}", f);
                    return Ok(());
                }
                println!("  - {:?}", f);
            }

            let client = AirDropClient::new(name, model);
            match client.send_files(&target_url, &files).await {
                Ok(_) => {
                    println!("\n✅ AirDrop transfer successful! Files received by Apple device.");
                }
                Err(e) => {
                    eprintln!("\n❌ Transfer failed: {}", e);
                }
            }
        }
        Commands::Receive {
            port,
            output,
            name,
            auto_accept,
        } => {
            std::fs::create_dir_all(&output)?;
            let abs_output = std::fs::canonicalize(&output)?;
            println!("📥 Starting AirDrop receiver daemon...");
            println!("   Device Name: {}", name);
            println!("   Port: {}", port);
            println!("   Output Directory: {:?}", abs_output);
            println!("   Auto-Accept: {}", auto_accept);

            // Register mDNS
            let mdns = ServiceDaemon::new()?;
            let instance_id = format!("012345{:06x}", port);
            if let Err(e) = register_airdrop_service(&mdns, &instance_id, "airdrop-host.local.", port, true) {
                error!("Failed to register mDNS service: {}", e);
            }

            let bind_addr: SocketAddr = format!("0.0.0.0:{}", port).parse()?;
            let config = AirDropServerConfig {
                bind_addr,
                computer_name: name,
                download_dir: abs_output,
                auto_accept,
            };

            let shutdown = Arc::new(AtomicBool::new(false));
            let s_clone = shutdown.clone();
            tokio::spawn(async move {
                tokio::signal::ctrl_c().await.ok();
                println!("\nShutting down receiver...");
                s_clone.store(true, std::sync::atomic::Ordering::Relaxed);
            });

            run_airdrop_server(config, shutdown).await?;
        }
        Commands::BlePayload { everyone } => {
            let payload = if everyone {
                AirDropBlePayload::new_everyone()
            } else {
                AirDropBlePayload::new_contacts(vec![])
            };
            let encoded = payload.encode();
            println!("Apple AirDrop BLE Manufacturer Data:");
            println!("Company ID: 0x004C (Apple, Inc.)");
            println!("Hex Payload: {}", hex::encode(&encoded));
            println!("Length: {} bytes", encoded.len());
            println!("\nUse in Android BluetoothLeAdvertiser:");
            println!("AdvertiseData.Builder()");
            println!("    .addManufacturerData(0x004C, hexStringToByteArray(\"{}\"))", hex::encode(&encoded));
            println!("    .build();");
        }
    }

    Ok(())
}
