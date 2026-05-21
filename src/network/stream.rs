use std::time::{Duration, SystemTime, UNIX_EPOCH};
use tokio::sync::{mpsc, Mutex};
use tokio::time::sleep;
use tracing::{info, warn, error, debug};
use rand::Rng;
use std::sync::Arc;

use crate::config::AppConfig;
use crate::types::pb::{
    monitor_service_client::MonitorServiceClient,
    MinerMessage, MinerRegister, Ping, server_message, miner_message::Payload,
};
use crate::engine::scheduler::TaskScheduler;

pub struct StreamEngine {
    config: AppConfig,
    outbound_rx: Arc<Mutex<mpsc::Receiver<MinerMessage>>>,
    _outbound_tx: mpsc::Sender<MinerMessage>,
}

impl StreamEngine {
    pub fn new(config: AppConfig, outbound_rx: mpsc::Receiver<MinerMessage>, outbound_tx: mpsc::Sender<MinerMessage>) -> Self {
        Self {
            config,
            outbound_rx: Arc::new(Mutex::new(outbound_rx)),
            _outbound_tx: outbound_tx
        }
    }

    pub async fn run_loop(self, scheduler: std::sync::Arc<TaskScheduler>) {
        let mut attempts = 0;
        let base_delay = Duration::from_secs(1);
        let max_delay = Duration::from_secs(30);

        loop {
            info!("Attempting connection setup to Go Ingress Core at: {}", self.config.server_url);

            match MonitorServiceClient::connect(self.config.server_url.clone()).await {
                Ok(mut client) => {
                    attempts = 0;
                    info!("Successfully multiplexed gRPC pipeline channel. Authenticating daemon...");

                    let (tx, rx) = mpsc::channel::<MinerMessage>(100);

                    // 1. Submit Registration Handshake Payload
                    let registration = MinerMessage {
                        payload: Some(Payload::Register(MinerRegister {
                            node_id: self.config.node_id.clone(),
                            version: env!("CARGO_PKG_VERSION").to_string(),
                            timestamp_ms: SystemTime::now().duration_since(UNIX_EPOCH).unwrap().as_millis() as u64,
                        })),
                    };
                    let _ = tx.send(registration).await;

                    // 2. Keepalive Heartbeat Task
                    let heartbeat_tx = tx.clone();
                    let node_id = self.config.node_id.clone();

                    let heartbeat_handle = tokio::spawn(async move {
                        loop {
                            sleep(Duration::from_secs(15)).await;
                            let ping = MinerMessage {
                                payload: Some(Payload::Ping(Ping {
                                    timestamp_ms: SystemTime::now().duration_since(UNIX_EPOCH).unwrap().as_millis() as u64,
                                    node_id: node_id.clone(),
                                })),
                            };
                            if heartbeat_tx.send(ping).await.is_err() {
                                break;
                            }
                        }
                    });

                    // 3. Thread-Safe Shared Result Forwarder Task
                    let pipeline_forwarder_tx = tx.clone();
                    let shared_rx = Arc::clone(&self.outbound_rx);
                    let forwarder_handle = tokio::spawn(async move {
                        let mut locked_rx = shared_rx.lock().await;
                        while let Some(msg) = locked_rx.recv().await {
                            if pipeline_forwarder_tx.send(msg).await.is_err() {
                                break;
                            }
                        }
                    });

                    let request_stream = tokio_stream::wrappers::ReceiverStream::new(rx);

                    match client.job_stream(request_stream).await {
                        Ok(response) => {
                            let mut response_stream = response.into_inner();
                            info!("Connection fully established. Miner listening for incoming streaming workloads...");

                            while let Ok(Some(server_msg)) = response_stream.message().await {
                                if let Some(payload) = server_msg.payload {
                                    match payload {
                                        server_message::Payload::Pong(pong) => {
                                            debug!("Heartbeat acknowledged by gateway core server. Latency check: {}ms",
                                                SystemTime::now().duration_since(UNIX_EPOCH).unwrap().as_millis() as u64 - pong.timestamp_ms
                                            );
                                        }
                                        server_message::Payload::JobBatch(batch) => {
                                            scheduler.dispatch_batch(batch).await;
                                        }
                                    }
                                }
                            }
                        }
                        Err(status) => {
                            error!("Error state emitted down active pipeline stream channel: {}", status.message());
                        }
                    }

                    // Gracefully terminate sub-tasks before performing reconnect backoff calculations
                    heartbeat_handle.abort();
                    forwarder_handle.abort();
                }
                Err(err) => {
                    warn!("Failed connection tracking to Go gateway core ingress endpoint: {}", err);
                }
            }

            attempts += 1;
            let mut delay = base_delay * 2_u32.pow(attempts - 1);
            if delay > max_delay {
                delay = max_delay;
            }
            let jitter = Duration::from_millis(rand::thread_rng().gen_range(0..500));
            delay += jitter;

            warn!("Reconnection safety engine triggered. Sleeping for {} seconds before retry...", delay.as_secs_f32());
            sleep(delay).await;
        }
    }
}
