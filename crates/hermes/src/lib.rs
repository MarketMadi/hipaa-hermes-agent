pub mod audit;
pub mod auth;
pub mod config;
pub mod deid;
pub mod inference;
pub mod llm;
pub mod metrics;
pub mod policy;

use axum::{
    extract::{Query, State},
    http::StatusCode,
    routing::{get, post},
    Json, Router,
};
use serde::Deserialize;
use serde_json::{json, Value};
use std::sync::Arc;
use tower_http::{
    cors::{Any, CorsLayer},
    services::ServeDir,
    trace::TraceLayer,
};

use audit::AuditLog;
use auth::{AuditorAuth, EitherAuth, OperatorAuth};
use config::Config;
use inference::{run_inference, InferenceRequest};
use metrics::MetricsRegistry;

#[derive(Clone)]
pub struct AppState {
    pub config: Config,
    pub audit: Arc<AuditLog>,
}

#[derive(Debug, Deserialize)]
struct AuditQuery {
    #[serde(default = "default_limit")]
    limit: i64,
}

fn default_limit() -> i64 {
    50
}

pub fn build_router(state: AppState) -> Router {
    let cors = CorsLayer::new()
        .allow_origin(Any)
        .allow_methods(Any)
        .allow_headers(Any);

    Router::new()
        .route("/health", get(health))
        .route("/metrics", get(prometheus_metrics))
        .route("/api/stats", get(stats_json))
        .route("/v1/inference", post(inference_handler))
        .route("/v1/audit", get(list_audit))
        .route("/v1/audit/export", get(export_audit))
        .route("/v1/demo/scenarios", get(demo_scenarios))
        .nest_service("/demo", ServeDir::new("demo").append_index_html_on_directories(true))
        .layer(cors)
        .layer(TraceLayer::new_for_http())
        .with_state(state)
}

async fn demo_scenarios() -> Result<Json<Value>, StatusCode> {
    let body = std::fs::read_to_string("demo/scenarios.json").map_err(|_| StatusCode::NOT_FOUND)?;
    let v: Value = serde_json::from_str(&body).map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
    Ok(Json(v))
}

async fn health() -> Json<Value> {
    Json(json!({ "status": "ok", "version": "0.2.0" }))
}

async fn prometheus_metrics(State(state): State<AppState>) -> Result<String, StatusCode> {
    MetricsRegistry::refresh_audit_gauges(&state.audit);
    MetricsRegistry::encode().map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)
}

async fn stats_json(State(state): State<AppState>) -> Result<Json<Value>, StatusCode> {
    let m = state
        .audit
        .metrics()
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
    MetricsRegistry::set_audit_metrics(&m);
    Ok(Json(json!({
        "audit_total": m.total_entries,
        "audit_failures": m.failure_count,
        "audit_by_action": m.by_action,
    })))
}

async fn inference_handler(
    State(state): State<AppState>,
    OperatorAuth(auth): OperatorAuth,
    Json(body): Json<InferenceRequest>,
) -> Result<Json<Value>, (StatusCode, Json<Value>)> {
    match run_inference(&state.config, &state.audit, &auth, body).await {
        Ok(resp) => Ok(Json(json!({
            "output": resp.output,
            "latency_ms": resp.latency_ms,
            "audit_id": resp.audit_id,
            "entry_hash": resp.entry_hash,
            "deid_redaction_count": resp.deid_redaction_count,
            "deid_categories": resp.deid_categories,
            "deidentified_prompt": resp.deidentified_prompt,
        }))),
        Err(e) => Err((e.status_code(), Json(json!({ "detail": e.detail() })))),
    }
}

async fn list_audit(
    State(state): State<AppState>,
    EitherAuth(auth): EitherAuth,
    Query(q): Query<AuditQuery>,
) -> Result<Json<Value>, StatusCode> {
    let limit = q.limit.clamp(1, 200);
    let entries = state
        .audit
        .list_entries(limit)
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    let mut out = Vec::new();
    for e in entries {
        let hash_valid = state.audit.verify_entry(e.id).unwrap_or(false);
        out.push(json!({
            "id": e.id,
            "ts": e.ts,
            "actor": e.actor,
            "role": e.role,
            "action": e.action,
            "resource": e.resource,
            "outcome": e.outcome,
            "metadata": e.metadata,
            "entry_hash": e.entry_hash,
            "hash_valid": hash_valid,
        }));
    }

    Ok(Json(json!({
        "role": auth.role.as_str(),
        "entries": out,
    })))
}

async fn export_audit(
    State(state): State<AppState>,
    AuditorAuth(auth): AuditorAuth,
) -> Result<Json<Value>, StatusCode> {
    let entries = state
        .audit
        .list_entries(500)
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    let out: Vec<Value> = entries
        .into_iter()
        .map(|e| {
            json!({
                "id": e.id,
                "ts": e.ts,
                "action": e.action,
                "resource": e.resource,
                "outcome": e.outcome,
                "entry_hash": e.entry_hash,
            })
        })
        .collect();

    Ok(Json(json!({
        "exported_by": auth.actor,
        "count": out.len(),
        "entries": out,
    })))
}
