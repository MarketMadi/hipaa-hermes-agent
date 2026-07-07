"""Two-role RBAC: operator (read/write) and auditor (read audit only)."""

from __future__ import annotations

from enum import Enum

from fastapi import Header, HTTPException


class Role(str, Enum):
    OPERATOR = "operator"
    AUDITOR = "auditor"


class AuthContext:
    def __init__(self, role: Role, actor: str) -> None:
        self.role = role
        self.actor = actor


def build_auth_checker(operator_secret: str, auditor_secret: str):
    """FastAPI dependency factory — header X-Role-Key maps to role."""

    def require_role(*allowed: Role):
        def _dep(x_role_key: str = Header(alias="X-Role-Key")) -> AuthContext:
            if not x_role_key:
                raise HTTPException(status_code=401, detail="missing X-Role-Key")
            if x_role_key == operator_secret and Role.OPERATOR in allowed:
                return AuthContext(Role.OPERATOR, "operator")
            if x_role_key == auditor_secret and Role.AUDITOR in allowed:
                return AuthContext(Role.AUDITOR, "auditor")
            if x_role_key in (operator_secret, auditor_secret):
                raise HTTPException(
                    status_code=403,
                    detail=f"role not permitted; need {[r.value for r in allowed]}",
                )
            raise HTTPException(status_code=401, detail="invalid X-Role-Key")

        return _dep

    return require_role
