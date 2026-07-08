# Local dev Vault Agent — renders secrets to data/vault/hermes.env
# Token file auth is acceptable for laptop demos only; use AppRole in dev/prod.

pid_file = "/vault/secrets/agent.pid"

vault {
  address = "http://127.0.0.1:8200"
}

auto_auth {
  method "token_file" {
    mount_path = "auth/token_file"
    config = {
      token_file_path = "/vault/config/dev-token"
    }
  }

  sink "file" {
    config = {
      path = "/vault/secrets/agent-token"
      mode = 0600
    }
  }
}

template {
  source      = "/vault/templates/hermes-secrets.tpl"
  destination = "/vault/secrets/hermes.env"
  perms       = 0644
}
