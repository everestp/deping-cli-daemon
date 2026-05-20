use std::sync::Arc;
use tokio::sync::{mpsc, Semaphore};
// Added debug here
use tracing::{info, warn, error, debug};
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
                let _permit = match semaphore.acquire().await {
                    Ok(permit) => permit,
                    Err(e) => {
                        error!("Failed to acquire rate limiting execution permit: {}", e);
                        return;
                    }
                };

                debug!("Acquired execution permit. Probing target: {}", job.target_url);
                let probe_result = worker.execute_probe(b_id, job).await;

                let response_msg = MinerMessage {
                    payload: Some(Payload::Result(probe_result)),
                };

                if let Err(err) = outbound_tx.send(response_msg).await {
                    warn!("Failed to dispatch execution metrics back to local stream pipeline channel: {}", err);
                }
            });
        }
    }
}
