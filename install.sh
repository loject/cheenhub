#!/usr/bin/env bash
set -euo pipefail

# Root обновляет текущий production; обычный пользователь устанавливает preview.
# Сборка из исходников не требуется; production использует существующие БД и Certbot.

readonly REPOSITORY='loject/cheenhub'
readonly ASSET='cheenhub-native-linux-x86_64.tar.gz'
readonly SERVICE='cheenhub.service'
RELEASE_VERSION=''
EXPECTED_COMMIT=''

fail() { printf 'Ошибка: %s\n' "$*" >&2; exit 1; }
info() { printf '==> %s\n' "$*"; }

is_system_install() { [[ ${EUID} -eq 0 ]]; }

select_release_source() {
    if [[ -n "$RELEASE_VERSION" ]]; then
        CHANNEL="$RELEASE_VERSION"
        RELEASE_URL="https://github.com/${REPOSITORY}/releases/download/${CHANNEL}"
    elif is_system_install; then
        CHANNEL=latest
        RELEASE_URL="https://github.com/${REPOSITORY}/releases/latest/download"
    else
        CHANNEL=native-preview
        RELEASE_URL="https://github.com/${REPOSITORY}/releases/download/${CHANNEL}"
    fi
}

verify_release_metadata() {
    local metadata="$1/BUILD_INFO"
    [[ -f "$metadata" ]] || fail 'В архиве отсутствует BUILD_INFO.'
    if [[ -n "$EXPECTED_COMMIT" ]]; then
        grep -Fxq "commit=$EXPECTED_COMMIT" "$metadata" || fail 'Архив собран из другого коммита.'
    fi
    if [[ -n "$RELEASE_VERSION" ]]; then
        grep -Fxq "release_tag=$RELEASE_VERSION" "$metadata" || fail 'Архив не соответствует запрошенному тегу.'
    fi
    if is_system_install; then
        grep -Fxq 'base_url=https://cheenhub.ru' "$metadata" || fail 'Архив собран с другим адресом production.'
    fi
}

initialize_paths() {
    if is_system_install; then
        APP_DIR=/opt/cheenhub-native
        CONFIG_DIR="$APP_DIR"
        CONFIG_FILE="$APP_DIR/.env"
        UNIT_DIR=/etc/systemd/system
    else
        APP_DIR="${HOME}/.local/share/cheenhub"
        CONFIG_DIR="${HOME}/.config/cheenhub"
        CONFIG_FILE="$CONFIG_DIR/backend.env"
        UNIT_DIR="${HOME}/.config/systemd/user"
    fi
}

service_control() {
    if is_system_install; then systemctl "$@"; else systemctl --user "$@"; fi
}

requirements() {
    [[ "$(uname -s)" == Linux && "$(uname -m)" == x86_64 ]] || fail 'Поддерживается только Linux x86_64.'
    local command
    for command in curl tar sha256sum mktemp openssl systemctl; do
        command -v "$command" >/dev/null 2>&1 || fail "Не найдена утилита $command. Установите её системным пакетным менеджером."
    done
    if is_system_install; then
        for command in systemd-run systemd-analyze groupadd getent flock install cmp readlink; do
            command -v "$command" >/dev/null 2>&1 || fail "Не найдена утилита $command."
        done
        [[ -s "$CONFIG_FILE" ]] || fail "Для текущего production нужен существующий $CONFIG_FILE с настройками БД и JWT."
        [[ -r /var/lib/docker/volumes/cheenhub_letsencrypt/_data/live/cheenhub.ru/fullchain.pem ]] \
            || fail 'Не найден действующий production-сертификат cheenhub.ru.'
    else
        systemctl --user show-environment >/dev/null 2>&1 || fail 'Нет пользовательской сессии systemd. Войдите по SSH обычным пользователем.'
    fi
}

