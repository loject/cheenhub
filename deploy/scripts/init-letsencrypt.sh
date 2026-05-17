#!/usr/bin/env bash
set -euo pipefail

domain="${CHEENHUB_DOMAIN:-cheenhub.ru}"
compose_files="${COMPOSE_FILES:--f deploy/compose.yml -f deploy/compose.artifact.yml}"
env_file="${ENV_FILE:-.env.production}"

if [[ ! -f "$env_file" ]]; then
    echo "$env_file is missing. Run deploy/scripts/prepare-production-env.sh first." >&2
    exit 1
fi

compose() {
    # shellcheck disable=SC2086
    docker compose --env-file "$env_file" $compose_files "$@"
}

docker volume create cheenhub_letsencrypt >/dev/null
volume_path="$(docker volume inspect cheenhub_letsencrypt --format '{{ .Mountpoint }}')"
cert_path="$volume_path/live/$domain/fullchain.pem"
if [[ ! -f "$cert_path" ]]; then
    tmpdir="$(mktemp -d)"
    trap 'rm -rf "$tmpdir"' EXIT
    mkdir -p "$tmpdir/live/$domain"
    openssl req -x509 -newkey rsa:2048 -nodes -days 1 \
        -keyout "$tmpdir/live/$domain/privkey.pem" \
        -out "$tmpdir/live/$domain/fullchain.pem" \
        -subj "/CN=$domain"
    mkdir -p "$volume_path/live/$domain"
    cp "$tmpdir/live/$domain/"*.pem "$volume_path/live/$domain/"
fi

compose up -d web

if [[ ! -f "$volume_path/renewal/$domain.conf" ]]; then
    rm -rf "$volume_path/live/$domain" "$volume_path/archive/$domain"
fi

email_args=(--register-unsafely-without-email)
if [[ -n "${CERTBOT_EMAIL:-}" ]]; then
    email_args=(--email "$CERTBOT_EMAIL" --no-eff-email)
fi

compose run --rm --entrypoint certbot certbot certonly \
    --webroot \
    -w /var/www/certbot \
    -d "$domain" \
    --agree-tos \
    "${email_args[@]}" \
    --force-renewal

if [[ -d "$volume_path/live/$domain-0001" && -f "$volume_path/renewal/$domain-0001.conf" ]]; then
    rm -rf "$volume_path/live/$domain" "$volume_path/archive/$domain" "$volume_path/renewal/$domain.conf"
    ln -s "$domain-0001" "$volume_path/live/$domain"
fi
compose exec web nginx -s reload
echo "Let's Encrypt certificate is ready for $domain."
