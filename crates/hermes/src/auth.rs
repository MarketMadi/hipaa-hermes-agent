use axum::{
    async_trait,
    extract::FromRequestParts,
    http::{request::Parts, StatusCode},
    response::{IntoResponse, Response},
    Json,
};
use secrecy::ExposeSecret;
use serde_json::json;
use std::sync::Arc;

use crate::config::Config;
use crate::oidc::{self, JwksCache};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Role {
    Operator,
    Auditor,
}

impl Role {
    pub fn as_str(self) -> &'static str {
        match self {
            Role::Operator => "operator",
            Role::Auditor => "auditor",
        }
    }
}

#[derive(Debug, Clone)]
pub struct AuthContext {
    pub role: Role,
    pub actor: String,
}

pub struct AuthRejection {
    pub status: StatusCode,
    pub detail: String,
}

impl IntoResponse for AuthRejection {
    fn into_response(self) -> Response {
        if self.status == StatusCode::UNAUTHORIZED || self.status == StatusCode::FORBIDDEN {
            crate::metrics::MetricsRegistry::record_auth_failure(&self.status.as_u16().to_string());
        }
        (self.status, Json(json!({ "detail": self.detail }))).into_response()
    }
}

pub fn authorize_role_key(
    config: &Config,
    role_key: Option<&str>,
    allowed: &[Role],
) -> Result<AuthContext, AuthRejection> {
    let key = role_key.filter(|k| !k.is_empty()).ok_or(AuthRejection {
        status: StatusCode::UNAUTHORIZED,
        detail: "missing X-Role-Key".into(),
    })?;

    let op = config.admin_secret.expose_secret();
    let aud = config.auditor_secret.expose_secret();

    if key == op && allowed.contains(&Role::Operator) {
        return Ok(AuthContext {
            role: Role::Operator,
            actor: "operator".into(),
        });
    }
    if key == aud && allowed.contains(&Role::Auditor) {
        return Ok(AuthContext {
            role: Role::Auditor,
            actor: "auditor".into(),
        });
    }
    if key == op || key == aud {
        let need: Vec<&str> = allowed.iter().map(|r| r.as_str()).collect();
        return Err(AuthRejection {
            status: StatusCode::FORBIDDEN,
            detail: format!("role not permitted; need {need:?}"),
        });
    }

    Err(AuthRejection {
        status: StatusCode::UNAUTHORIZED,
        detail: "invalid X-Role-Key".into(),
    })
}

pub async fn authenticate(
    config: &Config,
    jwks: Option<&Arc<JwksCache>>,
    bearer: Option<&str>,
    role_key: Option<&str>,
    allowed: &[Role],
) -> Result<AuthContext, AuthRejection> {
    if config.oidc.enabled {
        if let Some(token) = bearer {
            let cache = jwks.ok_or_else(|| AuthRejection {
                status: StatusCode::INTERNAL_SERVER_ERROR,
                detail: "OIDC enabled but JWKS cache not initialized".into(),
            })?;
            return cache.validate(token, &config.oidc, allowed).await;
        }

        if config.oidc.allow_role_key {
            if let Ok(ctx) = authorize_role_key(config, role_key, allowed) {
                return Ok(ctx);
            }
        }

        return Err(AuthRejection {
            status: StatusCode::UNAUTHORIZED,
            detail: "missing or invalid Authorization Bearer token".into(),
        });
    }

    authorize_role_key(config, role_key, allowed)
}

pub struct OperatorAuth(pub AuthContext);
pub struct AuditorAuth(pub AuthContext);
pub struct EitherAuth(pub AuthContext);

async fn extract_auth(
    parts: &mut Parts,
    state: &crate::AppState,
    allowed: &[Role],
) -> Result<AuthContext, AuthRejection> {
    let bearer = parts
        .headers
        .get(axum::http::header::AUTHORIZATION)
        .and_then(|v| v.to_str().ok())
        .and_then(oidc::parse_bearer);
    let role_key = parts
        .headers
        .get("X-Role-Key")
        .and_then(|v| v.to_str().ok());

    authenticate(
        &state.config,
        state.jwks.as_ref(),
        bearer,
        role_key,
        allowed,
    )
    .await
}

macro_rules! auth_extractor {
    ($name:ident, $allowed:expr) => {
        #[async_trait]
        impl FromRequestParts<crate::AppState> for $name {
            type Rejection = AuthRejection;

            async fn from_request_parts(
                parts: &mut Parts,
                state: &crate::AppState,
            ) -> Result<Self, Self::Rejection> {
                let ctx = extract_auth(parts, state, $allowed).await?;
                Ok($name(ctx))
            }
        }
    };
}

auth_extractor!(OperatorAuth, &[Role::Operator]);
auth_extractor!(AuditorAuth, &[Role::Auditor]);
auth_extractor!(EitherAuth, &[Role::Operator, Role::Auditor]);