verify_archive() {
    local directory="$1" name entry
    local archive="$directory/$ASSET"
    (cd "$directory" && sha256sum -c -- "$ASSET.sha256" >/dev/null) || fail 'Контрольная сумма архива неверна.'
    name="$(tar -tzf "$archive" | sed -n '1p')"
    name="${name%%/*}"
    [[ "$name" =~ ^cheenhub-native-linux-x86_64-[0-9a-f]{12}$ ]] || fail 'Недопустимая структура релиза.'
    tar -tzf "$archive" >/dev/null || fail 'Архив повреждён.'
    while IFS= read -r entry; do
        case "$entry" in
            "$name"|"$name/"|"$name/"*) ;;
            *) fail "Недопустимый путь архива: $entry" ;;
        esac
        case "$entry" in
            */../*|*/..|*/./*|/*) fail "Недопустимый путь архива: $entry" ;;
        esac
    done < <(tar -tzf "$archive")
    printf '%s\n' "$name"
}

write_config() {
    if is_system_install; then
        [[ -s "$CONFIG_FILE" ]] || fail "Не найден production .env: $CONFIG_FILE"
        chmod 600 "$CONFIG_FILE"
        return
    fi
    if [[ ! -e "$CONFIG_FILE" ]]; then
        info 'Создаю локальную тестовую конфигурацию.'
        (umask 077; cat > "$CONFIG_FILE" <<CONFIG
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
}

write_unit() {
    if is_system_install; then
        cat > "$UNIT_DIR/$SERVICE" <<UNIT
[Unit]
Description=CheenHub native production backend
Wants=network-online.target
After=network-online.target docker.service
Requires=docker.service
[Service]
Type=simple
DynamicUser=yes
PrivateTmp=yes
NoNewPrivileges=yes
AmbientCapabilities=CAP_NET_BIND_SERVICE
CapabilityBoundingSet=CAP_NET_BIND_SERVICE
WorkingDirectory=$APP_DIR/current
EnvironmentFile=$CONFIG_FILE
BindReadOnlyPaths=/var/lib/docker/volumes/cheenhub_certbot-webroot/_data:/var/lib/cheenhub/acme
ExecStart=$APP_DIR/current/bin/cheenhub_backend
Restart=on-failure
RestartSec=3
TimeoutStopSec=35
[Install]
WantedBy=multi-user.target
UNIT
        if [[ -f /opt/cheenhub/deploy/secrets/firebase-service-account.json ]]; then
            sed -i '/^ExecStart=/i LoadCredential=fcm:/opt/cheenhub/deploy/secrets/firebase-service-account.json' "$UNIT_DIR/$SERVICE"
        fi
        return
    fi
    cat > "$UNIT_DIR/$SERVICE" <<'UNIT'
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
}

healthcheck() {
    local attempt
    local -a health_url=(http://127.0.0.1:3000/health)
    if is_system_install; then
        health_url=(--resolve cheenhub.ru:443:127.0.0.1 https://cheenhub.ru/health)
    fi
    for attempt in {1..15}; do
        if service_control is-active --quiet "$SERVICE" && \
           curl --noproxy '*' --silent --fail --max-time 2 "${health_url[@]}" >/dev/null; then
            return 0
        fi
        sleep 1
    done
    return 1
}

install_or_update() {
    requirements
    local archive name release old_link='' next_link
    mkdir -p "$APP_DIR/releases" "$CONFIG_DIR" "$UNIT_DIR"
    if is_system_install; then
        install -d -m 755 "$APP_DIR" "$APP_DIR/releases" "$UNIT_DIR"
        exec 8>"$APP_DIR/.install.lock"
        flock -n 8 || fail 'Установка CheenHub уже выполняется.'
    fi
    INSTALL_TEMP_DIR="$(mktemp -d)"
    trap 'if [[ -n "${INSTALL_TEMP_DIR:-}" ]]; then rm -rf -- "$INSTALL_TEMP_DIR"; fi' EXIT
    info "Скачиваю нативную сборку CheenHub: $CHANNEL."
    curl -fLsS --retry 3 --connect-timeout 15 "$RELEASE_URL/$ASSET" -o "$INSTALL_TEMP_DIR/$ASSET" \
        || fail "Нативный архив $CHANNEL не опубликован в GitHub Releases."
    curl -fLsS --retry 3 --connect-timeout 15 "$RELEASE_URL/$ASSET.sha256" -o "$INSTALL_TEMP_DIR/$ASSET.sha256" \
        || fail 'Не удалось скачать файл SHA-256.'
    name="$(verify_archive "$INSTALL_TEMP_DIR")"
    archive="$INSTALL_TEMP_DIR/$ASSET"
    release="$APP_DIR/releases/$name"
    if [[ ! -d "$release" ]]; then
        info "Распаковываю $name"
        tar --no-same-owner -xzf "$archive" -C "$INSTALL_TEMP_DIR"
        for path in bin/cheenhub_backend bin/cheenhub_migrations web-public/index.html SHA256SUMS; do
            [[ -s "$INSTALL_TEMP_DIR/$name/$path" ]] || fail "В релизе отсутствует $path"
        done
        (cd "$INSTALL_TEMP_DIR/$name" && sha256sum --check --status SHA256SUMS) || fail 'Внутренние контрольные суммы неверны.'
        verify_release_metadata "$INSTALL_TEMP_DIR/$name"
        mv "$INSTALL_TEMP_DIR/$name" "$release"
    else
        (cd "$release" && sha256sum --check --status SHA256SUMS) || fail 'Ранее установленный релиз повреждён.'
        verify_release_metadata "$release"
        info "Релиз $name уже скачан."
    fi
    write_config
    write_unit
    if is_system_install; then
        configure_tls_renewal
        info 'Применяю миграции к существующей production-базе.'
        systemd-run --quiet --wait --pipe --collect --unit="cheenhub-migrate-$$" \
            --property="EnvironmentFile=$CONFIG_FILE" --property=DynamicUser=yes \
            --working-directory="$release" "$release/bin/cheenhub_migrations" up \
            || fail 'Миграции не выполнены; текущая версия приложения продолжает работать.'
    fi
    if [[ -L "$APP_DIR/current" ]]; then
        old_link="$(readlink "$APP_DIR/current")"
    elif [[ -e "$APP_DIR/current" ]]; then
        fail 'Путь current существует и не является ссылкой. Отказываюсь перезаписывать его.'
    fi
    next_link="$APP_DIR/.current-next-$$"
    ln -s "$release" "$next_link"
    mv -Tf "$next_link" "$APP_DIR/current"
    service_control daemon-reload
    service_control enable "$SERVICE" >/dev/null
    info 'Запускаю сервис.'
    if service_control restart "$SERVICE" && healthcheck; then
        if is_system_install; then
            info 'Готово: https://cheenhub.ru — продление TLS и горячее применение настроены.'
        else
            info 'Готово: http://127.0.0.1:3000'
            printf '%s\n' 'Внимание: AUTH_STORE=inmemory — данные не сохраняются после рестарта.'
        fi
        info "Настройки: $CONFIG_FILE"
    else
        printf 'Новая версия не запустилась.\n' >&2
        if [[ -n "$old_link" ]]; then
            ln -s "$old_link" "$next_link"
            mv -Tf "$next_link" "$APP_DIR/current"
            service_control restart "$SERVICE" || true
            info 'Возвращён предыдущий релиз.'
        else
            service_control stop "$SERVICE" || true
            rm -f "$APP_DIR/current"
        fi
        service_control status "$SERVICE" --no-pager >&2 || true
        exit 1
    fi
}

configure_tls_renewal() {
    local letsencrypt_dir webroot_dir marker hook lineage
    # Подключает существующий Certbot в Docker к нативному production-сервису.
    if [[ $EUID -ne 0 ]]; then
        echo 'Run this script as root on the production server.' >&2
        exit 1
    fi

    letsencrypt_dir=/var/lib/docker/volumes/cheenhub_letsencrypt/_data
    webroot_dir=/var/lib/docker/volumes/cheenhub_certbot-webroot/_data
    marker="$webroot_dir/.cheenhub-tls-renewed"
    hook="$letsencrypt_dir/renewal-hooks/deploy/50-cheenhub-native"

    test -r "$letsencrypt_dir/live/cheenhub.ru/fullchain.pem"
    test -r "$letsencrypt_dir/live/cheenhub.ru/privkey.pem"
    lineage=$(basename "$(readlink -f "$letsencrypt_dir/live/cheenhub.ru")")
    test -r "$letsencrypt_dir/renewal/$lineage.conf"
    test -d "$webroot_dir"
    test -f "$CONFIG_FILE"
    grep -q '^WEBTRANSPORT_TLS_CERT_PATH=' "$CONFIG_FILE"
    grep -q '^WEBTRANSPORT_TLS_KEY_PATH=' "$CONFIG_FILE"

    if systemctl is-active --quiet cheenhub-tls-renewal.path; then
        systemctl stop cheenhub-tls-renewal.path
    fi

    if ! getent group cheenhub-tls >/dev/null; then
        groupadd --system cheenhub-tls
    fi
    install -d -m 755 /etc/cheenhub /usr/local/libexec
    install -d -m 750 -o root -g cheenhub-tls /etc/cheenhub/tls

    cat >/usr/local/libexec/cheenhub-sync-tls <<'SYNC'
#!/usr/bin/env bash
set -euo pipefail

# Только root публикует проверенную пару; приложение получает доступ на чтение.
exec 9>/run/lock/cheenhub-sync-tls.lock
flock 9
source_dir=/var/lib/docker/volumes/cheenhub_letsencrypt/_data/live/cheenhub.ru
target_dir=/etc/cheenhub/tls
staging=$(mktemp -d "$target_dir/.update.XXXXXXXX")
trap 'rm -f "$staging/fullchain.pem" "$staging/privkey.pem"; rmdir "$staging"' EXIT
install -m 640 -o root -g cheenhub-tls "$source_dir/fullchain.pem" "$staging/fullchain.pem"
install -m 640 -o root -g cheenhub-tls "$source_dir/privkey.pem" "$staging/privkey.pem"
openssl verify -purpose sslserver -verify_hostname cheenhub.ru \
    -untrusted "$staging/fullchain.pem" "$staging/fullchain.pem" >/dev/null
cert_public=$(openssl x509 -in "$staging/fullchain.pem" -pubkey -noout |
    openssl pkey -pubin -outform DER | sha256sum)
key_public=$(openssl pkey -in "$staging/privkey.pem" -pubout -outform DER | sha256sum)
if [[ "$cert_public" != "$key_public" ]]; then
    echo 'CheenHub: TLS certificate and private key do not match; keeping current pair.' >&2
    exit 1
fi
if cmp -s "$staging/fullchain.pem" "$target_dir/fullchain.pem" &&
    cmp -s "$staging/privkey.pem" "$target_dir/privkey.pem"; then
    echo 'CheenHub: TLS files unchanged; application restart is not required.'
    exit 0
fi

# Оба TLS-наблюдателя отклоняют неполную пару и ждут двух стабильных чтений.
mv -f "$staging/fullchain.pem" "$target_dir/fullchain.pem"
mv -f "$staging/privkey.pem" "$target_dir/privkey.pem"
echo 'CheenHub: TLS files published for HTTPS and WebTransport hot reload.'
SYNC
    chmod 755 /usr/local/libexec/cheenhub-sync-tls
    /usr/local/libexec/cheenhub-sync-tls

    install -d -m 755 "$(dirname "$hook")"
    cat >"$hook" <<'HOOK'
#!/bin/sh
set -eu

# Учитываем ссылку cheenhub.ru на фактический lineage cheenhub.ru-0001.
if [ -z "${RENEWED_LINEAGE:-}" ] ||
    [ "$(readlink -f "$RENEWED_LINEAGE")" != "$(readlink -f /etc/letsencrypt/live/cheenhub.ru)" ]; then
    exit 0
fi

# Сигнал сохраняется до успешного применения, в том числе при перезагрузке хоста.
: > /var/www/certbot/.cheenhub-tls-renewed
echo 'CheenHub: renewed TLS certificate queued for hot reload.'
HOOK
    chmod 755 "$hook"

    cat >/etc/systemd/system/cheenhub-tls-renewal.path <<UNIT
[Unit]
Description=Watch for renewed CheenHub TLS certificate

[Path]
PathExists=$marker
Unit=cheenhub-tls-renewal.service

[Install]
WantedBy=multi-user.target
UNIT

    cat >/etc/systemd/system/cheenhub-tls-renewal.service <<UNIT
[Unit]
Description=Publish renewed CheenHub TLS files for hot reload
StartLimitIntervalSec=300
StartLimitBurst=3

[Service]
Type=oneshot
ExecStart=/usr/local/libexec/cheenhub-sync-tls
ExecStartPost=/usr/bin/rm -f $marker
Restart=on-failure
RestartSec=15
UNIT

    install -d -m 755 /etc/systemd/system/cheenhub.service.d
    cat >/etc/systemd/system/cheenhub.service.d/tls.conf <<'UNIT'
[Service]
SupplementaryGroups=cheenhub-tls
UNIT
    sed -i \
        -e 's#^WEBTRANSPORT_TLS_CERT_PATH=.*#WEBTRANSPORT_TLS_CERT_PATH=/etc/cheenhub/tls/fullchain.pem#' \
        -e 's#^WEBTRANSPORT_TLS_KEY_PATH=.*#WEBTRANSPORT_TLS_KEY_PATH=/etc/cheenhub/tls/privkey.pem#' \
        "$CONFIG_FILE"

    systemd-analyze verify /etc/systemd/system/cheenhub-tls-renewal.path \
        /etc/systemd/system/cheenhub-tls-renewal.service /etc/systemd/system/cheenhub.service
    systemctl daemon-reload
    # Приложение перезапускается один раз общим процессом установки/обновления.
    systemctl enable --now cheenhub-tls-renewal.path
    echo 'CheenHub: automatic TLS renewal and hot reload are configured.'
}

usage() {
    cat <<'HELP'
CheenHub Native Installer (Linux x86_64)
  От root: текущий production cheenhub.ru, существующие .env, БД и Certbot
  От обычного пользователя: локальный HTTP-preview
  install.sh                 Интерактивное меню
  install.sh install         Установить или обновить из GitHub Releases
  install.sh update          То же, что install
  install.sh update --version vX.Y.Z   Установить конкретный релиз
  --expected-commit SHA      Проверить SHA коммита архива (для CI)
  install.sh status          Состояние сервиса
  install.sh restart         Перезапустить сервис
  install.sh stop            Остановить сервис
  install.sh logs            Последние записи журнала
HELP
}

main() {
    initialize_paths
    local action=''
    while [[ $# -gt 0 ]]; do
        case "$1" in
            --version|--expected-commit)
                [[ $# -ge 2 ]] || fail "После $1 требуется значение."
                if [[ "$1" == --version ]]; then RELEASE_VERSION="$2"; else EXPECTED_COMMIT="$2"; fi
                shift 2 ;;
            *) [[ -z "$action" ]] || fail "Лишний аргумент: $1"; action="$1"; shift ;;
        esac
    done
    [[ -z "$RELEASE_VERSION" || "$RELEASE_VERSION" =~ ^v[0-9]+\.[0-9]+\.[0-9]+([+-][0-9A-Za-z.-]+)?$ ]] || fail 'Некорректный тег версии.'
    [[ -z "$EXPECTED_COMMIT" || "$EXPECTED_COMMIT" =~ ^[0-9a-f]{40}$ ]] || fail 'Ожидается полный SHA коммита.'
    [[ -z "$EXPECTED_COMMIT" || -n "$RELEASE_VERSION" ]] || fail 'Проверка SHA требует --version.'
    if [[ -z "$action" && -t 0 ]]; then
        printf '\n=== CheenHub ===\n1) Установить / обновить\n2) Статус\n3) Перезапустить\n4) Остановить\n5) Логи\n6) Выход\n'
        read -r -p 'Выберите действие [1]: ' action
        action="${action:-1}"
    fi
    action="${action:-install}"
    case "$action" in
        1|install|update) select_release_source; install_or_update ;;
        2|status) service_control status "$SERVICE" --no-pager ;;
        3|restart) service_control restart "$SERVICE" ;;
        4|stop) service_control stop "$SERVICE" ;;
        5|logs)
            if is_system_install; then journalctl -u "$SERVICE" --no-pager -n 80
            else journalctl --user -u "$SERVICE" --no-pager -n 80; fi ;;
        6|exit) exit 0 ;;
        -h|--help|help) usage ;;
        *) usage; exit 2 ;;
    esac
}

main "$@"
