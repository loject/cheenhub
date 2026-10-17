#!/usr/bin/env bash
set -euo pipefail

# Изолированный smoke-тест автономного Rust-образа рядом с работающим Nginx.
# Запуск: bash deploy/scripts/smoke-standalone.sh ghcr.io/loject/cheenhub/backend-standalone:sha-<SHA>
# Не использует .env.production, production-БД, сертификаты или Docker Compose.

if [[ $# -ne 1 ]]; then
    echo 'Usage: bash deploy/scripts/smoke-standalone.sh ghcr.io/loject/cheenhub/backend-standalone:sha-<40-hex-sha>' >&2
    exit 2
fi

image_ref="$1"
if [[ ! "$image_ref" =~ ^ghcr\.io/loject/cheenhub/backend-standalone:sha-[0-9a-f]{40}$ ]]; then
    echo 'Expected an immutable backend-standalone preview image tag (sha-<40-hex-sha>).' >&2
    exit 2
fi

for program in docker curl openssl mktemp; do
    if ! command -v "$program" >/dev/null 2>&1; then
        echo "Required executable not found: $program" >&2
        exit 1
    fi
done

workdir="$(mktemp -d)"
container="cheenhub-standalone-smoke-$$"
cleanup() {
    docker rm -f "$container" >/dev/null 2>&1 || true
    rm -rf -- "$workdir"
}
trap cleanup EXIT

# Создаём только тестовые секреты и ACME webroot. Production volume не монтируем.
openssl req -x509 -newkey rsa:2048 -nodes -days 1 \
    -subj '/CN=cheenhub.ru' -addext 'subjectAltName=DNS:cheenhub.ru' \
    -keyout "$workdir/key.pem" -out "$workdir/cert.pem" >/dev/null 2>&1
chmod 600 "$workdir/key.pem"
mkdir -p "$workdir/webroot/.well-known/acme-challenge"
printf 'cheenhub-preview-challenge\n' > "$workdir/webroot/.well-known/acme-challenge/preview-smoke"
jwt_seed="$(openssl rand -base64 32 | tr -d '\n')"

# Docker выбирает свободные порты и публикует только TCP на localhost.
# Контейнер не подключается к сети Compose, а AUTH_STORE=inmemory не использует Postgres.
docker run --detach --rm --name "$container" \
    --pull=missing \
    --network bridge \
    --publish '127.0.0.1::3000/tcp' \
    --publish '127.0.0.1::3443/tcp' \
    --publish '127.0.0.1::8080/tcp' \
    --cap-drop ALL --security-opt no-new-privileges \
    --read-only --tmpfs /tmp:rw,nosuid,nodev \
    --mount "type=bind,src=$workdir,dst=/run/cheenhub-preview,readonly" \
    --env AUTH_STORE=inmemory \
    --env 'DATABASE_URL=postgres://preview:preview@127.0.0.1:1/unused' \
    --env BACKEND_HOST=0.0.0.0 \
    --env BACKEND_PORT=3000 \
    --env BACKEND_HTTPS_PORT=3443 \
    --env BACKEND_EDGE_HTTP_PORT=8080 \
    --env BACKEND_ACME_WEBROOT=/run/cheenhub-preview/webroot \
    --env WEBTRANSPORT_TLS_CERT_PATH=/run/cheenhub-preview/cert.pem \
    --env WEBTRANSPORT_TLS_KEY_PATH=/run/cheenhub-preview/key.pem \
    --env CHEENHUB_BASE_URL=https://cheenhub.ru \
    --env CHEENHUB_CLIENT_BASE_URL=https://cheenhub.ru \
    --env "JWT_ED25519_PRIVATE_KEY_BASE64=$jwt_seed" \
    --env JWT_KEY_ID=standalone-preview \
    --env ACCESS_TOKEN_LIFETIME_MINUTES=15 \
    --env REFRESH_TOKEN_LIFETIME_DAYS=1 \
    --env RUST_LOG=cheenhub_backend=info,warn \
    "$image_ref" >/dev/null
unset jwt_seed

published_port() {
    local mapping
    mapping="$(docker port "$container" "$1/tcp")"
    if [[ ! "$mapping" =~ ^127\.0\.0\.1:[0-9]+$ ]]; then
        echo "Unsafe or unexpected Docker port binding for $1/tcp: $mapping" >&2
        exit 1
    fi
    printf '%s\n' "${mapping##*:}"
}

http_port="$(published_port 3000)"
https_port="$(published_port 3443)"
edge_port="$(published_port 8080)"

ready=false
for _ in {1..30}; do
    if curl --noproxy '*' -fsS --max-time 2 \
        "http://127.0.0.1:$http_port/health" >/dev/null 2>&1; then
        ready=true
        break
    fi
    if [[ "$(docker inspect -f '{{.State.Running}}' "$container")" != true ]]; then
        break
    fi
    sleep 1
done
if [[ "$ready" != true ]]; then
    docker logs --tail 80 "$container" >&2 || true
    echo 'Standalone backend did not pass /health readiness.' >&2
    exit 1
fi

for path in / /index.html /sw.js /manifest.webmanifest; do
    curl --noproxy '*' -fsS --max-time 5 \
        "http://127.0.0.1:$http_port$path" -o "$workdir/page"
    if [[ ! -s "$workdir/page" ]]; then
        echo "Empty response from $path" >&2
        exit 1
    fi
    echo "OK HTTP $path"
done

curl --noproxy '*' -fsS --max-time 5 \
    --cacert "$workdir/cert.pem" \
    --resolve "cheenhub.ru:$https_port:127.0.0.1" \
    -D "$workdir/https.headers" \
    "https://cheenhub.ru:$https_port/" -o "$workdir/page"
[[ -s "$workdir/page" ]]
grep -qi '^strict-transport-security: max-age=' "$workdir/https.headers"
echo 'OK HTTPS / (certificate verified, HSTS present)'

curl --noproxy '*' -sS --max-time 5 \
    -D "$workdir/redirect.headers" -o /dev/null \
    "http://127.0.0.1:$edge_port/preview?x=1"
grep -q '^HTTP/1.1 301' "$workdir/redirect.headers"
grep -Fqi 'location: https://cheenhub.ru/preview?x=1' "$workdir/redirect.headers"
echo 'OK HTTP edge redirect (301)'

challenge="$(curl --noproxy '*' -fsS --max-time 5 \
    "http://127.0.0.1:$edge_port/.well-known/acme-challenge/preview-smoke")"
[[ "$challenge" == 'cheenhub-preview-challenge' ]]
echo 'OK ACME HTTP-01 response'

missing_status="$(curl --noproxy '*' -sS -o /dev/null -w '%{http_code}' --max-time 5 \
    "http://127.0.0.1:$edge_port/.well-known/acme-challenge/missing")"
[[ "$missing_status" == 404 ]]
echo 'OK missing ACME token (404)'

echo 'PASS: standalone image serves HTTP, HTTPS, static site, redirect, and ACME.'
echo 'Preview container and temporary credentials are being removed.'
