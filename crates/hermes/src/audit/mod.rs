mod postgres;
mod sqlite;

use secrecy::ExposeSecret;
use serde::{Deserialize, Serialize};
use serde_json::{Map, Value};
use sha2::{Digest, Sha256};
use std::collections::BTreeMap;
use std::path::Path;
use std::sync::Arc;
use thiserror::Error;

pub use crate::config::{AuditBackend, AuditConfig};

#[derive(Debug, Error)]
pub enum AuditError {
    #[error("sqlite error: {0}")]
    Sqlite(#[from] rusqlite::Error),
    #[error("postgres error: {0}")]
    Postgres(#[from] sqlx::Error),
    #[error("json error: {0}")]
    Json(#[from] serde_json::Error),
    #[error("task join error: {0}")]
    Join(#[from] tokio::task::JoinError),
    #[error("configuration error: {0}")]
    Config(String),
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AuditEntry {
    pub id: i64,
    pub ts: String,
    pub actor: String,
    pub role: String,
    pub action: String,
    pub resource: String,
    pub outcome: String,
    pub metadata: Value,
    pub entry_hash: String,
}

#[derive(Debug, Clone)]
pub struct AuditMetrics {
    pub total_entries: i64,
    pub failure_count: i64,
    pub by_action: BTreeMap<String, i64>,
}

enum AuditStore {
    Sqlite(Arc<sqlite::SqliteAuditStore>),
    Postgres(postgres::PostgresAuditStore),
}

pub struct AuditLog {
    backend: AuditBackend,
    store: AuditStore,
}

impl AuditLog {
    pub async fn open(config: &AuditConfig) -> Result<Self, AuditError> {
        let store = match config.backend {
            AuditBackend::Sqlite => AuditStore::Sqlite(Arc::new(sqlite::SqliteAuditStore::open(
                &config.sqlite_path,
            )?)),
            AuditBackend::Postgres => {
                let url = config.database_url.as_ref().ok_or_else(|| {
                    AuditError::Config("DATABASE_URL required for postgres audit backend".into())
                })?;
                AuditStore::Postgres(
                    postgres::PostgresAuditStore::connect(url.expose_secret()).await?,
                )
            }
        };
        Ok(Self {
            backend: config.backend,
            store,
        })
    }

    pub fn backend_label(&self) -> &'static str {
        match self.backend {
            AuditBackend::Sqlite => "sqlite",
            AuditBackend::Postgres => "postgres",
        }
    }

    pub async fn append(
        &self,
        actor: &str,
        role: &str,
        action: &str,
        resource: &str,
        outcome: &str,
        metadata: &Value,
    ) -> Result<AuditEntry, AuditError> {
        match &self.store {
            AuditStore::Sqlite(store) => {
                let store = Arc::clone(store);
                let actor = actor.to_string();
                let role = role.to_string();
                let action = action.to_string();
                let resource = resource.to_string();
                let outcome = outcome.to_string();
                let metadata = metadata.clone();
                tokio::task::spawn_blocking(move || {
                    store.append(
                        &actor,
                        &role,
                        &action,
                        &resource,
                        &outcome,
                        &metadata,
                    )
                })
                .await?
            }
            AuditStore::Postgres(store) => {
                store
                    .append(actor, role, action, resource, outcome, metadata)
                    .await
            }
        }
    }

    pub async fn list_entries(&self, limit: i64) -> Result<Vec<AuditEntry>, AuditError> {
        match &self.store {
            AuditStore::Sqlite(store) => {
                let store = Arc::clone(store);
                tokio::task::spawn_blocking(move || store.list_entries(limit)).await?
            }
            AuditStore::Postgres(store) => store.list_entries(limit).await,
        }
    }

    pub async fn verify_entry(&self, entry_id: i64) -> Result<bool, AuditError> {
        match &self.store {
            AuditStore::Sqlite(store) => {
                let store = Arc::clone(store);
                tokio::task::spawn_blocking(move || store.verify_entry(entry_id)).await?
            }
            AuditStore::Postgres(store) => store.verify_entry(entry_id).await,
        }
    }

    pub async fn metrics(&self) -> Result<AuditMetrics, AuditError> {
        match &self.store {
            AuditStore::Sqlite(store) => {
                let store = Arc::clone(store);
                tokio::task::spawn_blocking(move || store.metrics()).await?
            }
            AuditStore::Postgres(store) => store.metrics().await,
        }
    }
}

pub async fn migrate_sqlite_to_postgres(
    sqlite_path: impl AsRef<Path>,
    database_url: &str,
) -> Result<usize, AuditError> {
    let sqlite = sqlite::SqliteAuditStore::open(sqlite_path)?;
    let postgres = postgres::PostgresAuditStore::connect(database_url).await?;
    let entries = sqlite.list_all_entries()?;
    let mut migrated = 0usize;
    for entry in entries {
        if postgres.insert_migrated(&entry).await? {
            migrated += 1;
        }
    }
    postgres.sync_sequence().await?;
    Ok(migrated)
}

pub(crate) fn hash_entry(
    ts: &str,
    actor: &str,
    role: &str,
    action: &str,
    resource: &str,
    outcome: &str,
    metadata: &Value,
) -> Result<String, AuditError> {
    let mut map = Map::new();
    map.insert("ts".into(), Value::String(ts.into()));
    map.insert("actor".into(), Value::String(actor.into()));
    map.insert("role".into(), Value::String(role.into()));
    map.insert("action".into(), Value::String(action.into()));
    map.insert("resource".into(), Value::String(resource.into()));
    map.insert("outcome".into(), Value::String(outcome.into()));
    map.insert("metadata".into(), canonicalize_value(metadata));
    let canonical = serde_json::to_string(&map)?;
    let digest = Sha256::digest(canonical.as_bytes());
    Ok(hex::encode(digest))
}

fn canonicalize_value(value: &Value) -> Value {
    match value {
        Value::Object(map) => {
            let sorted: BTreeMap<_, _> = map
                .iter()
                .map(|(k, v)| (k.as_str(), canonicalize_value(v)))
                .collect();
            let mut out = Map::new();
            for (k, v) in sorted {
                out.insert(k.to_string(), v);
            }
            Value::Object(out)
        }
        Value::Array(arr) => Value::Array(arr.iter().map(canonicalize_value).collect()),
        other => other.clone(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::AuditConfig;
    use tempfile::NamedTempFile;

    #[tokio::test]
    async fn append_and_verify_hash_sqlite() {
        let tmp = NamedTempFile::new().unwrap();
        let config = AuditConfig {
            backend: AuditBackend::Sqlite,
            sqlite_path: tmp.path().to_path_buf(),
            database_url: None,
        };
        let log = AuditLog::open(&config).await.unwrap();
        let meta = serde_json::json!({"prompt_len": 35, "latency_ms": 0.01});
        let entry = log
            .append("operator", "operator", "inference", "vault-answer", "ok", &meta)
            .await
            .unwrap();
        assert!(log.verify_entry(entry.id).await.unwrap());
        assert_eq!(entry.entry_hash.len(), 64);
    }
}
