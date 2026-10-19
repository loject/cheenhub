#!/usr/bin/env bash
set -euo pipefail

# Устанавливает нативный CheenHub текущему пользователю без sudo и Docker.
# Первый запуск предназначен для локальной проверки: данные хранятся в памяти.

if [[ $# -ne 1 || ${EUID} -eq 0 ]]; then
    echo 'Usage (without sudo): bash deploy/scripts/install-native.sh ARCHIVE.tar.gz' >&2
    exit 2
fi

for tool in realpath tar sha256sum mktemp openssl systemctl curl; do
    if ! command -v "$tool" >/dev/null 2>&1; then
        echo "Missing required command: $tool" >&2
        exit 1
    fi
done
if ! systemctl --user show-environment >/dev/null 2>&1; then
    echo 'systemd user session is unavailable; log in with a normal user session.' >&2
    exit 1
fi

archive="$(realpath -e -- "$1")"
name="$(basename "$archive" .tar.gz)"
if [[ ! "$name" =~ ^cheenhub-native-linux-x86_64-[0-9a-f]{12}$ || "$archive" != *.tar.gz ]]; then
    echo 'Expected cheenhub-native-linux-x86_64-<commit>.tar.gz' >&2
    exit 2
fi
if [[ ! -f "$archive.sha256" ]]; then
    echo "Missing $archive.sha256; download it alongside the archive." >&2
    exit 1
fi
(cd "$(dirname "$archive")" && sha256sum --check -- "$(basename "$archive").sha256")

app_dir="$HOME/.local/share/cheenhub"
config_dir="$HOME/.config/cheenhub"
unit_dir="$HOME/.config/systemd/user"
release="$app_dir/releases/$name"
mkdir -p "$app_dir/releases" "$config_dir" "$unit_dir"
workdir="$(mktemp -d "$app_dir/.install.XXXXXXXX")"
trap 'rm -rf -- "$workdir"' EXIT

# Архив должен содержать единственную папку с заранее известным именем.
while IFS= read -r entry; do
    if [[ "$entry" != "$name" && "$entry" != "$name/" && "$entry" != "$name/"* ]] ||
       [[ "$entry" == *'/../'* || "$entry" == *'/..' || "$entry" == /* ]]; then
        echo "Unexpected archive path: $entry" >&2
        exit 1
    fi
done < <(tar -tzf "$archive")
tar --no-same-owner -xzf "$archive" -C "$workdir"
for file in bin/cheenhub_backend bin/cheenhub_migrations web-public/index.html SHA256SUMS; do
    if [[ ! -s "$workdir/$name/$file" ]]; then
        echo "Missing release file: $file" >&2
        exit 1
    fi
done
(cd "$workdir/$name" && sha256sum --check --status SHA256SUMS)

if [[ -e "$release" ]]; then
    (cd "$release" && sha256sum --check --status SHA256SUMS) || {
        echo "Existing release has modified files: $release" >&2
        exit 1
    }
else
    mv -- "$workdir/$name" "$release"
fi

# Секреты остаются в домашнем каталоге и сохраняются при обновлениях.
if [[ ! -e "$config_dir/backend.env" ]]; then
    (umask 077
     cat > "$config_dir/backend.env" <<CONFIG
AUTH_STORE=inmemory
DATABASE_URL=postgres://unused:unused@127.0.0.1:1/unused
BACKEND_HOST=127.0.0.1
BACKEND_PORT=3000
CHEENHUB_BASE_URL=http://127.0.0.1:3000
CHEENHUB_CLIENT_BASE_URL=http://127.0.0.1:3000
JWT_ED25519_PRIVATE_KEY_BASE64=$(openssl rand -base64 32 | tr -d '\n')
JWT_KEY_ID=native-local
ACCESS_TOKEN_LIFETIME_MINUTES=15
REFRESH_TOKEN_LIFETIME_DAYS=1
RUST_LOG=cheenhub_backend=info,warn
CONFIG
    )
fi

cat > "$unit_dir/cheenhub.service" <<'UNIT'
[Unit]
Description=CheenHub native backend
After=network.target

[Service]
Type=simple
WorkingDirectory=%h/.local/share/cheenhub/current
EnvironmentFile=%h/.config/cheenhub/backend.env
ExecStart=%h/.local/share/cheenhub/current/bin/cheenhub_backend
Restart=on-failure
RestartSec=3
TimeoutStopSec=30
NoNewPrivileges=true

[Install]
WantedBy=default.target
UNIT

ln -s -- "$release" "$app_dir/.current-next-$$"
mv -Tf -- "$app_dir/.current-next-$$" "$app_dir/current"
systemctl --user daemon-reload
systemctl --user enable cheenhub.service >/dev/null
systemctl --user restart cheenhub.service

for _ in {1..15}; do
    if systemctl --user is-active --quiet cheenhub.service &&
       curl --noproxy '*' --max-time 2 -fsS http://127.0.0.1:3000/health >/dev/null 2>&1; then
        echo 'CheenHub is running: http://127.0.0.1:3000'
        echo 'Logs: journalctl --user -u cheenhub -f'
        echo 'Local preview uses AUTH_STORE=inmemory; accounts are not persistent.'
        exit 0
    fi
    sleep 1
done
systemctl --user status cheenhub.service --no-pager >&2 || true
exit 1
