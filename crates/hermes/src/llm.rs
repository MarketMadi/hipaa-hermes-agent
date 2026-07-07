use lazy_static::lazy_static;
use regex::Regex;
use serde::{Deserialize, Serialize};
use std::time::Duration;
use thiserror::Error;

use crate::config::{Config, LlmProvider};

const ANTHROPIC_API_URL: &str = "https://api.anthropic.com/v1/messages";
const ANTHROPIC_VERSION: &str = "2023-06-01";

lazy_static! {
    static ref HTTP: ureq::Agent = ureq::AgentBuilder::new()
        .timeout_connect(Duration::from_secs(10))
        .timeout_read(Duration::from_secs(120))
        .build();
}

#[derive(Debug, Error)]
pub enum LlmError {
    #[error("LLM disabled or API key not configured")]
    NotConfigured,
    #[error("http error: {0}")]
    Http(String),
    #[error("api error ({status}): {body}")]
    Api { status: u16, body: String },
    #[error("empty response from model")]
    EmptyResponse,
}

impl LlmError {
    /// Billing / connectivity errors — safe to fall back to demo stub for local sales demos.
    pub fn eligible_for_stub_fallback(&self) -> bool {
        let text = match self {
            LlmError::Api { body, .. } => body.as_str(),
            LlmError::Http(msg) => msg.as_str(),
            _ => return false,
        };
        let b = text.to_lowercase();
        b.contains("credit balance")
            || b.contains("billing")
            || b.contains("quota")
            || b.contains("insufficient")
            || b.contains("connection refused")
            || b.contains("connection failed")
            || b.contains("failed to connect")
            || b.contains("timed out")
            || b.contains("local model refused")
    }
}

pub async fn complete(
    config: &Config,
    prompt: &str,
    skill: &str,
) -> Result<String, LlmError> {
    match config.llm_provider {
        LlmProvider::Anthropic => complete_anthropic(config, prompt, skill).await,
        LlmProvider::Ollama => complete_ollama(config, prompt, skill).await,
    }
}

async fn complete_anthropic(
    config: &Config,
    prompt: &str,
    skill: &str,
) -> Result<String, LlmError> {
    let api_key = config.anthropic_key().ok_or(LlmError::NotConfigured)?.to_string();
    let model = config.claude_model.clone();
    let prompt = prompt.to_string();
    let skill = skill.to_string();

    tokio::task::spawn_blocking(move || anthropic_blocking(&api_key, &model, &prompt, &skill))
        .await
        .map_err(|e| LlmError::Http(e.to_string()))?
}

async fn complete_ollama(
    config: &Config,
    prompt: &str,
    skill: &str,
) -> Result<String, LlmError> {
    let base_url = config.ollama_base_url.clone();
    let model = config.ollama_model.clone();
    let prompt = prompt.to_string();
    let skill = skill.to_string();

    tokio::task::spawn_blocking(move || ollama_blocking(&base_url, &model, &prompt, &skill))
        .await
        .map_err(|e| LlmError::Http(e.to_string()))?
}

#[derive(Serialize)]
struct MessagesRequest<'a> {
    model: &'a str,
    max_tokens: u32,
    system: &'a str,
    messages: Vec<Message<'a>>,
}

#[derive(Serialize)]
struct Message<'a> {
    role: &'a str,
    content: &'a str,
}

#[derive(Deserialize)]
struct MessagesResponse {
    content: Vec<ContentBlock>,
}

#[derive(Deserialize)]
struct ContentBlock {
    #[serde(rename = "type")]
    block_type: String,
    text: Option<String>,
}

fn system_prompt(skill: &str, provider: LlmProvider) -> String {
    match provider {
        LlmProvider::Anthropic => format!(
            "You are a HIPAA-aligned clinical assistant skill '{skill}'. \
             Respond concisely. The user prompt is assumed de-identified upstream."
        ),
        LlmProvider::Ollama => String::new(), // Ollama uses facts-only user prompts — no system role
    }
}

