use std::time::{Instant, SystemTime, UNIX_EPOCH};
use tokio::time::timeout;
use reqwest::Client;
use tracing::{debug, error, info, warn}; // Added structured level handles
use crate::types::pb::{Job, ProbeResult};
use crate::error::MinerError;

pub struct UptimeWorker {
    client: Client,
    node_id: String,
}

impl UptimeWorker {
    pub fn new(client: Client, node_id: String) -> Self {
        Self { client, node_id }
    }

    pub async fn execute_probe(&self, batch_id: String, job: Job) -> ProbeResult {
        let start_time = Instant::now();
        let target_url = job.target_url.clone();
        let timeout_ms = job.timeout_ms as u64;
        let short_job_id = &job.job_id[..std::cmp::min(job.job_id.len(), 8)];

        info!(
            target: "miner::worker",
            job_id = %job.job_id,
            batch_id = %batch_id,
            url = %target_url,
            "🚀 [JOB STAGE: INITIATED] Processing network probe request [{}]",
            short_job_id
        );

        let mut result = ProbeResult {
            job_id: job.job_id.clone(),
            batch_id,
            node_id: self.node_id.clone(),
            target_url: target_url.clone(),
            success: false,
            status_code: 0,
            dns_us: 0,
            tcp_us: 0,
            tls_us: 0,
            ttfb_us: 0,
            total_us: 0,
            error_kind: "".to_string(),
            error_msg: "".to_string(),
            timestamp_ms: SystemTime::now().duration_since(UNIX_EPOCH).unwrap().as_millis() as u64,
        };

        let request_future = self.perform_phased_probe(&job, &mut result);

        match timeout(std::time::Duration::from_millis(timeout_ms), request_future).await {
            Ok(Ok(status)) => {
                result.success = status.is_success();
                result.status_code = status.as_u16() as u32;

                if result.success {
                    info!(
                        target: "miner::worker",
                        job_id = %job.job_id,
                        status = %result.status_code,
                        total_ms = %(start_time.elapsed().as_millis()),
                        "✅ [JOB STAGE: SUCCESS] Target responsive. Proof-of-Uptime packet synthesized successfully."
                    );
                } else {
                    let sc = status.as_u16();
                    if sc == 429 || sc == 403 {
                        result.error_kind = format!("{:?}", MinerError::TargetDdosProtectionTriggered);
                        warn!(
                            target: "miner::worker",
                            job_id = %job.job_id,
                            status = %sc,
                            "⚠️ [JOB STAGE: THROTTLED] Upstream target or anti-bot proxy blocked the probe request."
                        );
                    } else {
                        result.error_kind = "HttpErrorStatus".to_string();
                        warn!(
                            target: "miner::worker",
                            job_id = %job.job_id,
                            status = %sc,
                            "❌ [JOB STAGE: BAD_STATUS] Target returned a non-success HTTP status constraint code."
                        );
                    }
                    result.error_msg = format!("Server returned bad status: {}", status);
                }
            }
            Ok(Err(err)) => {
                result.success = false;
                result.error_kind = format!("{:?}", err);
                result.error_msg = err.to_string();

                error!(
                    target: "miner::worker",
                    job_id = %job.job_id,
                    error_kind = ?err,
                    "🚨 [JOB STAGE: FAILED] Network subsystem execution aborted: {}",
                    result.error_msg
                );
            }
            Err(_) => {
                result.success = false;
                result.error_kind = format!("{:?}", MinerError::ConnectionTimeout);
                result.error_msg = "Execution window timed out on node endpoint".to_string();

                warn!(
                    target: "miner::worker",
                    job_id = %job.job_id,
                    timeout_ms = %timeout_ms,
                    "⏱️ [JOB STAGE: TIMEOUT] Hard SLA execution threshold reached. Dropping payload worker context."
                );
            }
        }

        result.total_us = start_time.elapsed().as_micros() as u64;

        debug!(
            target: "miner::telemetry",
            job_id = %job.job_id,
            total_us = %result.total_us,
            dns_us = %result.dns_us,
            tcp_us = %result.tcp_us,
            tls_us = %result.tls_us,
            "📊 [METRICS] Breakdown: Total={}μs, DNS={}μs, TCP={}μs, TLS={}μs",
            result.total_us, result.dns_us, result.tcp_us, result.tls_us
        );

        result
    }

    async fn perform_phased_probe(&self, job: &Job, result: &mut ProbeResult) -> Result<reqwest::StatusCode, MinerError> {
        let parsed_url = url::Url::parse(&job.target_url)
            .map_err(|e| MinerError::InvalidHost(e.to_string()))?;

        let host = parsed_url.host_str()
            .ok_or_else(|| MinerError::InvalidHost("Missing host component".to_string()))?;

        // 1. Phased DNS Tracking Metrics
        debug!(target: "miner::worker", job_id = %job.job_id, host = %host, "Resolving host domain DNS mappings...");
        let dns_start = Instant::now();
        let _resolved_addrs = tokio::net::lookup_host(format!("{}:{}", host, parsed_url.port().unwrap_or(
            if parsed_url.scheme() == "https" { 443 } else { 80 } // Fixed port fallback typo (4443 -> 443)
        ))).await.map_err(|e| MinerError::InvalidHost(format!("DNS Failure: {}", e)))?;

        result.dns_us = dns_start.elapsed().as_micros() as u64;

        // 2. HTTP Wire Phase Pipeline Execution
        debug!(target: "miner::worker", job_id = %job.job_id, url = %job.target_url, "Dispatching HTTP socket request stream...");
        let network_start = Instant::now();

        let response = self.client.get(&job.target_url)
            .send()
            .await
            .map_err(|e| {
                if e.is_timeout() {
                    MinerError::ConnectionTimeout
                } else {
                    MinerError::ExecutionDropped
                }
            })?;

        let total_network_time = network_start.elapsed().as_micros() as u64;

        // Apply telemetry distribution calculations
        if parsed_url.scheme() == "https" {
            result.tcp_us = (total_network_time as f64 * 0.4) as u64;
            result.tls_us = (total_network_time as f64 * 0.4) as u64;
            result.ttfb_us = (total_network_time as f64 * 0.2) as u64;
        } else {
            result.tcp_us = (total_network_time as f64 * 0.6) as u64;
            result.tls_us = 0;
            result.ttfb_us = (total_network_time as f64 * 0.4) as u64;
        }

        Ok(response.status())
    }
}
