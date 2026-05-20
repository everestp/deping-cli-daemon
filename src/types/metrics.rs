use serde::{Deserialize, Serialize};

/// Internal probe outcome.  This mirrors the protobuf `ProbeResult` but lives
/// in pure Rust domain space so the engine layer never imports proto types
/// directly.  Conversion to the wire format happens at the stream boundary.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProbeOutcome {
    pub job_id:      String,
    pub batch_id:    String,
    pub node_id:     String,
    pub target_url:  String,

    /// True only when we received a 2xx response.
    pub success:     bool,
    /// Raw HTTP status code; 0 when the connection was never established.
    pub status_code: u32,

    // ── Phase latencies in **microseconds** ───────────────────────────────────
    pub dns_us:   u64,
    pub tcp_us:   u64,
    pub tls_us:   u64,  // 0 for plain HTTP targets
    pub ttfb_us:  u64,
    pub total_us: u64,

    // ── Error envelope (both empty on success) ────────────────────────────────
    /// Stable uppercase tag matching `MinerError::kind_tag()`.
    pub error_kind: String,
    /// Human-readable error description.
    pub error_msg:  String,

    /// Unix epoch milliseconds when the probe was dispatched.
    pub timestamp_ms: u64,
}

impl ProbeOutcome {
    /// Convenience constructor for a successful probe.
    #[allow(clippy::too_many_arguments)]
    pub fn success(
        job_id: String,
        batch_id: String,
        node_id: String,
        target_url: String,
        status_code: u32,
        dns_us: u64,
        tcp_us: u64,
        tls_us: u64,
        ttfb_us: u64,
        total_us: u64,
        timestamp_ms: u64,
    ) -> Self {
        Self {
            job_id,
            batch_id,
            node_id,
            target_url,
            success: true,
            status_code,
            dns_us,
            tcp_us,
            tls_us,
            ttfb_us,
            total_us,
            error_kind: String::new(),
            error_msg: String::new(),
            timestamp_ms,
        }
    }

    /// Convenience constructor for a failed probe.  Phase timings up to the
    /// point of failure are preserved; remaining phases are zeroed.
    #[allow(clippy::too_many_arguments)]
    pub fn failure(
        job_id: String,
        batch_id: String,
        node_id: String,
        target_url: String,
        dns_us: u64,
        tcp_us: u64,
        tls_us: u64,
        total_us: u64,
        error_kind: &'static str,
        error_msg: String,
        timestamp_ms: u64,
    ) -> Self {
        Self {
            job_id,
            batch_id,
            node_id,
            target_url,
            success: false,
            status_code: 0,
            dns_us,
            tcp_us,
            tls_us,
            ttfb_us: 0,
            total_us,
            error_kind: error_kind.to_string(),
            error_msg,
            timestamp_ms,
        }
    }
}

// ─── Wire Conversion ──────────────────────────────────────────────────────────

/// Convert our internal type into the protobuf wire type.
/// Imported here to keep proto types out of the engine crate boundary.
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
        }
    }
}
