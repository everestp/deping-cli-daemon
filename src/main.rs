mod config;
mod error;
mod types;
mod engine;
mod network;
mod identity;

use std::sync::Arc;
use tokio::sync::mpsc;
use tracing::info;
use tracing_subscriber::EnvFilter;

use clap::{Parser, Subcommand};

use crate::config::AppConfig;
use crate::engine::worker::UptimeWorker;
use crate::engine::scheduler::TaskScheduler;
use crate::network::stream::StreamEngine;
use crate::identity::Identity;

#[derive(Parser)]
#[command(name = "deping")]
#[command(about = "DePIN Uptime Monitor CLI Miner")]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    /// Initialize node identity (Solana-style keypair)
    Setup {
        #[arg(long)]
        force: bool,
    },

    /// Start miner node
    Start {
        #[arg(long)]
        force: bool,
    },
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    // logging
    let filter = EnvFilter::try_from_default_env()
        .unwrap_or_else(|_| EnvFilter::new("info"));

    tracing_subscriber::fmt()
        .with_env_filter(filter)
        .with_writer(std::io::stdout)
        .init();

    let cli = Cli::parse();

    match cli.command {

        // -------------------------
        // SETUP COMMAND
        // -------------------------
        Commands::Setup { force } => {
            let identity = Identity::load_or_create(force);

            println!("\n🚀 DePing Node Initialized");
            println!("🔑 Public Key: {}\n", identity.public_key);

            return Ok(());
        }

        // -------------------------
        // START COMMAND
        // -------------------------
        Commands::Start { force } => {
            let identity = Identity::load_or_create(force);

            info!("Starting DePIN Miner Node...");
            info!("Node Public Key: {}", identity.public_key);

            let config = AppConfig::load_from_env();

            let (outbound_tx, outbound_rx) = mpsc::channel(500);

            let http_client = reqwest::Client::builder()
                .pool_max_idle_per_host(20)
                .pool_idle_timeout(std::time::Duration::from_secs(90))
                .tcp_nodelay(true)
                .build()?;

            let worker = UptimeWorker::new(http_client, identity.public_key.clone());

            let scheduler = Arc::new(
                TaskScheduler::new(
                    worker,
                    config.max_concurrent_jobs,
                    outbound_tx.clone(),
                )
            );

            let stream_engine = StreamEngine::new(config, outbound_rx, outbound_tx);

            stream_engine.run_loop(scheduler).await;
        }
    }

    Ok(())
}
