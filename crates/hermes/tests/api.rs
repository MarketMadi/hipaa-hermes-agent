use axum::{
    body::Body,
    http::{Request, StatusCode},
};
use hermes::{
    audit::AuditLog,
    build_router,
    config::{Config, LlmProvider},
    AppState,
};
use secrecy::Secret;
use serde_json::Value;
use std::sync::Arc;
use tempfile::NamedTempFile;
use tower::ServiceExt;

fn test_config(db_path: &std::path::Path) -> Config {
    Config {
        env: hermes::config::HermesEnv::Local,
        behind_proxy: false,
        database_path: db_path.to_path_buf(),
        admin_secret: Secret::new("change-me-operator".into()),
        auditor_secret: Secret::new("change-me-auditor".into()),
        anthropic_api_key: None,
        claude_model: "claude-sonnet-4-20250514".into(),
        llm_provider: LlmProvider::Anthropic,
        ollama_base_url: "http://127.0.0.1:11434".into(),
            ollama_model: "biomistral-hermes".into(),
        llm_disabled: true,
        llm_fallback_stub: true,
        deid: hermes::deid::DeidConfig {
            mode: hermes::deid::DeidMode::Rules,
            ner_url: "http://127.0.0.1:3001".into(),
            block_on_high_risk: false,
        },
        bind_host: "127.0.0.1".into(),
        bind_port: 8090,
        oidc: hermes::config::OidcConfig {
            enabled: false,
            issuer: String::new(),
            audience: "hermes-api".into(),
            jwks_url: String::new(),
            operator_groups: vec!["hermes-operator".into()],
            auditor_groups: vec!["hermes-auditor".into()],
            allow_role_key: true,
        },
    }
}

fn test_app(db_path: &std::path::Path) -> axum::Router {
    let config = test_config(db_path);
    let audit = Arc::new(AuditLog::open(db_path).unwrap());
    build_router(AppState {
        config,
        audit,
        jwks: None,
    })
}

async fn body_json(response: axum::response::Response) -> Value {
    let bytes = axum::body::to_bytes(response.into_body(), usize::MAX)
        .await
        .unwrap();
    serde_json::from_slice(&bytes).unwrap()
}

#[tokio::test]
async fn audit_hash_round_trip() {
    let tmp = NamedTempFile::new().unwrap();
    let log = AuditLog::open(tmp.path()).unwrap();
    let meta = serde_json::json!({"prompt_len": 10});
    let entry = log
        .append("operator", "operator", "inference", "vault-answer", "ok", &meta)
        .unwrap();
    assert!(log.verify_entry(entry.id).unwrap());
}

#[tokio::test]
async fn operator_denied_export() {
    let tmp = NamedTempFile::new().unwrap();
    let app = test_app(tmp.path());

    let response = app
        .oneshot(
            Request::builder()
                .uri("/v1/audit/export")
                .header("X-Role-Key", "change-me-operator")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::FORBIDDEN);
}

#[tokio::test]
async fn policy_blocks_ssn() {
    let tmp = NamedTempFile::new().unwrap();
    let app = test_app(tmp.path());

    let response = app
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/v1/inference")
                .header("X-Role-Key", "change-me-operator")
                .header("content-type", "application/json")
                .body(Body::from(
                    r#"{"prompt":"patient SSN 123-45-6789","skill":"vault-answer"}"#,
                ))
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::FORBIDDEN);
    let json = body_json(response).await;
    assert!(json["detail"]
        .as_str()
        .unwrap()
        .contains("phi_pattern_detected"));
}

#[tokio::test]
async fn stub_inference_succeeds() {
    let tmp = NamedTempFile::new().unwrap();
    let app = test_app(tmp.path());

    let response = app
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/v1/inference")
                .header("X-Role-Key", "change-me-operator")
                .header("content-type", "application/json")
                .body(Body::from(
                    r#"{"prompt":"de-identified clinical note summary","skill":"vault-answer"}"#,
                ))
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::OK);
    let json = body_json(response).await;
    assert!(json["output"].as_str().unwrap().contains("DEMO STUB"));
    assert!(json["entry_hash"].as_str().unwrap().len() == 64);
}

#[tokio::test]
async fn deid_scrubs_before_stub_inference() {
    let tmp = NamedTempFile::new().unwrap();
    let app = test_app(tmp.path());

    let prompt = "DE-IDENTIFIED DISCHARGE NOTE\nAge: 67 | Sex: F | MRN: ABC123\nAdmission: pneumonia.";
    let response = app
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/v1/inference")
                .header("X-Role-Key", "change-me-operator")
                .header("content-type", "application/json")
                .body(Body::from(format!(
                    r#"{{"prompt":{prompt},"skill":"vault-answer"}}"#,
                    prompt = serde_json::to_string(prompt).unwrap()
                )))
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::OK);
    let json = body_json(response).await;
    let deid = json["deidentified_prompt"].as_str().unwrap();
    assert!(deid.contains("[REDACTED-AGE]"));
    assert!(deid.contains("[REDACTED-MRN]"));
    assert!(json["deid_redaction_count"].as_u64().unwrap() >= 2);
}
