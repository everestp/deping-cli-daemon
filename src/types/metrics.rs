use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProbeOutcome {
    pub job_id:       String,
    pub batch_id:     String,
    pub node_id:      String,
    pub target_url:   String,
    pub success:      bool,
    pub status_code:  u32,

    // ── Latencies ─────────────────────────────────────────────────────────────
    pub dns_us:   u64,
    pub tcp_us:   u64,
    pub tls_us:   u64,
    pub ttfb_us:  u64,
    pub total_us: u64,

    // ── Security Fields ───────────────────────────────────────────────────────
    pub task_nonce: String,
    pub signature:  Vec<u8>,

    // ── Error Envelope ────────────────────────────────────────────────────────
    pub error_kind: String,
    pub error_msg:  String,
    pub timestamp_ms: u64,
}

impl ProbeOutcome {
    // Note: Update these constructors to accept task_nonce and signature
    #[allow(clippy::too_many_arguments)]
    pub fn success(
        job_id: String, batch_id: String, node_id: String, target_url: String,
        status_code: u32, dns_us: u64, tcp_us: u64, tls_us: u64, ttfb_us: u64,
        total_us: u64, task_nonce: String, signature: Vec<u8>, timestamp_ms: u64,
    ) -> Self {
        Self {
            job_id, batch_id, node_id, target_url, success: true, status_code,
            dns_us, tcp_us, tls_us, ttfb_us, total_us, task_nonce, signature,
            error_kind: String::new(), error_msg: String::new(), timestamp_ms,
        }
    }
}

// ─── Wire Conversion ──────────────────────────────────────────────────────────

impl From<ProbeOutcome> for crate::proto::ProbeResult {
    fn from(o: ProbeOutcome) -> Self {
        crate::proto::ProbeResult {
            job_id:       o.job_id,
            batch_id:     o.batch_id,
            node_id:      o.node_id,
            target_url:   o.target_url,
            success:      o.success,
            status_code:  o.status_code,
            dns_us:       o.dns_us,
            tcp_us:       o.tcp_us,
            tls_us:       o.tls_us,
            ttfb_us:      o.ttfb_us,
            total_us:     o.total_us,
            error_kind:   o.error_kind,
            error_msg:    o.error_msg,
            timestamp_ms: o.timestamp_ms,
            // 🛡️ SECURITY: Map these fields to the wire format
            task_nonce:   o.task_nonce,
            signature:    o.signature,
        }
    }
}
