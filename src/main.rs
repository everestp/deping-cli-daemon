mod config;
mod error;
mod types;
mod engine;
mod network;

use std::sync::Arc;
use tokio::sync::mpsc;
use tracing::info;
use tracing_subscriber::EnvFilter;

use crate::config::AppConfig;
use crate::engine::worker::UptimeWorker;
use crate::engine::scheduler::TaskScheduler;
use crate::network::stream::StreamEngine;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    // Cleaner environment fallback verification loop
    let filter = EnvFilter::try_from_default_env()
        .unwrap_or_else(|_| EnvFilter::new("info"));

    tracing_subscriber::fmt()
        .with_env_filter(filter)
        .with_writer(std::io::stdout)
        .init();

    info!("Initializing DePIN Uptime Monitor CLI Miner Instance...");

    let config = AppConfig::load_from_env();
    info!("Runtime structural configuration loaded successfully. Target node: {}", config.node_id);

    let (outbound_tx, outbound_rx) = mpsc::channel(500);

    let http_client = reqwest::Client::builder()
        .pool_max_idle_per_host(20)
        .pool_idle_timeout(std::time::Duration::from_secs(90))
        .tcp_nodelay(true)
        .build()?;

    let worker = UptimeWorker::new(http_client, config.node_id.clone());
    let scheduler = Arc::new(TaskScheduler::new(worker, config.max_concurrent_jobs, outbound_tx.clone()));

    let stream_engine = StreamEngine::new(config, outbound_rx, outbound_tx);
    stream_engine.run_loop(scheduler).await;

    Ok(())
}
