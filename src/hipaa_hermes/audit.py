"""Append-only audit log with per-entry SHA-256 hashing (no full hash-chain)."""

from __future__ import annotations

import hashlib
import json
import sqlite3
from dataclasses import dataclass
from datetime import datetime, timezone
from pathlib import Path
from typing import Any


def _utcnow() -> str:
    return datetime.now(timezone.utc).isoformat()


def _canonical(payload: dict[str, Any]) -> str:
    return json.dumps(payload, sort_keys=True, separators=(",", ":"))


def _hash_entry(fields: dict[str, Any]) -> str:
    return hashlib.sha256(_canonical(fields).encode()).hexdigest()


@dataclass(frozen=True)
class AuditEntry:
    id: int
    ts: str
    actor: str
    role: str
    action: str
    resource: str
    outcome: str
    metadata: dict[str, Any]
    entry_hash: str


class AuditLog:
    """SQLite-backed append-only log. No UPDATE/DELETE APIs."""

    def __init__(self, db_path: Path) -> None:
        self.db_path = db_path
        self.db_path.parent.mkdir(parents=True, exist_ok=True)
        self._init_schema()

    def _connect(self) -> sqlite3.Connection:
        conn = sqlite3.connect(self.db_path)
        conn.row_factory = sqlite3.Row
        conn.execute("PRAGMA journal_mode=WAL")
        return conn

    def _init_schema(self) -> None:
        with self._connect() as conn:
            conn.execute(
                """
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
                )
                """
            )
            conn.execute(
                "CREATE INDEX IF NOT EXISTS idx_audit_ts ON audit_entries(ts)"
            )
            conn.commit()

    def append(
        self,
        *,
        actor: str,
        role: str,
        action: str,
        resource: str,
        outcome: str = "ok",
        metadata: dict[str, Any] | None = None,
    ) -> AuditEntry:
        ts = _utcnow()
        meta = metadata or {}
        hash_input = {
            "ts": ts,
            "actor": actor,
            "role": role,
            "action": action,
            "resource": resource,
            "outcome": outcome,
            "metadata": meta,
        }
        entry_hash = _hash_entry(hash_input)
        with self._connect() as conn:
            cur = conn.execute(
                """
                INSERT INTO audit_entries
                    (ts, actor, role, action, resource, outcome, metadata_json, entry_hash)
                VALUES (?, ?, ?, ?, ?, ?, ?, ?)
                """,
                (
                    ts,
                    actor,
                    role,
                    action,
                    resource,
                    outcome,
                    json.dumps(meta),
                    entry_hash,
                ),
            )
            conn.commit()
            row_id = cur.lastrowid
        return AuditEntry(
            id=row_id,
            ts=ts,
            actor=actor,
            role=role,
            action=action,
            resource=resource,
            outcome=outcome,
            metadata=meta,
            entry_hash=entry_hash,
        )

    def list_entries(self, *, limit: int = 100) -> list[AuditEntry]:
        with self._connect() as conn:
            rows = conn.execute(
                """
                SELECT id, ts, actor, role, action, resource, outcome,
                       metadata_json, entry_hash
                FROM audit_entries
                ORDER BY id DESC
                LIMIT ?
                """,
                (limit,),
            ).fetchall()
        return [_row_to_entry(r) for r in reversed(rows)]

    def verify_entry(self, entry_id: int) -> bool:
        """Recompute hash for one row — tamper detection, not a chain."""
        with self._connect() as conn:
            row = conn.execute(
                "SELECT * FROM audit_entries WHERE id = ?", (entry_id,)
            ).fetchone()
        if not row:
            return False
        meta = json.loads(row["metadata_json"])
        expected = _hash_entry(
            {
                "ts": row["ts"],
                "actor": row["actor"],
                "role": row["role"],
                "action": row["action"],
                "resource": row["resource"],
                "outcome": row["outcome"],
                "metadata": meta,
            }
        )
        return expected == row["entry_hash"]

    def metrics(self) -> dict[str, Any]:
        with self._connect() as conn:
            total = conn.execute("SELECT COUNT(*) FROM audit_entries").fetchone()[0]
            failures = conn.execute(
                "SELECT COUNT(*) FROM audit_entries WHERE outcome != 'ok'"
            ).fetchone()[0]
            by_action = conn.execute(
                """
                SELECT action, COUNT(*) AS n
                FROM audit_entries
                GROUP BY action
                ORDER BY n DESC
                """
            ).fetchall()
        return {
            "total_entries": total,
            "failure_count": failures,
            "by_action": {r["action"]: r["n"] for r in by_action},
        }


def _row_to_entry(row: sqlite3.Row) -> AuditEntry:
    return AuditEntry(
        id=row["id"],
        ts=row["ts"],
        actor=row["actor"],
        role=row["role"],
        action=row["action"],
        resource=row["resource"],
        outcome=row["outcome"],
        metadata=json.loads(row["metadata_json"]),
        entry_hash=row["entry_hash"],
    )
