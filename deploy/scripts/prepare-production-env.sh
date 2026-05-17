#!/usr/bin/env bash
set -euo pipefail

host="${1:-cheenhub.ru}"
env_file="${2:-.env.production}"
key_id="${CHEENHUB_JWT_KEY_ID:-prod-ed25519-1}"

if [[ -f "$env_file" ]]; then
    echo "$env_file already exists; refusing to overwrite it." >&2
    exit 1
fi

jwt_keys="$(cargo run --quiet -p cheenhub_backend --bin cheenhub_generate_jwt_keys)"
private_key="$(printf '%s\n' "$jwt_keys" | sed -n 's/^JWT_ED25519_PRIVATE_KEY_BASE64=//p')"
public_key="$(printf '%s\n' "$jwt_keys" | sed -n 's/^CHEENHUB_JWT_PUBLIC_KEY_BASE64=//p')"
postgres_password="$(openssl rand -hex 32)"

cat > "$env_file" <<EOF
CHEENHUB_DOMAIN=$host
CHEENHUB_BACKEND_IMAGE=cheenhub-backend
CHEENHUB_WEB_IMAGE=cheenhub-web
CHEENHUB_IMAGE_TAG=latest

POSTGRES_DB=cheenhub
POSTGRES_USER=cheenhub
POSTGRES_PASSWORD=$postgres_password

DATABASE_URL=postgres://cheenhub:$postgres_password@db:5432/cheenhub
AUTH_STORE=postgres
BACKEND_HOST=0.0.0.0
BACKEND_PORT=3000
RUST_LOG=cheenhub_backend=info,tower_http=info,warn

JWT_ED25519_PRIVATE_KEY_BASE64=$private_key
JWT_KEY_ID=$key_id
CHEENHUB_JWT_KEY_ID=$key_id
CHEENHUB_JWT_PUBLIC_KEY_BASE64=$public_key
ACCESS_TOKEN_LIFETIME_MINUTES=15
REFRESH_TOKEN_LIFETIME_DAYS=30

CHEENHUB_CLIENT_BASE_URL=https://$host
CHEENHUB_API_BASE_URL=https://$host/api
CHEENHUB_REALTIME_URL=https://$host/realtime
CHEENHUB_REALTIME_CERT_SHA256=

WEBTRANSPORT_HOST=0.0.0.0
WEBTRANSPORT_PORT=4443
WEBTRANSPORT_TLS_CERT_PATH=/etc/letsencrypt/live/cheenhub.ru/fullchain.pem
WEBTRANSPORT_TLS_KEY_PATH=/etc/letsencrypt/live/cheenhub.ru/privkey.pem

OAUTH_STATE_LIFETIME_MINUTES=10
OAUTH_HANDOFF_LIFETIME_MINUTES=5
OAUTH_REGISTRATION_LIFETIME_MINUTES=15
PASSWORD_RESET_TOKEN_LIFETIME_MINUTES=30
SMTP_PORT=587
EOF

chmod 600 "$env_file"
echo "Wrote $env_file."
