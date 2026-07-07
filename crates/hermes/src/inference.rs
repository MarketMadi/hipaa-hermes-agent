use axum::http::StatusCode;
use serde::Deserialize;
use serde_json::json;
use std::time::Instant;
use tracing::{info, warn};

use crate::audit::AuditLog;
use crate::auth::AuthContext;
use crate::config::Config;
use crate::deid;
use crate::llm::{self, LlmError};
use crate::metrics::MetricsRegistry;
use crate::policy::{self, PolicyResult};

#[derive(Debug, Deserialize)]
pub struct InferenceRequest {
    pub prompt: String,
    #[serde(default = "default_skill")]
    pub skill: String,
}

fn default_skill() -> String {
    "vault-answer".into()
}

fn stub_response(skill: &str, deid_len: usize, reason: &str) -> String {
    format!(
        "[DEMO STUB — {reason}]\n\n\
         With Claude credits enabled, the gateway returns a real clinical answer here.\n\n\
         Example (illustrative only): For discharge on antibiotics, the pharmacist would \
         review each medication, explain how and when to take it, warn about side effects, \
         and confirm the patient knows when to call the clinic.\n\n\
         (skill={skill}, deid_prompt_len={deid_len} chars sent to model)",
        skill = skill,
        deid_len = deid_len
    )
}

#[derive(Debug)]
pub struct InferenceResponse {
    pub output: String,
    pub latency_ms: f64,
    pub audit_id: i64,
    pub entry_hash: String,
    pub deid_redaction_count: u32,
    pub deid_categories: Vec<String>,
    pub deidentified_prompt: String,
}

