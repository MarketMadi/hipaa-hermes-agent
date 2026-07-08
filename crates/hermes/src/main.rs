use hermes::{build_router, config::Config, oidc, AppState};
use std::sync::Arc;
use tracing::info;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| "hermes=info,tower_http=info".into()),
        )
        .init();

    let config = Config::from_env().map_err(|e| format!("config error: {e}"))?;
    let audit = Arc::new(hermes::audit::AuditLog::open(&config.audit).await?);
    hermes::metrics::MetricsRegistry::refresh_audit_gauges(&audit).await;

    let jwks = if config.oidc.enabled {
        Some(Arc::new(oidc::JwksCache::new(config.oidc.jwks_url.clone())))
    } else {
        None
    };

    let state = AppState {
        config: config.clone(),
        audit,
        jwks,
    };

    let llm_status = if config.llm_available() {
        config.model_label()
    } else {
        "stub (run ./scripts/setup-biomistral.sh or set LLM_PROVIDER=anthropic)".into()
    };
    info!(env = %config.env.as_str(), %llm_status, "HIPAA Hermes starting");

    let app = build_router(state);
    let addr = format!("{}:{}", config.bind_host, config.bind_port);
    let listener = tokio::net::TcpListener::bind(&addr).await?;
    info!(%addr, "listening");
    axum::serve(listener, app).await?;
    Ok(())
}
