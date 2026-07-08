use jsonwebtoken::jwk::{AlgorithmParameters, JwkSet};
use jsonwebtoken::{decode, decode_header, Algorithm, DecodingKey, Validation};
use serde::Deserialize;
use std::time::{Duration, Instant};
use tokio::sync::Mutex;

use crate::auth::{AuthContext, AuthRejection, Role};
use crate::config::OidcConfig;

#[derive(Debug)]
pub struct JwksCache {
    url: String,
    inner: Mutex<CachedJwks>,
}

#[derive(Debug, Clone)]
struct CachedJwks {
    fetched_at: Option<Instant>,
    keys: JwkSet,
}

const JWKS_TTL: Duration = Duration::from_secs(300);

impl JwksCache {
    pub fn new(url: String) -> Self {
        Self {
            url,
            inner: Mutex::new(CachedJwks {
                fetched_at: None,
                keys: JwkSet { keys: vec![] },
            }),
        }
    }

    pub async fn validate(
        &self,
        token: &str,
        config: &OidcConfig,
        allowed: &[Role],
    ) -> Result<AuthContext, AuthRejection> {
        let header = decode_header(token).map_err(|e| AuthRejection {
            status: axum::http::StatusCode::UNAUTHORIZED,
            detail: format!("invalid JWT header: {e}"),
        })?;

        let kid = header.kid.ok_or_else(|| AuthRejection {
            status: axum::http::StatusCode::UNAUTHORIZED,
            detail: "JWT missing kid".into(),
        })?;

        let decoding_key = self.decoding_key(&kid).await?;

        let mut validation = Validation::new(Algorithm::RS256);
        validation.set_issuer(&[&config.issuer]);
        validation.set_audience(&[config.audience.as_str()]);

        let token_data = decode::<Claims>(token, &decoding_key, &validation).map_err(|e| {
            AuthRejection {
                status: axum::http::StatusCode::UNAUTHORIZED,
                detail: format!("JWT validation failed: {e}"),
            }
        })?;

        let groups = groups_from_claims(&token_data.claims);
        let role = role_from_groups(&groups, config).ok_or_else(|| AuthRejection {
            status: axum::http::StatusCode::FORBIDDEN,
            detail: format!(
                "no permitted OIDC group; need one of operator {:?} or auditor {:?}",
                config.operator_groups, config.auditor_groups
            ),
        })?;

        if !allowed.contains(&role) {
            return Err(AuthRejection {
                status: axum::http::StatusCode::FORBIDDEN,
                detail: format!(
                    "role {} not permitted; need {:?}",
                    role.as_str(),
                    allowed.iter().map(|r| r.as_str()).collect::<Vec<_>>()
                ),
            });
        }

        let actor = actor_from_claims(&token_data.claims);
        Ok(AuthContext { role, actor })
    }

    async fn decoding_key(&self, kid: &str) -> Result<DecodingKey, AuthRejection> {
        let keys = self.jwks().await?;
        let jwk = keys.keys.iter().find(|k| k.common.key_id.as_deref() == Some(kid));
        let jwk = jwk.ok_or_else(|| AuthRejection {
            status: axum::http::StatusCode::UNAUTHORIZED,
            detail: format!("no JWK for kid {kid}"),
        })?;
        match &jwk.algorithm {
            AlgorithmParameters::RSA(_) => DecodingKey::from_jwk(jwk).map_err(|e| AuthRejection {
                status: axum::http::StatusCode::INTERNAL_SERVER_ERROR,
                detail: format!("JWK parse error: {e}"),
            }),
            _ => Err(AuthRejection {
                status: axum::http::StatusCode::UNAUTHORIZED,
                detail: "unsupported JWK algorithm (expected RSA)".into(),
            }),
        }
    }

    async fn jwks(&self) -> Result<JwkSet, AuthRejection> {
        let mut guard = self.inner.lock().await;
        let stale = guard
            .fetched_at
            .map(|t| t.elapsed() > JWKS_TTL)
            .unwrap_or(true);

        if stale {
            let fetched: JwkSet = tokio::task::spawn_blocking({
                let url = self.url.clone();
                move || {
                    ureq::get(&url)
                        .call()
                        .map_err(|e| format!("JWKS fetch failed: {e}"))?
                        .into_json()
                        .map_err(|e| format!("JWKS parse failed: {e}"))
                }
            })
            .await
            .map_err(|e| AuthRejection {
                status: axum::http::StatusCode::INTERNAL_SERVER_ERROR,
                detail: format!("JWKS task failed: {e}"),
            })?
            .map_err(|e| AuthRejection {
                status: axum::http::StatusCode::SERVICE_UNAVAILABLE,
                detail: e,
            })?;

            guard.keys = fetched;
            guard.fetched_at = Some(Instant::now());
        }

        Ok(guard.keys.clone())
    }
}

#[derive(Debug, Deserialize)]
struct RealmAccess {
    #[serde(default)]
    roles: Vec<String>,
}

#[derive(Debug, Deserialize)]
struct Claims {
    sub: String,
    #[serde(default)]
    email: Option<String>,
    #[serde(default)]
    preferred_username: Option<String>,
    #[serde(default)]
    groups: Vec<String>,
    #[serde(default)]
    realm_access: Option<RealmAccess>,
}

fn groups_from_claims(claims: &Claims) -> Vec<String> {
    let mut groups = claims.groups.clone();
    if let Some(ra) = &claims.realm_access {
        groups.extend(ra.roles.clone());
    }
    groups
}

fn role_from_groups(groups: &[String], config: &OidcConfig) -> Option<Role> {
    if groups
        .iter()
        .any(|g| config.operator_groups.iter().any(|o| o == g))
    {
        return Some(Role::Operator);
    }
    if groups
        .iter()
        .any(|g| config.auditor_groups.iter().any(|a| a == g))
    {
        return Some(Role::Auditor);
    }
    None
}

fn actor_from_claims(claims: &Claims) -> String {
    claims
        .email
        .clone()
        .or_else(|| claims.preferred_username.clone())
        .unwrap_or_else(|| claims.sub.clone())
}

pub fn parse_bearer(header: &str) -> Option<&str> {
    let header = header.trim();
    header
        .strip_prefix("Bearer ")
        .or_else(|| header.strip_prefix("bearer "))
        .map(str::trim)
        .filter(|s| !s.is_empty())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::OidcConfig;

    #[test]
    fn parse_bearer_header() {
        assert_eq!(parse_bearer("Bearer abc"), Some("abc"));
        assert_eq!(parse_bearer("bearer xyz"), Some("xyz"));
        assert_eq!(parse_bearer("Basic x"), None);
    }

    #[test]
    fn maps_keycloak_realm_roles() {
        let claims = Claims {
            sub: "u1".into(),
            email: Some("op@hospital.test".into()),
            preferred_username: None,
            groups: vec![],
            realm_access: Some(RealmAccess {
                roles: vec!["hermes-operator".into()],
            }),
        };
        let config = OidcConfig {
            enabled: true,
            issuer: "http://issuer".into(),
            audience: "hermes-api".into(),
            jwks_url: "http://jwks".into(),
            operator_groups: vec!["hermes-operator".into()],
            auditor_groups: vec!["hermes-auditor".into()],
            allow_role_key: true,
        };
        assert_eq!(
            role_from_groups(&groups_from_claims(&claims), &config),
            Some(Role::Operator)
        );
        assert_eq!(actor_from_claims(&claims), "op@hospital.test");
    }
}
