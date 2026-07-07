"""Minimal inference gateway — v1 skeleton."""

from __future__ import annotations

import time
from typing import Any

from fastapi import Depends, FastAPI
from pydantic import BaseModel, Field

from hipaa_hermes.audit import AuditLog
from hipaa_hermes.auth import AuthContext, Role, build_auth_checker
from hipaa_hermes.settings import settings

app = FastAPI(title="HIPAA Hermes", version="0.1.0", description="Inference platform v1 skeleton")

audit = AuditLog(settings.database_path)
_checker = build_auth_checker(settings.admin_secret, settings.auditor_secret)
RequireOperator = Depends(_checker(Role.OPERATOR))
RequireAuditor = Depends(_checker(Role.AUDITOR))
RequireEither = Depends(_checker(Role.OPERATOR, Role.AUDITOR))


class InferenceRequest(BaseModel):
    prompt: str = Field(..., max_length=4000)
    skill: str = Field(default="vault-answer", max_length=64)


class InferenceResponse(BaseModel):
    output: str
    latency_ms: float
    audit_id: int
    entry_hash: str


@app.get("/health")
def health() -> dict[str, str]:
    return {"status": "ok", "version": "0.1.0"}


@app.get("/metrics")
def metrics() -> dict[str, Any]:
    """Prometheus-friendly JSON for Grafana JSON datasource or manual import."""
    m = audit.metrics()
    return {
        "audit_total": m["total_entries"],
        "audit_failures": m["failure_count"],
        "audit_by_action": m["by_action"],
    }


@app.post("/v1/inference", response_model=InferenceResponse)
def inference(
    body: InferenceRequest,
    auth: AuthContext = RequireOperator,
) -> InferenceResponse:
    """Stub inference — records audit event; no external LLM in v1."""
    started = time.perf_counter()
    # v1: deterministic stub (no PHI stored in prompt beyond audit metadata hash)
    output = f"[stub:{body.skill}] processed {len(body.prompt)} chars (de-ID assumed upstream)"
    latency_ms = (time.perf_counter() - started) * 1000
    entry = audit.append(
        actor=auth.actor,
        role=auth.role.value,
        action="inference",
        resource=body.skill,
        outcome="ok",
        metadata={"prompt_len": len(body.prompt), "latency_ms": round(latency_ms, 2)},
    )
    return InferenceResponse(
        output=output,
        latency_ms=latency_ms,
        audit_id=entry.id,
        entry_hash=entry.entry_hash,
    )


@app.get("/v1/audit")
def list_audit(
    auth: AuthContext = RequireEither,
    limit: int = 50,
) -> dict[str, Any]:
    entries = audit.list_entries(limit=min(limit, 200))
    return {
        "role": auth.role.value,
        "entries": [
            {
                "id": e.id,
                "ts": e.ts,
                "actor": e.actor,
                "role": e.role,
                "action": e.action,
                "resource": e.resource,
                "outcome": e.outcome,
                "metadata": e.metadata,
                "entry_hash": e.entry_hash,
                "hash_valid": audit.verify_entry(e.id),
            }
            for e in entries
        ],
    }


@app.get("/v1/audit/export")
def export_audit(
    auth: AuthContext = RequireAuditor,
) -> dict[str, Any]:
    """Auditor-only bulk export."""
    entries = audit.list_entries(limit=500)
    return {
        "exported_by": auth.actor,
        "count": len(entries),
        "entries": [
            {
                "id": e.id,
                "ts": e.ts,
                "action": e.action,
                "resource": e.resource,
                "outcome": e.outcome,
                "entry_hash": e.entry_hash,
            }
            for e in entries
        ],
    }
