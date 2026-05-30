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
#[command(name = "deping", version, about = "DePIN Uptime Monitor CLI Miner")]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    Setup { #[arg(long)] force: bool },
    Start { #[arg(long)] force: bool },
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let filter = EnvFilter::try_from_default_env().unwrap_or_else(|_| EnvFilter::new("info"));
    tracing_subscriber::fmt().with_env_filter(filter).init();

    let cli = Cli::parse();

    match cli.command {
        Commands::Setup { force } => {
            let identity = Identity::load_or_create(force);
            println!("\n🚀 DePing Node Initialized");
            // Assuming your Identity struct has a public_key_hex() method now
            println!("🔑 Public Key (Hex): {}\n", identity.public_key_hex());
        }

        Commands::Start { force } => {
            let identity = Arc::new(Identity::load_or_create(force));
            info!("Starting DePIN Miner Node...");
            info!("Node Public Key (Hex): {}", identity.public_key_hex());

            let config = AppConfig::load_from_env();
            let (outbound_tx, outbound_rx) = mpsc::channel(500);

            let http_client = reqwest::Client::builder()
                .pool_max_idle_per_host(20)
                .tcp_nodelay(true)
                .build()?;

            // We pass the hex-encoded string to the worker/register
            let worker = UptimeWorker::new(
                http_client,
                identity.public_key_hex(),
                identity.clone()
            );

            let scheduler = Arc::new(TaskScheduler::new(
                worker,
                config.max_concurrent_jobs,
                outbound_tx.clone(),
            ));

            let stream_engine = StreamEngine::new(config, outbound_rx, outbound_tx);

            stream_engine.run_loop(scheduler, identity).await;
        }
    }
    Ok(())
}