#[derive(Debug)]
pub enum InferenceError {
    PolicyDenied { reason: &'static str, audit_id: i64, entry_hash: String },
    Llm(LlmError),
    Audit(crate::audit::AuditError),
}

fn audit_metadata(
    prompt_len: usize,
    deid: &deid::DeidResult,
    latency_ms: f64,
    extra: serde_json::Value,
) -> serde_json::Value {
    let mut meta = json!({
        "prompt_len": prompt_len,
        "deid_prompt_len": deid.text.len(),
        "deid_redaction_count": deid.redaction_count,
        "deid_categories": deid.categories,
        "latency_ms": (latency_ms * 100.0).round() / 100.0,
    });
    if let serde_json::Value::Object(ref mut map) = meta {
        if let serde_json::Value::Object(ex) = extra {
            map.extend(ex);
        }
    }
    meta
}

pub async fn run_inference(
    config: &Config,
    audit: &AuditLog,
    auth: &AuthContext,
    body: InferenceRequest,
) -> Result<InferenceResponse, InferenceError> {
    if body.prompt.len() > 4000 {
        return Err(InferenceError::Llm(LlmError::Api {
            status: 400,
            body: "prompt exceeds 4000 characters".into(),
        }));
    }
    if body.skill.len() > 64 {
        return Err(InferenceError::Llm(LlmError::Api {
            status: 400,
            body: "skill exceeds 64 characters".into(),
        }));
    }

    let started = Instant::now();

    match policy::check_skill(&body.skill) {
        PolicyResult::Deny { reason } => {
            return policy_denied(audit, auth, &body, reason, started).await;
        }
        PolicyResult::Allow => {}
    }

    match policy::check_hard_block(&body.prompt) {
        PolicyResult::Deny { reason } => {
            return policy_denied(audit, auth, &body, reason, started).await;
        }
        PolicyResult::Allow => {}
    }

    let deid = deid::scrub(&body.prompt);

    info!(
        actor = %auth.actor,
        redactions = deid.redaction_count,
        categories = ?deid.categories,
        "de-identification applied before LLM"
    );

    let (output, model, outcome) = if config.llm_available() {
        match llm::complete(config, &deid.text, &body.skill).await {
            Ok(text) => (text, config.model_label(), "ok"),
            Err(e) => {
                if config.llm_fallback_stub && e.eligible_for_stub_fallback() {
                    tracing::warn!(error = %e, "LLM unavailable — using demo stub fallback");
                    (
                        stub_response(&body.skill, deid.text.len(), "LLM backend unavailable"),
                        "stub-fallback".into(),
                        "ok",
                    )
                } else {
                    let latency_ms = started.elapsed().as_secs_f64() * 1000.0;
                    let metadata = audit_metadata(
                        body.prompt.len(),
                        &deid,
                        latency_ms,
                        json!({ "error": e.to_string(), "model": null }),
                    );
                    let _entry = audit
                        .append(
                            &auth.actor,
                            auth.role.as_str(),
                            "inference",
                            &body.skill,
                            "error",
                            &metadata,
                        )
                        .map_err(InferenceError::Audit)?;
                    MetricsRegistry::record_audit_append("inference", "error");
                    MetricsRegistry::record_inference_latency(
                        started.elapsed().as_secs_f64(),
                        "error",
                    );
                    MetricsRegistry::refresh_audit_gauges(audit);
                    return Err(InferenceError::Llm(e));
                }
            }
        }
    } else {
        (
            stub_response(&body.skill, deid.text.len(), "LLM disabled"),
            "stub".into(),
            "ok",
        )
    };

    let latency_ms = started.elapsed().as_secs_f64() * 1000.0;
    let metadata = audit_metadata(
        body.prompt.len(),
        &deid,
        latency_ms,
        json!({ "model": model }),
    );
    let entry = audit
        .append(
            &auth.actor,
            auth.role.as_str(),
            "inference",
            &body.skill,
            outcome,
            &metadata,
        )
        .map_err(InferenceError::Audit)?;
    MetricsRegistry::record_audit_append("inference", outcome);
    MetricsRegistry::record_inference_latency(started.elapsed().as_secs_f64(), outcome);
    MetricsRegistry::refresh_audit_gauges(audit);

    info!(
        actor = %auth.actor,
        skill = %body.skill,
        prompt_len = body.prompt.len(),
        deid_prompt_len = deid.text.len(),
        audit_id = entry.id,
        latency_ms = latency_ms,
        model = %model,
        "inference completed"
    );

    Ok(InferenceResponse {
        output,
        latency_ms,
        audit_id: entry.id,
        entry_hash: entry.entry_hash,
        deid_redaction_count: deid.redaction_count,
        deid_categories: deid.categories.clone(),
        deidentified_prompt: deid.text.clone(),
    })
}

async fn policy_denied(
    audit: &AuditLog,
    auth: &AuthContext,
    body: &InferenceRequest,
    reason: &'static str,
    started: Instant,
) -> Result<InferenceResponse, InferenceError> {
    let latency_ms = started.elapsed().as_secs_f64() * 1000.0;
    let empty_deid = deid::scrub("");
    let metadata = audit_metadata(body.prompt.len(), &empty_deid, latency_ms, json!({
        "policy_reason": reason,
    }));
    let entry = audit
        .append(
            &auth.actor,
            auth.role.as_str(),
            "inference",
            &body.skill,
            "blocked",
            &metadata,
        )
        .map_err(InferenceError::Audit)?;
    MetricsRegistry::record_audit_append("inference", "blocked");
    MetricsRegistry::record_inference_latency(started.elapsed().as_secs_f64(), "blocked");
    MetricsRegistry::refresh_audit_gauges(audit);
    warn!(
        actor = %auth.actor,
        skill = %body.skill,
        reason = reason,
        audit_id = entry.id,
        "inference blocked by policy"
    );
    Err(InferenceError::PolicyDenied {
        reason,
        audit_id: entry.id,
        entry_hash: entry.entry_hash,
    })
}

impl InferenceError {
    pub fn status_code(&self) -> StatusCode {
        match self {
            InferenceError::PolicyDenied { .. } => StatusCode::FORBIDDEN,
            InferenceError::Llm(LlmError::NotConfigured) => StatusCode::SERVICE_UNAVAILABLE,
            InferenceError::Llm(LlmError::Api { status, body: _ }) if *status == 400 => {
                StatusCode::BAD_REQUEST
            }
            InferenceError::Llm(LlmError::Api { status, .. }) if *status == 402 => {
                StatusCode::BAD_REQUEST
            }
            InferenceError::Llm(_) => StatusCode::BAD_GATEWAY,
            InferenceError::Audit(_) => StatusCode::INTERNAL_SERVER_ERROR,
        }
    }

    pub fn detail(&self) -> String {
        match self {
            InferenceError::PolicyDenied { reason, .. } => {
                format!("request blocked by policy: {reason}")
            }
            InferenceError::Llm(e) => e.to_string(),
            InferenceError::Audit(e) => e.to_string(),
        }
    }
}
