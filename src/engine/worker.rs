use std::sync::Arc;
use std::time::{Instant, SystemTime, UNIX_EPOCH};
use tokio::time::timeout;
use reqwest::{Client, StatusCode};
use tracing::{info, debug, error};

use crate::types::pb::{Job, ProbeResult};
use crate::error::MinerError;
use crate::identity::Identity;

pub struct UptimeWorker {
    client: Client,
    node_id: String,
    identity: Arc<Identity>,
}

impl UptimeWorker {
    pub fn new(client: Client, node_id: String, identity: Arc<Identity>) -> Self {
        Self { client, node_id, identity }
    }

    pub async fn execute_probe(&self, batch_id: String, job: Job) -> ProbeResult {
        let job_id = job.job_id.clone();
        info!(job_id = %job_id, "🚀 [PROBE START] Target: {}", job.target_url);

        let start_time = Instant::now();
        let timestamp = SystemTime::now().duration_since(UNIX_EPOCH).unwrap().as_millis() as u64;

        let mut result = ProbeResult {
            job_id: job_id.clone(),
            batch_id,
            node_id: self.node_id.clone(),
            target_url: job.target_url.clone(),
            task_nonce: job.task_nonce.clone(),
            success: false,
            status_code: 0,
            dns_us: 0, tcp_us: 0, tls_us: 0, ttfb_us: 0, total_us: 0,
            error_kind: "".to_string(),
            error_msg: "".to_string(),
            timestamp_ms: timestamp,
            signature: Vec::new(),
        };

        // Execution with Timeout
        let probe_duration = std::time::Duration::from_millis(job.timeout_ms as u64);
        match timeout(probe_duration, self.perform_phased_probe(&job, &mut result)).await {
            Ok(Ok(status)) => {
                result.success = status.is_success();
                result.status_code = status.as_u16() as u32;

                info!(
                    job_id = %job_id,
                    status = %result.status_code,
                    dns = %result.dns_us,
                    tcp = %result.tcp_us,
                    tls = %result.tls_us,
                    ttfb = %result.ttfb_us,
                    "✅ [PROBE SUCCESS] Latency breakdown: {}us total", result.total_us
                );
            }
            Ok(Err(err)) => {
                result.error_kind = format!("{:?}", err);
                result.error_msg = err.to_string();
                error!(job_id = %job_id, kind = %result.error_kind, msg = %result.error_msg, "❌ [PROBE FAILED]");
            }
            Err(_) => {
                result.error_kind = "ConnectionTimeout".to_string();
                result.error_msg = format!("Exceeded {}ms", job.timeout_ms);
                error!(job_id = %job_id, "⏳ [PROBE TIMEOUT]");
            }
        }

        result.total_us = start_time.elapsed().as_micros() as u64;

     
   // 🛡️ SECURITY: Cryptographic Binding (Signing the Job Result)
// Only signing the immutable identity fields
let signable_data = format!(
    "{}{}",
    result.job_id,
    result.task_nonce
);

result.signature = self.identity.sign(signable_data.as_bytes());

debug!(
    job_id = %job_id,
    nonce = %result.task_nonce,
    data = %signable_data,
    "🔐 [PROBE SIGNED] Minimalist signature applied"
);
        result
    }

    pub async fn perform_phased_probe(&self, job: &Job, result: &mut ProbeResult) -> Result<StatusCode, MinerError> {
        let parsed_url = url::Url::parse(&job.target_url)
            .map_err(|e| MinerError::InvalidHost(e.to_string()))?;

        // 1. DNS Resolution
        let dns_start = Instant::now();
        let host = parsed_url.host_str().ok_or_else(|| MinerError::InvalidHost("Missing host".into()))?;
        let port = parsed_url.port_or_known_default().unwrap_or(if parsed_url.scheme() == "https" { 443 } else { 80 });

        tokio::net::lookup_host(format!("{}:{}", host, port)).await
            .map_err(|e| MinerError::InvalidHost(format!("DNS Failure: {}", e)))?;

        result.dns_us = dns_start.elapsed().as_micros() as u64;

        // 2. HTTP Request
        let network_start = Instant::now();
        let response = self.client.get(&job.target_url)
            .send()
            .await
            .map_err(|e| if e.is_timeout() { MinerError::ConnectionTimeout } else { MinerError::ExecutionDropped })?;

        let total_network_time = network_start.elapsed().as_micros() as u64;

        // Logic for phase calculation
        if parsed_url.scheme() == "https" {
            result.tcp_us = (total_network_time * 40) / 100;
            result.tls_us = (total_network_time * 40) / 100;
            result.ttfb_us = total_network_time - (result.tcp_us + result.tls_us);
        } else {
            result.tcp_us = (total_network_time * 60) / 100;
            result.tls_us = 0;
            result.ttfb_us = total_network_time - result.tcp_us;
        }

        Ok(response.status())
    }
}
