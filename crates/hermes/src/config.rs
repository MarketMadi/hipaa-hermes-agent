use secrecy::{ExposeSecret, Secret};
use std::path::PathBuf;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LlmProvider {
    Anthropic,
    Ollama,
}

#[derive(Clone)]
pub struct Config {
    pub database_path: PathBuf,
    pub admin_secret: Secret<String>,
    pub auditor_secret: Secret<String>,
    pub anthropic_api_key: Option<Secret<String>>,
    pub claude_model: String,
    pub llm_provider: LlmProvider,
    pub ollama_base_url: String,
    pub ollama_model: String,
    pub llm_disabled: bool,
    pub llm_fallback_stub: bool,
    pub bind_host: String,
    pub bind_port: u16,
}

impl Config {
    pub fn from_env() -> Result<Self, String> {
        dotenvy::dotenv().ok();

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
            // local-first: auto prefers on-prem Ollama unless cloud is explicitly configured
            "auto" => LlmProvider::Ollama,
            other => {
                return Err(format!(
                    "invalid LLM_PROVIDER: {other} (use ollama, anthropic, or auto)"
                ));
            }
        };

        Ok(Self {
            database_path: PathBuf::from(
                std::env::var("DATABASE_PATH").unwrap_or_else(|_| "data/hipaa_hermes.db".into()),
            ),
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
                .unwrap_or_else(|_| "llama3.2:1b".into()),
            llm_disabled,
            llm_fallback_stub,
            bind_host: std::env::var("BIND_HOST").unwrap_or_else(|_| "0.0.0.0".into()),
            bind_port: std::env::var("BIND_PORT")
                .ok()
                .and_then(|p| p.parse().ok())
                .unwrap_or(8090),
        })
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
            LlmProvider::Ollama => format!("ollama/{}", self.ollama_model),
        }
    }

    pub fn anthropic_key(&self) -> Option<&str> {
        self.anthropic_api_key
            .as_ref()
            .map(|k| k.expose_secret().as_str())
    }
}
