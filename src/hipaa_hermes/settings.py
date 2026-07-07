from pathlib import Path

from pydantic_settings import BaseSettings, SettingsConfigDict


class Settings(BaseSettings):
    model_config = SettingsConfigDict(env_file=".env", extra="ignore")

    database_path: Path = Path("data/hipaa_hermes.db")
    admin_secret: str = "change-me-operator"
    auditor_secret: str = "change-me-auditor"


settings = Settings()
