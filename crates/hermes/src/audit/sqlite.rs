use chrono::Utc;
use rusqlite::{params, Connection};
use serde_json::Value;
use std::path::{Path, PathBuf};

use super::{hash_entry, AuditEntry, AuditError, AuditMetrics};

pub struct SqliteAuditStore {
    db_path: PathBuf,
}

impl SqliteAuditStore {
    pub fn open(db_path: impl AsRef<Path>) -> Result<Self, AuditError> {
        let db_path = db_path.as_ref().to_path_buf();
        if let Some(parent) = db_path.parent() {
            std::fs::create_dir_all(parent).map_err(|e| {
                AuditError::Sqlite(rusqlite::Error::SqliteFailure(
                    rusqlite::ffi::Error::new(rusqlite::ffi::SQLITE_CANTOPEN),
                    Some(e.to_string()),
                ))
            })?;
        }
        let store = Self { db_path };
        store.init_schema()?;
        Ok(store)
    }

    fn connect(&self) -> Result<Connection, AuditError> {
        let conn = Connection::open(&self.db_path)?;
        conn.execute_batch("PRAGMA journal_mode=WAL;")?;
        Ok(conn)
    }

    fn init_schema(&self) -> Result<(), AuditError> {
        let conn = self.connect()?;
        conn.execute_batch(
            "
            CREATE TABLE IF NOT EXISTS audit_entries (
                id INTEGER PRIMARY KEY AUTOINCREMENT,
                ts TEXT NOT NULL,
                actor TEXT NOT NULL,
                role TEXT NOT NULL,
                action TEXT NOT NULL,
                resource TEXT NOT NULL,
                outcome TEXT NOT NULL,
                metadata_json TEXT NOT NULL DEFAULT '{}',
                entry_hash TEXT NOT NULL UNIQUE
            );
            CREATE INDEX IF NOT EXISTS idx_audit_ts ON audit_entries(ts);
            ",
        )?;
        Ok(())
    }

    pub fn append(
        &self,
        actor: &str,
        role: &str,
        action: &str,
        resource: &str,
        outcome: &str,
        metadata: &Value,
    ) -> Result<AuditEntry, AuditError> {
        let ts = Utc::now().to_rfc3339();
        let entry_hash = hash_entry(&ts, actor, role, action, resource, outcome, metadata)?;
        let conn = self.connect()?;
        conn.execute(
            "INSERT INTO audit_entries
                (ts, actor, role, action, resource, outcome, metadata_json, entry_hash)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8)",
            params![
                ts,
                actor,
                role,
                action,
                resource,
                outcome,
                serde_json::to_string(metadata)?,
                entry_hash,
            ],
        )?;
        let id = conn.last_insert_rowid();
        Ok(AuditEntry {
            id,
            ts,
            actor: actor.into(),
            role: role.into(),
            action: action.into(),
            resource: resource.into(),
            outcome: outcome.into(),
            metadata: metadata.clone(),
            entry_hash,
        })
    }

    pub fn list_entries(&self, limit: i64) -> Result<Vec<AuditEntry>, AuditError> {
        let conn = self.connect()?;
        let mut stmt = conn.prepare(
            "SELECT id, ts, actor, role, action, resource, outcome, metadata_json, entry_hash
             FROM audit_entries ORDER BY id DESC LIMIT ?1",
        )?;
        let rows: Vec<AuditEntry> = stmt
            .query_map([limit], |row| {
                let metadata_str: String = row.get(7)?;
                Ok(AuditEntry {
                    id: row.get(0)?,
                    ts: row.get(1)?,
                    actor: row.get(2)?,
                    role: row.get(3)?,
                    action: row.get(4)?,
                    resource: row.get(5)?,
                    outcome: row.get(6)?,
                    metadata: serde_json::from_str(&metadata_str).unwrap_or(Value::Null),
                    entry_hash: row.get(8)?,
                })
            })?
            .collect::<Result<Vec<_>, _>>()?;
        Ok(rows.into_iter().rev().collect())
    }

    pub fn verify_entry(&self, entry_id: i64) -> Result<bool, AuditError> {
        let conn = self.connect()?;
        let row: Option<(String, String, String, String, String, String, String)> = conn
            .query_row(
                "SELECT ts, actor, role, action, resource, outcome, metadata_json
                 FROM audit_entries WHERE id = ?1",
                [entry_id],
                |row| {
                    Ok((
                        row.get(0)?,
                        row.get(1)?,
                        row.get(2)?,
                        row.get(3)?,
                        row.get(4)?,
                        row.get(5)?,
                        row.get(6)?,
                    ))
                },
            )
            .ok();

        let Some((ts, actor, role, action, resource, outcome, metadata_str)) = row else {
            return Ok(false);
        };
        let metadata: Value = serde_json::from_str(&metadata_str)?;
        let expected = hash_entry(&ts, &actor, &role, &action, &resource, &outcome, &metadata)?;
        let stored: String = conn.query_row(
            "SELECT entry_hash FROM audit_entries WHERE id = ?1",
            [entry_id],
            |row| row.get(0),
        )?;
        Ok(expected == stored)
    }

    pub fn metrics(&self) -> Result<AuditMetrics, AuditError> {
        let conn = self.connect()?;
        let total: i64 =
            conn.query_row("SELECT COUNT(*) FROM audit_entries", [], |row| row.get(0))?;
        let failures: i64 = conn.query_row(
            "SELECT COUNT(*) FROM audit_entries WHERE outcome != 'ok'",
            [],
            |row| row.get(0),
        )?;
        let mut stmt = conn.prepare(
            "SELECT action, COUNT(*) AS n FROM audit_entries GROUP BY action ORDER BY n DESC",
        )?;
        let by_action = stmt
            .query_map([], |row| Ok((row.get::<_, String>(0)?, row.get::<_, i64>(1)?)))?
            .filter_map(|r| r.ok())
            .collect();
        Ok(AuditMetrics {
            total_entries: total,
            failure_count: failures,
            by_action,
        })
    }

    pub fn list_all_entries(&self) -> Result<Vec<AuditEntry>, AuditError> {
        let conn = self.connect()?;
        let mut stmt = conn.prepare(
            "SELECT id, ts, actor, role, action, resource, outcome, metadata_json, entry_hash
             FROM audit_entries ORDER BY id ASC",
        )?;
        let rows: Vec<AuditEntry> = stmt
            .query_map([], |row| {
                let metadata_str: String = row.get(7)?;
                Ok(AuditEntry {
                    id: row.get(0)?,
                    ts: row.get(1)?,
                    actor: row.get(2)?,
                    role: row.get(3)?,
                    action: row.get(4)?,
                    resource: row.get(5)?,
                    outcome: row.get(6)?,
                    metadata: serde_json::from_str(&metadata_str).unwrap_or(Value::Null),
                    entry_hash: row.get(8)?,
                })
            })?
            .collect::<Result<Vec<_>, _>>()?;
        Ok(rows)
    }
}
