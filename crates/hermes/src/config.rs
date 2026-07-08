use secrecy::{ExposeSecret, Secret};
use std::path::PathBuf;
use tracing::warn;

use crate::deid::DeidConfig;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum HermesEnv {
    Local,
    Dev,
    Prod,
}

impl HermesEnv {
    pub fn parse(raw: &str) -> Result<Self, String> {
        match raw.to_lowercase().as_str() {
            "local" => Ok(Self::Local),
            "dev" => Ok(Self::Dev),
            "prod" | "production" => Ok(Self::Prod),
            other => Err(format!(
                "invalid HERMES_ENV: {other} (use local, dev, or prod)"
            )),
        }
    }

    pub fn as_str(self) -> &'static str {
        match self {
            Self::Local => "local",
            Self::Dev => "dev",
            Self::Prod => "prod",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LlmProvider {
    Anthropic,
    Ollama,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AuditBackend {
    Sqlite,
    Postgres,
}

impl AuditBackend {
    pub fn parse(raw: &str) -> Result<Self, String> {
        match raw.to_lowercase().as_str() {
            "sqlite" => Ok(Self::Sqlite),
            "postgres" | "postgresql" | "pg" => Ok(Self::Postgres),
            other => Err(format!(
                "invalid AUDIT_BACKEND: {other} (use sqlite or postgres)"
            )),
        }
    }

    pub fn as_str(self) -> &'static str {
        match self {
            Self::Sqlite => "sqlite",
            Self::Postgres => "postgres",
        }
    }
}

#[derive(Debug, Clone)]
pub struct AuditConfig {
    pub backend: AuditBackend,
    pub sqlite_path: PathBuf,
    pub database_url: Option<Secret<String>>,
}

impl AuditConfig {
    pub fn from_env(env: HermesEnv) -> Self {
        let sqlite_path = PathBuf::from(
            std::env::var("DATABASE_PATH").unwrap_or_else(|_| "data/hipaa_hermes.db".into()),
        );
        let database_url = std::env::var("DATABASE_URL")
            .ok()
            .filter(|s| !s.is_empty())
            .map(Secret::new);

        let backend = match std::env::var("AUDIT_BACKEND").as_deref() {
            Ok(raw) => AuditBackend::parse(raw).unwrap_or(AuditBackend::Sqlite),
            Err(_) => {
                if database_url.is_some() {
                    AuditBackend::Postgres
                } else {
                    AuditBackend::Sqlite
                }
            }
        };

        // Dev/prod default to Postgres when DATABASE_URL is set.
        let backend = match env {
            HermesEnv::Local => backend,
            HermesEnv::Dev | HermesEnv::Prod => {
                if database_url.is_some() {
                    AuditBackend::Postgres
                } else {
                    backend
                }
            }
        };

        Self {
            backend,
            sqlite_path,
            database_url,
        }
    }
}

#[derive(Debug, Clone)]
pub struct OidcConfig {
    pub enabled: bool,
    pub issuer: String,
    pub audience: String,
    pub jwks_url: String,
    pub operator_groups: Vec<String>,
    pub auditor_groups: Vec<String>,
    pub allow_role_key: bool,
}

impl OidcConfig {
    pub fn from_env(env: HermesEnv) -> Self {
        let enabled = std::env::var("OIDC_ENABLED")
            .map(|v| v == "1" || v.eq_ignore_ascii_case("true"))
            .unwrap_or(false);

        let issuer = std::env::var("OIDC_ISSUER").unwrap_or_default();
        let audience = std::env::var("OIDC_AUDIENCE").unwrap_or_else(|_| "hermes-api".into());
        let jwks_url = std::env::var("OIDC_JWKS_URL").unwrap_or_else(|_| {
            if issuer.is_empty() {
                String::new()
            } else {
                format!(
                    "{}/protocol/openid-connect/certs",
                    issuer.trim_end_matches('/')
                )
            }
        });

        let operator_groups = parse_csv(
            &std::env::var("OIDC_OPERATOR_GROUPS").unwrap_or_else(|_| "hermes-operator".into()),
        );
        let auditor_groups = parse_csv(
            &std::env::var("OIDC_AUDITOR_GROUPS").unwrap_or_else(|_| "hermes-auditor".into()),
        );

        let allow_role_key = match std::env::var("OIDC_ALLOW_ROLE_KEY").as_deref() {
            Ok("0") | Ok("false") => false,
            Ok("1") | Ok("true") => true,
            Ok(_) => true,
            Err(_) => matches!(env, HermesEnv::Local | HermesEnv::Dev),
        };

        Self {
            enabled,
            issuer,
            audience,
            jwks_url,
            operator_groups,
            auditor_groups,
            allow_role_key,
        }
    }
}

fn parse_csv(raw: &str) -> Vec<String> {
    raw.split(',')
        .map(str::trim)
        .filter(|s| !s.is_empty())
        .map(str::to_string)
        .collect()
}

#[derive(Clone)]
pub struct Config {
    pub env: HermesEnv,
    pub behind_proxy: bool,
    pub oidc: OidcConfig,
    pub audit: AuditConfig,
    pub admin_secret: Secret<String>,
    pub auditor_secret: Secret<String>,
    pub anthropic_api_key: Option<Secret<String>>,
    pub claude_model: String,
    pub llm_provider: LlmProvider,
    pub ollama_base_url: String,
    pub ollama_model: String,
    pub llm_disabled: bool,
    pub llm_fallback_stub: bool,
    pub deid: DeidConfig,
    pub bind_host: String,
    pub bind_port: u16,
    pub secrets_from_vault: bool,
}

fn is_weak_secret(secret: &Secret<String>) -> bool {
    let s = secret.expose_secret();
    s.is_empty()
        || s.starts_with("change-me")
        || s == "admin"
        || s == "password"
        || s.len() < 16
}

impl Config {
    pub fn from_env() -> Result<Self, String> {
        dotenvy::dotenv().ok();

        let env = HermesEnv::parse(
            &std::env::var("HERMES_ENV").unwrap_or_else(|_| "local".into()),
        )?;

        let behind_proxy = std::env::var("HERMES_BEHIND_PROXY")
            .map(|v| v == "1" || v.eq_ignore_ascii_case("true"))
            .unwrap_or(false);

        let llm_disabled = std::env::var("LLM_DISABLED")
            .map(|v| v == "1" || v.eq_ignore_ascii_case("true"))
            .unwrap_or(false);

        let llm_fallback_stub = match std::env::var("LLM_FALLBACK_STUB").as_deref() {
            Ok("0") | Ok("false") => false,
            Ok(_) => true,
            Err(_) => true,
        };

        let anthropic_api_key = std::env::var("ANTHROPIC_API_KEY")
            .ok()
            .filter(|k| !k.is_empty())
            .map(Secret::new);

        let llm_provider = match std::env::var("LLM_PROVIDER")
            .unwrap_or_else(|_| "ollama".into())
            .to_lowercase()
            .as_str()
        {
            "anthropic" => LlmProvider::Anthropic,
            "ollama" => LlmProvider::Ollama,
            "auto" => LlmProvider::Ollama,
            other => {
                return Err(format!(
                    "invalid LLM_PROVIDER: {other} (use ollama, anthropic, or auto)"
                ));
            }
        };

        let default_bind = if behind_proxy {
            "127.0.0.1"
        } else {
            "0.0.0.0"
        };

        let config = Self {
            env,
            behind_proxy,
            oidc: OidcConfig::from_env(env),
            audit: AuditConfig::from_env(env),
            admin_secret: Secret::new(
                std::env::var("ADMIN_SECRET").unwrap_or_else(|_| "change-me-operator".into()),
            ),
            auditor_secret: Secret::new(
                std::env::var("AUDITOR_SECRET").unwrap_or_else(|_| "change-me-auditor".into()),
            ),
            anthropic_api_key,
            claude_model: std::env::var("CLAUDE_MODEL")
                .unwrap_or_else(|_| "claude-sonnet-4-20250514".into()),
            llm_provider,
            ollama_base_url: std::env::var("OLLAMA_BASE_URL")
                .unwrap_or_else(|_| "http://127.0.0.1:11434".into()),
            ollama_model: std::env::var("OLLAMA_MODEL")
                .unwrap_or_else(|_| "biomistral-hermes".into()),
            llm_disabled,
            llm_fallback_stub,
            deid: DeidConfig::from_env(),
            bind_host: std::env::var("BIND_HOST").unwrap_or_else(|_| default_bind.into()),
            bind_port: std::env::var("BIND_PORT")
                .ok()
                .and_then(|p| p.parse().ok())
                .unwrap_or(8090),
            secrets_from_vault: std::env::var("VAULT_ENABLED")
                .map(|v| v == "1" || v.eq_ignore_ascii_case("true"))
                .unwrap_or(false),
        };

        config.validate()?;
        config.log_local_warnings();
        Ok(config)
    }

    /// Hard failures for dev/prod misconfiguration.
    pub fn validate(&self) -> Result<(), String> {
        if self.oidc.enabled {
            if self.oidc.issuer.is_empty() {
                return Err("OIDC_ENABLED=1 requires OIDC_ISSUER".into());
            }
            if self.oidc.jwks_url.is_empty() {
                return Err("OIDC_ENABLED=1 requires OIDC_JWKS_URL or OIDC_ISSUER".into());
            }
        }

        if matches!(self.env, HermesEnv::Dev | HermesEnv::Prod) {
            if self.audit.backend != AuditBackend::Postgres {
                return Err(
                    "HERMES_ENV=dev/prod: set AUDIT_BACKEND=postgres and DATABASE_URL".into(),
                );
            }
            if self.audit.database_url.is_none() {
                return Err("HERMES_ENV=dev/prod: DATABASE_URL is required for Postgres audit".into());
            }
        }

        match self.env {
            HermesEnv::Local => Ok(()),
            HermesEnv::Dev => {
                if is_weak_secret(&self.admin_secret) {
                    return Err(
                        "HERMES_ENV=dev: ADMIN_SECRET must not use default or weak values"
                            .into(),
                    );
                }
                if is_weak_secret(&self.auditor_secret) {
                    return Err(
                        "HERMES_ENV=dev: AUDITOR_SECRET must not use default or weak values"
                            .into(),
                    );
                }
                Ok(())
            }
            HermesEnv::Prod => {
                if is_weak_secret(&self.admin_secret) || is_weak_secret(&self.auditor_secret) {
                    return Err(
                        "HERMES_ENV=prod: set strong ADMIN_SECRET and AUDITOR_SECRET (16+ chars)"
                            .into(),
                    );
                }
                if self.bind_host == "0.0.0.0" && self.behind_proxy {
                    return Err(
                        "HERMES_ENV=prod: use BIND_HOST=127.0.0.1 when HERMES_BEHIND_PROXY=1"
                            .into(),
                    );
                }
                if self.bind_host == "0.0.0.0" && !self.behind_proxy {
                    return Err(
                        "HERMES_ENV=prod: do not bind 0.0.0.0 without TLS proxy (set HERMES_BEHIND_PROXY=1)"
                            .into(),
                    );
                }
                if self.llm_fallback_stub {
                    return Err(
                        "HERMES_ENV=prod: set LLM_FALLBACK_STUB=0 (no demo stubs in production)"
                            .into(),
                    );
                }
                if self.oidc.enabled && self.oidc.allow_role_key {
                    return Err(
                        "HERMES_ENV=prod: set OIDC_ALLOW_ROLE_KEY=0 when OIDC_ENABLED=1".into(),
                    );
                }
                Ok(())
            }
        }
    }

    fn log_local_warnings(&self) {
        if self.env != HermesEnv::Local {
            return;
        }
        if is_weak_secret(&self.admin_secret) {
            warn!("ADMIN_SECRET is a default dev value — fine for local demos only");
        }
        if is_weak_secret(&self.auditor_secret) {
            warn!("AUDITOR_SECRET is a default dev value — fine for local demos only");
        }
        if self.bind_host == "0.0.0.0" && !self.behind_proxy {
            warn!("API listening on 0.0.0.0 without TLS — OK for local; use HERMES_BEHIND_PROXY=1 for HTTPS");
        }
    }

    pub fn llm_available(&self) -> bool {
        if self.llm_disabled {
            return false;
        }
        match self.llm_provider {
            LlmProvider::Anthropic => self.anthropic_api_key.is_some(),
            LlmProvider::Ollama => true,
        }
    }

    pub fn model_label(&self) -> String {
        match self.llm_provider {
            LlmProvider::Anthropic => self.claude_model.clone(),
            LlmProvider::Ollama => self.ollama_model.clone(),
        }
    }

    pub fn anthropic_key(&self) -> Option<&str> {
        self.anthropic_api_key
            .as_ref()
            .map(|k| k.expose_secret().as_str())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn base_config(env: HermesEnv) -> Config {
        Config {
            env,
            behind_proxy: false,
            oidc: OidcConfig {
                enabled: false,
                issuer: String::new(),
                audience: "hermes-api".into(),
                jwks_url: String::new(),
                operator_groups: vec!["hermes-operator".into()],
                auditor_groups: vec!["hermes-auditor".into()],
                allow_role_key: true,
            },
            audit: AuditConfig {
                backend: AuditBackend::Sqlite,
                sqlite_path: "data/test.db".into(),
                database_url: None,
            },
            admin_secret: Secret::new("change-me-operator".into()),
            auditor_secret: Secret::new("change-me-auditor".into()),
            anthropic_api_key: None,
            claude_model: "claude-sonnet-4-20250514".into(),
            llm_provider: LlmProvider::Ollama,
            ollama_base_url: "http://127.0.0.1:11434".into(),
            ollama_model: "biomistral-hermes".into(),
            llm_disabled: false,
            llm_fallback_stub: true,
            deid: DeidConfig {
                mode: crate::deid::DeidMode::Rules,
                ner_url: "http://127.0.0.1:3001".into(),
                block_on_high_risk: false,
            },
            bind_host: "0.0.0.0".into(),
            bind_port: 8090,
            secrets_from_vault: false,
        }
    }

    #[test]
    fn local_allows_default_secrets() {
        assert!(base_config(HermesEnv::Local).validate().is_ok());
    }

    #[test]
    fn dev_rejects_default_secrets() {
        assert!(base_config(HermesEnv::Dev).validate().is_err());
    }

    fn prod_audit_config() -> AuditConfig {
        AuditConfig {
            backend: AuditBackend::Postgres,
            sqlite_path: "data/test.db".into(),
            database_url: Some(Secret::new(
                "postgres://hermes:hermes@127.0.0.1:5432/hermes_audit".into(),
            )),
        }
    }

    #[test]
    fn prod_requires_proxy_and_strong_secrets() {
        let mut c = base_config(HermesEnv::Prod);
        c.admin_secret = Secret::new("x".repeat(24));
        c.auditor_secret = Secret::new("y".repeat(24));
        c.llm_fallback_stub = false;
        c.audit = prod_audit_config();
        assert!(c.validate().is_err());

        c.behind_proxy = true;
        c.bind_host = "127.0.0.1".into();
        assert!(c.validate().is_ok());
    }

    #[test]
    fn prod_rejects_oidc_with_role_key_fallback() {
        let mut c = base_config(HermesEnv::Prod);
        c.admin_secret = Secret::new("x".repeat(24));
        c.auditor_secret = Secret::new("y".repeat(24));
        c.llm_fallback_stub = false;
        c.behind_proxy = true;
        c.bind_host = "127.0.0.1".into();
        c.audit = prod_audit_config();
        c.oidc = OidcConfig {
            enabled: true,
            issuer: "http://issuer/realms/hermes".into(),
            audience: "hermes-api".into(),
            jwks_url: "http://issuer/certs".into(),
            operator_groups: vec!["hermes-operator".into()],
            auditor_groups: vec!["hermes-auditor".into()],
            allow_role_key: true,
        };
        assert!(c.validate().is_err());
        c.oidc.allow_role_key = false;
        assert!(c.validate().is_ok());
    }

    #[test]
    fn dev_requires_postgres_audit() {
        let mut c = base_config(HermesEnv::Dev);
        c.admin_secret = Secret::new("x".repeat(24));
        c.auditor_secret = Secret::new("y".repeat(24));
        assert!(c.validate().is_err());
        c.audit = prod_audit_config();
        assert!(c.validate().is_ok());
    }
}
