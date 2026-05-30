use std::sync::Arc;
use tokio::sync::{mpsc, Semaphore};
use tokio::time::{timeout, Duration}; // Add this for safety
use tracing::{info, warn, error};
use crate::types::pb::{JobBatch, MinerMessage, miner_message::Payload};
use crate::engine::worker::UptimeWorker;

pub struct TaskScheduler {
    worker: Arc<UptimeWorker>,
    semaphore: Arc<Semaphore>,
    outbound_tx: mpsc::Sender<MinerMessage>,
}

impl TaskScheduler {
    pub fn new(worker: UptimeWorker, max_concurrency: usize, outbound_tx: mpsc::Sender<MinerMessage>) -> Self {
        Self {
            worker: Arc::new(worker),
            semaphore: Arc::new(Semaphore::new(max_concurrency)),
            outbound_tx,
        }
    }

    pub async fn dispatch_batch(&self, batch: JobBatch) {
        let batch_id = batch.batch_id.clone();
        info!("Scheduling job batch: {} containing {} jobs", batch_id, batch.jobs.len());

        for job in batch.jobs {
            let worker = Arc::clone(&self.worker);
            let semaphore = Arc::clone(&self.semaphore);
            let outbound_tx = self.outbound_tx.clone();
            let b_id = batch_id.clone();

            tokio::spawn(async move {
                // 🛡️ GATEKEEPER: Enforce concurrency limits
                let _permit = semaphore.acquire().await.expect("Semaphore closed");

                // 🎯 EXECUTION: Perform the probe with a 30s hard timeout
                let probe_result = match timeout(Duration::from_secs(30), worker.execute_probe(b_id, job)).await {
                    Ok(result) => result,
                    Err(_) => {
                        error!("❌ Job execution timed out, worker hung.");
                        return; // Permit drops automatically here
                    }
                };

                // 📤 DISPATCH: Send back to gRPC stream
                let response_msg = MinerMessage {
                    payload: Some(Payload::Result(probe_result)),
                };

                if let Err(err) = outbound_tx.send(response_msg).await {
                    warn!("Stream pipeline disconnected, dropping result: {}", err);
                }
                // _permit dropped here
            });
        }
    }
}
