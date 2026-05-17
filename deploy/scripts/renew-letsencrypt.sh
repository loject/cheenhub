#!/usr/bin/env bash
set -euo pipefail

compose_files="${COMPOSE_FILES:--f deploy/compose.yml -f deploy/compose.artifact.yml}"
env_file="${ENV_FILE:-.env.production}"

compose() {
    # shellcheck disable=SC2086
    docker compose --env-file "$env_file" $compose_files "$@"
}

compose run --rm --entrypoint certbot certbot renew --webroot -w /var/www/certbot
compose exec web nginx -s reload
compose restart backend