/// llama3.2:1b refuses clinical document framing. Extract neutral bullet facts instead.
fn parse_context_question(prompt: &str) -> (String, String) {
    if let Some((ctx, q)) = prompt.split_once("\n\nQUESTION:\n") {
        let ctx = ctx.strip_prefix("CONTEXT:\n").unwrap_or(ctx);
        return (ctx.trim().to_string(), q.trim().to_string());
    }
    (prompt.trim().to_string(), "Summarize in plain English.".into())
}

fn strip_redaction_tokens(text: &str) -> String {
    lazy_static! {
        static ref REDACTED: Regex = Regex::new(r"\[REDACTED-[A-Z]+\]").unwrap();
        static ref HEADER: Regex =
            Regex::new(r"(?i)^DE-IDENTIFIED\s+(DISCHARGE NOTE|LAB PANEL)\s*$").unwrap();
    }
    let mut out = String::new();
    for line in text.lines() {
        let line = HEADER.replace_all(line, "Practice case").into_owned();
        let line = REDACTED.replace_all(&line, "").into_owned();
        let line = line.trim().trim_matches('|').trim();
        if line.is_empty() {
            continue;
        }
        out.push_str(line);
        out.push('\n');
    }
    out.trim().to_string()
}

fn soften_task(task: &str) -> String {
    task.replace("plain-language summary", "simple summary")
        .replace("Plain-language summary", "Simple summary")
        .replace("clinician", "reader")
        .replace("Clinician", "Reader")
        .replace("patient and family", "a general audience")
        .replace("critical value", "notable lab result")
}

fn ollama_facts_prompt(prompt: &str) -> String {
    let (context, task) = parse_context_question(prompt);
    let context = strip_redaction_tokens(&context);
    let task = soften_task(&task);

    let bullets: Vec<String> = context
        .lines()
        .map(str::trim)
        .filter(|l| !l.is_empty())
        .map(|l| format!("- {l}"))
        .collect();

    format!(
        "Writing class exercise (fictional practice material).\n\
         Task: {task}\n\n\
         Facts:\n{}\n\n\
         Complete the task using only the facts above. Be concise.",
        bullets.join("\n")
    )
}

fn ollama_minimal_prompt(prompt: &str) -> String {
    let (context, _) = parse_context_question(prompt);
    let context = strip_redaction_tokens(&context);
    format!(
        "Turn the following practice notes into 3 short simple sentences:\n\n{context}"
    )
}

fn looks_like_refusal(text: &str) -> bool {
    let t = text.to_lowercase();
    t.contains("can i help you with something else")
        || t.contains("is there anything else i can help")
        || t.contains("can't assist")
        || t.contains("cannot assist")
        || t.contains("cannot provide")
        || t.contains("can't provide")
        || t.contains("sensitive")
        || t.contains("private medical")
        || t.contains("medical diagnosis")
        || t.contains("medical information")
        || t.contains("healthcare information")
        || t.contains("i'm not able")
        || t.contains("i am not able")
}

fn anthropic_blocking(
    api_key: &str,
    model: &str,
    prompt: &str,
    skill: &str,
) -> Result<String, LlmError> {
    let system = system_prompt(skill, LlmProvider::Anthropic);

    let body = MessagesRequest {
        model,
        max_tokens: 1024,
        system: &system,
        messages: vec![Message {
            role: "user",
            content: prompt,
        }],
    };

    let response = match HTTP.post(ANTHROPIC_API_URL)
        .set("x-api-key", api_key)
        .set("anthropic-version", ANTHROPIC_VERSION)
        .set("content-type", "application/json")
        .send_json(&body)
    {
        Ok(resp) => resp,
        Err(ureq::Error::Status(status, resp)) => {
            let body = resp.into_string().unwrap_or_default();
            return Err(parse_anthropic_error(status, &body));
        }
        Err(e) => return Err(LlmError::Http(e.to_string())),
    };

    let status = response.status();
    if status != 200 {
        let body = response.into_string().unwrap_or_default();
        return Err(parse_anthropic_error(status, &body));
    }

    let parsed: MessagesResponse = response
        .into_json()
        .map_err(|e| LlmError::Http(e.to_string()))?;

    parsed
        .content
        .into_iter()
        .find(|b| b.block_type == "text")
        .and_then(|b| b.text)
        .filter(|t| !t.is_empty())
        .ok_or(LlmError::EmptyResponse)
}

