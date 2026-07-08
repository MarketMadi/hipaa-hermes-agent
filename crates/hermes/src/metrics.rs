use lazy_static::lazy_static;
use prometheus::{
    register_counter_vec, register_gauge, register_histogram_vec, Encoder, HistogramOpts,
    TextEncoder,
};
use std::sync::Mutex;

use crate::audit::{AuditLog, AuditMetrics};

lazy_static! {
    pub static ref AUDIT_ENTRIES: prometheus::Gauge =
        register_gauge!("hipaa_hermes_audit_entries_total", "Total audit log rows").unwrap();
    pub static ref AUDIT_FAILURES: prometheus::Gauge = register_gauge!(
        "hipaa_hermes_audit_failures_total",
        "Audit rows where outcome is not ok"
    )
    .unwrap();
    pub static ref AUDIT_APPENDS: prometheus::CounterVec = register_counter_vec!(
        "hipaa_hermes_audit_appends_total",
        "Audit append operations",
        &["action", "outcome"]
    )
    .unwrap();
    pub static ref INFERENCE_LATENCY: prometheus::HistogramVec = register_histogram_vec!(
        HistogramOpts::new(
            "hipaa_hermes_inference_latency_seconds",
            "Inference handler latency"
        )
        .buckets(vec![
            0.001, 0.005, 0.01, 0.025, 0.05, 0.1, 0.25, 0.5, 1.0, 2.5, 5.0, 10.0
        ]),
        &["outcome"]
    )
    .unwrap();
    pub static ref AUTH_FAILURES: prometheus::CounterVec = register_counter_vec!(
        "hipaa_hermes_auth_failures_total",
        "401/403 responses",
        &["status"]
    )
    .unwrap();
    pub static ref HTTP_REQUESTS: prometheus::CounterVec = register_counter_vec!(
        "http_requests_total",
        "HTTP requests",
        &["method", "handler", "status"]
    )
    .unwrap();
}

pub struct MetricsRegistry;

impl MetricsRegistry {
    pub async fn refresh_audit_gauges(audit: &AuditLog) {
        if let Ok(m) = audit.metrics().await {
            Self::set_audit_metrics(&m);
        }
    }

    pub fn set_audit_metrics(m: &AuditMetrics) {
        AUDIT_ENTRIES.set(m.total_entries as f64);
        AUDIT_FAILURES.set(m.failure_count as f64);
    }

    pub fn record_audit_append(action: &str, outcome: &str) {
        AUDIT_APPENDS
            .with_label_values(&[action, outcome])
            .inc();
    }

    pub fn record_inference_latency(seconds: f64, outcome: &str) {
        INFERENCE_LATENCY
            .with_label_values(&[outcome])
            .observe(seconds);
    }

    pub fn record_auth_failure(status: &str) {
        AUTH_FAILURES.with_label_values(&[status]).inc();
    }

    pub fn record_http_request(method: &str, handler: &str, status: u16) {
        HTTP_REQUESTS
            .with_label_values(&[method, handler, &status.to_string()])
            .inc();
    }

    pub fn encode() -> Result<String, prometheus::Error> {
        let encoder = TextEncoder::new();
        let metric_families = prometheus::gather();
        let mut buffer = Vec::new();
        encoder.encode(&metric_families, &mut buffer)?;
        Ok(String::from_utf8_lossy(&buffer).into_owned())
    }
}

pub struct HttpMetricsGuard {
    method: String,
    handler: String,
    status: Mutex<Option<u16>>,
}

impl HttpMetricsGuard {
    pub fn new(method: &str, handler: &str) -> Self {
        Self {
            method: method.into(),
            handler: handler.into(),
            status: Mutex::new(None),
        }
    }

    pub fn set_status(&self, status: u16) {
        if let Ok(mut s) = self.status.lock() {
            *s = Some(status);
        }
    }
}

impl Drop for HttpMetricsGuard {
    fn drop(&mut self) {
        let status = self.status.lock().ok().and_then(|s| *s).unwrap_or(500);
        MetricsRegistry::record_http_request(&self.method, &self.handler, status);
        if status == 401 || status == 403 {
            MetricsRegistry::record_auth_failure(&status.to_string());
        }
    }
}
