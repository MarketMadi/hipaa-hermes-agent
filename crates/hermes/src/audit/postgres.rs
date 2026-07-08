use chrono::Utc;
use serde_json::Value;
use sqlx::postgres::PgPoolOptions;
use sqlx::{PgPool, Row};

use super::{hash_entry, AuditEntry, AuditError, AuditMetrics};

pub struct PostgresAuditStore {
    pool: PgPool,
}

impl PostgresAuditStore {
    pub async fn connect(database_url: &str) -> Result<Self, AuditError> {
        let pool = PgPoolOptions::new()
            .max_connections(10)
            .connect(database_url)
            .await?;
        let store = Self { pool };
        store.init_schema().await?;
        Ok(store)
    }

    async fn init_schema(&self) -> Result<(), AuditError> {
        sqlx::query(
            "CREATE TABLE IF NOT EXISTS audit_entries (
                id BIGSERIAL PRIMARY KEY,
                ts TEXT NOT NULL,
                actor TEXT NOT NULL,
                role TEXT NOT NULL,
                action TEXT NOT NULL,
                resource TEXT NOT NULL,
                outcome TEXT NOT NULL,
                metadata_json TEXT NOT NULL DEFAULT '{}',
                entry_hash TEXT NOT NULL UNIQUE
            )",
        )
        .execute(&self.pool)
        .await?;
        sqlx::query("CREATE INDEX IF NOT EXISTS idx_audit_ts ON audit_entries(ts)")
            .execute(&self.pool)
            .await?;
        Ok(())
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
        let ts = Utc::now().to_rfc3339();
        let entry_hash = hash_entry(&ts, actor, role, action, resource, outcome, metadata)?;
        let metadata_json = serde_json::to_string(metadata)?;
        let row = sqlx::query(
            "INSERT INTO audit_entries
                (ts, actor, role, action, resource, outcome, metadata_json, entry_hash)
             VALUES ($1, $2, $3, $4, $5, $6, $7, $8)
             RETURNING id",
        )
        .bind(&ts)
        .bind(actor)
        .bind(role)
        .bind(action)
        .bind(resource)
        .bind(outcome)
        .bind(&metadata_json)
        .bind(&entry_hash)
        .fetch_one(&self.pool)
        .await?;
        let id: i64 = row.get(0);
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

    pub async fn list_entries(&self, limit: i64) -> Result<Vec<AuditEntry>, AuditError> {
        let rows = sqlx::query(
            "SELECT id, ts, actor, role, action, resource, outcome, metadata_json, entry_hash
             FROM audit_entries ORDER BY id DESC LIMIT $1",
        )
        .bind(limit)
        .fetch_all(&self.pool)
        .await?;

        let mut entries = Vec::with_capacity(rows.len());
        for row in rows {
            let metadata_str: String = row.get(7);
            entries.push(AuditEntry {
                id: row.get(0),
                ts: row.get(1),
                actor: row.get(2),
                role: row.get(3),
                action: row.get(4),
                resource: row.get(5),
                outcome: row.get(6),
                metadata: serde_json::from_str(&metadata_str).unwrap_or(Value::Null),
                entry_hash: row.get(8),
            });
        }
        entries.reverse();
        Ok(entries)
    }

    pub async fn verify_entry(&self, entry_id: i64) -> Result<bool, AuditError> {
        let row = sqlx::query(
            "SELECT ts, actor, role, action, resource, outcome, metadata_json, entry_hash
             FROM audit_entries WHERE id = $1",
        )
        .bind(entry_id)
        .fetch_optional(&self.pool)
        .await?;

        let Some(row) = row else {
            return Ok(false);
        };

        let ts: String = row.get(0);
        let actor: String = row.get(1);
        let role: String = row.get(2);
        let action: String = row.get(3);
        let resource: String = row.get(4);
        let outcome: String = row.get(5);
        let metadata_str: String = row.get(6);
        let stored: String = row.get(7);
        let metadata: Value = serde_json::from_str(&metadata_str)?;
        let expected = hash_entry(
            &ts, &actor, &role, &action, &resource, &outcome, &metadata,
        )?;
        Ok(expected == stored)
    }

    pub async fn metrics(&self) -> Result<AuditMetrics, AuditError> {
        let total: i64 =
            sqlx::query_scalar("SELECT COUNT(*)::bigint FROM audit_entries")
                .fetch_one(&self.pool)
                .await?;
        let failures: i64 = sqlx::query_scalar(
            "SELECT COUNT(*)::bigint FROM audit_entries WHERE outcome != 'ok'",
        )
        .fetch_one(&self.pool)
        .await?;
        let rows = sqlx::query(
            "SELECT action, COUNT(*)::bigint AS n FROM audit_entries GROUP BY action ORDER BY n DESC",
        )
        .fetch_all(&self.pool)
        .await?;
        let by_action = rows
            .into_iter()
            .map(|row| (row.get::<String, _>(0), row.get::<i64, _>(1)))
            .collect();
        Ok(AuditMetrics {
            total_entries: total,
            failure_count: failures,
            by_action,
        })
    }

    pub async fn insert_migrated(&self, entry: &AuditEntry) -> Result<bool, AuditError> {
        let metadata_json = serde_json::to_string(&entry.metadata)?;
        let result = sqlx::query(
            "INSERT INTO audit_entries
                (id, ts, actor, role, action, resource, outcome, metadata_json, entry_hash)
             VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9)
             ON CONFLICT (entry_hash) DO NOTHING",
        )
        .bind(entry.id)
        .bind(&entry.ts)
        .bind(&entry.actor)
        .bind(&entry.role)
        .bind(&entry.action)
        .bind(&entry.resource)
        .bind(&entry.outcome)
        .bind(&metadata_json)
        .bind(&entry.entry_hash)
        .execute(&self.pool)
        .await?;
        Ok(result.rows_affected() > 0)
    }

    pub async fn sync_sequence(&self) -> Result<(), AuditError> {
        sqlx::query(
            "SELECT setval(
                pg_get_serial_sequence('audit_entries', 'id'),
                COALESCE((SELECT MAX(id) FROM audit_entries), 1)
            )",
        )
        .execute(&self.pool)
        .await?;
        Ok(())
    }
}