#[derive(Serialize)]
struct OllamaChatRequest<'a> {
    model: &'a str,
    messages: Vec<OllamaMessage<'a>>,
    stream: bool,
    options: OllamaOptions,
}

#[derive(Serialize)]
struct OllamaOptions {
    temperature: f32,
    num_predict: i32,
}

#[derive(Serialize)]
struct OllamaMessage<'a> {
    role: &'a str,
    content: &'a str,
}

#[derive(Deserialize)]
struct OllamaChatResponse {
    message: OllamaMessageOut,
}

#[derive(Deserialize)]
struct OllamaMessageOut {
    content: String,
}

fn ollama_blocking(
    base_url: &str,
    model: &str,
    prompt: &str,
    _skill: &str,
) -> Result<String, LlmError> {
    let user = ollama_facts_prompt(prompt);
    let mut out = ollama_chat(base_url, model, "", &user)?;

    if looks_like_refusal(&out) {
        let retry = ollama_minimal_prompt(prompt);
        out = ollama_chat(base_url, model, "", &retry)?;
    }

    if looks_like_refusal(&out) {
        return Err(LlmError::Api {
            status: 422,
            body: "local model refused fictional demo content — try restarting Ollama or set LLM_PROVIDER=anthropic".into(),
        });
    }

    Ok(out)
}

fn ollama_chat(
    base_url: &str,
    model: &str,
    system: &str,
    user: &str,
) -> Result<String, LlmError> {
    let url = format!("{}/api/chat", base_url.trim_end_matches('/'));

    let mut messages = Vec::new();
    if !system.is_empty() {
        messages.push(OllamaMessage {
            role: "system",
            content: system,
        });
    }
    messages.push(OllamaMessage {
        role: "user",
        content: user,
    });

    let body = OllamaChatRequest {
        model,
        messages,
        stream: false,
        options: OllamaOptions {
            temperature: 0.3,
            num_predict: 1024,
        },
    };

    let response = match HTTP.post(&url)
        .set("content-type", "application/json")
        .send_json(&body)
    {
        Ok(resp) => resp,
        Err(ureq::Error::Status(status, resp)) => {
            let body = resp.into_string().unwrap_or_default();
            return Err(LlmError::Api { status, body });
        }
        Err(e) => return Err(LlmError::Http(e.to_string())),
    };

    let status = response.status();
    if status != 200 {
        let body = response.into_string().unwrap_or_default();
        return Err(LlmError::Api { status, body });
    }

    let parsed: OllamaChatResponse = response
        .into_json()
        .map_err(|e| LlmError::Http(e.to_string()))?;

    if parsed.message.content.is_empty() {
        return Err(LlmError::EmptyResponse);
    }

    Ok(parsed.message.content)
}

#[derive(Deserialize)]
struct AnthropicErrorBody {
    error: Option<AnthropicErrorDetail>,
}

#[derive(Deserialize)]
struct AnthropicErrorDetail {
    message: Option<String>,
}

fn parse_anthropic_error(status: u16, body: &str) -> LlmError {
    let message = serde_json::from_str::<AnthropicErrorBody>(body)
        .ok()
        .and_then(|e| e.error)
        .and_then(|e| e.message)
        .unwrap_or_else(|| body.to_string());
    LlmError::Api {
        status,
        body: message,
    }
}
