{{- with secret "secret/data/hermes/local" -}}
ADMIN_SECRET={{ .Data.data.admin_secret }}
AUDITOR_SECRET={{ .Data.data.auditor_secret }}
{{- if .Data.data.database_url }}
DATABASE_URL={{ .Data.data.database_url }}
{{- end }}
{{- if .Data.data.anthropic_api_key }}
ANTHROPIC_API_KEY={{ .Data.data.anthropic_api_key }}
{{- end }}
{{- end -}}
