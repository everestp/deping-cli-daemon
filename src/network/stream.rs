use std::time::{Duration, SystemTime, UNIX_EPOCH};
use std::sync::Arc;
use tokio::sync::{mpsc, Mutex};
use tokio::time::sleep;
use tracing::{info, warn, error, debug};
use rand::Rng;

use crate::config::AppConfig;
use crate::identity::Identity;
use crate::engine::scheduler::TaskScheduler;
use crate::types::pb::{
    monitor_service_client::MonitorServiceClient,
    MinerMessage, MinerRegister, AuthResponse,
    server_message, miner_message::Payload,
};

pub struct StreamEngine {
    config: AppConfig,
    outbound_rx: Arc<Mutex<mpsc::Receiver<MinerMessage>>>,
}

impl StreamEngine {
    pub fn new(config: AppConfig, outbound_rx: mpsc::Receiver<MinerMessage>, _outbound_tx: mpsc::Sender<MinerMessage>) -> Self {
        Self {
            config,
            outbound_rx: Arc::new(Mutex::new(outbound_rx)),
        }
    }

    pub async fn run_loop(self, scheduler: Arc<TaskScheduler>, identity: Arc<Identity>) {
        let mut attempts = 0;

        loop {
            info!(server = %self.config.server_url, "📡 [NETWORK] Initializing connection to Ingress Core...");

            let endpoint = tonic::transport::Endpoint::from_shared(self.config.server_url.clone())
                .expect("Invalid server URL")
                .tcp_nodelay(true)
                .connect_timeout(Duration::from_secs(5));

            match MonitorServiceClient::connect(endpoint).await {
                Ok(mut client) => {
                    info!("🔗 [NETWORK] Connected. Authenticating Node: {}", self.config.node_id);
                    attempts = 0;

                    let (tx, rx) = mpsc::channel::<MinerMessage>(100);

                    // 1. Immediate Registration
                    let _ = tx.send(MinerMessage {
                        payload: Some(Payload::Register(MinerRegister {
                            node_id: self.config.node_id.clone(),
                            version: env!("CARGO_PKG_VERSION").to_string(),
                            timestamp_ms: SystemTime::now().duration_since(UNIX_EPOCH).unwrap().as_millis() as u64,
                        })),
                    }).await;

                    let heartbeat_handle = self.spawn_heartbeat(tx.clone(), self.config.node_id.clone());
                    let forwarder_handle = self.spawn_forwarder(tx.clone());

                    // 2. Stream Pipeline
                    match client.job_stream(tokio_stream::wrappers::ReceiverStream::new(rx)).await {
                        Ok(response) => {
                            let mut stream = response.into_inner();
                            info!("✅ [SYNC] Stream active and synchronized.");

                            while let Ok(Some(msg)) = stream.message().await {
                                if let Some(payload) = msg.payload {
                                    match payload {
                                        server_message::Payload::AuthChallenge(ch) => {
                                            info!(nonce = %ch.nonce, "🛡️ [AUTH] Handshake challenge received.");
                                            let sig = identity.sign(ch.nonce.as_bytes());
                                            let _ = tx.send(MinerMessage {
                                                payload: Some(Payload::AuthResponse(AuthResponse { signature: sig })),
                                            }).await;
                                        }
                                        server_message::Payload::JobBatch(batch) => {
                                            info!(batch_id = %batch.batch_id, jobs = batch.jobs.len(), "📦 [BATCH] Work received.");
                                            scheduler.dispatch_batch(batch).await;
                                        }
                                        server_message::Payload::Pong(pong) => {
                                            let latency = SystemTime::now().duration_since(UNIX_EPOCH).unwrap().as_millis() as u64 - pong.timestamp_ms;
                                            debug!(latency_ms = %latency, "🏓 [PONG] Health check verified.");
                                        }
                                        _ => {}
                                    }
                                }
                            }
                            warn!("⚠️ [STREAM] Connection closed by remote host.");
                        }
                        Err(e) => error!("❌ [STREAM] Pipeline failure: {:?}", e),
                    }
                    heartbeat_handle.abort();
                    forwarder_handle.abort();
                }
                Err(e) => warn!("⏳ [NETWORK] Connection failed: {:?}. Retrying...", e),
            }

            attempts += 1;
            let delay = self.calculate_delay(attempts);
            info!(retry_in = ?delay, "💤 [SYSTEM] Entering cool-down...");
            sleep(delay).await;
        }
    }

    fn spawn_heartbeat(&self, tx: mpsc::Sender<MinerMessage>, node_id: String) -> tokio::task::JoinHandle<()> {
        tokio::spawn(async move {
            loop {
                sleep(Duration::from_secs(30)).await;
                let ping = MinerMessage { payload: Some(Payload::Ping(crate::types::pb::Ping {
                    timestamp_ms: SystemTime::now().duration_since(UNIX_EPOCH).unwrap().as_millis() as u64,
                    node_id: node_id.clone(),
                }))};
                if tx.send(ping).await.is_err() { break; }
            }
        })
    }

    fn spawn_forwarder(&self, tx: mpsc::Sender<MinerMessage>) -> tokio::task::JoinHandle<()> {
        let shared_rx = Arc::clone(&self.outbound_rx);
        tokio::spawn(async move {
            loop {
                let msg = { shared_rx.lock().await.recv().await };
                match msg {
                    Some(m) => { if tx.send(m).await.is_err() { break; } },
                    None => break,
                }
            }
        })
    }

    fn calculate_delay(&self, attempts: u32) -> Duration {
        let delay = Duration::from_secs(1) * 2_u32.pow(attempts.saturating_sub(1));
        std::cmp::min(delay, Duration::from_secs(30)) + Duration::from_millis(rand::thread_rng().gen_range(0..500))
    }
}
