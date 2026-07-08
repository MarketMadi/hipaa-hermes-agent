# Hermes can read secrets for the current environment.
path "secret/data/hermes/{{identity.entity.aliases.hermes_local_*.name}}" {
  capabilities = ["read"]
}

path "secret/data/hermes/local" {
  capabilities = ["read"]
}
