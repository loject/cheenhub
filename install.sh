#!/usr/bin/env bash
set -euo pipefail

# Установка и обновление CheenHub для текущего пользователя без Docker и Rust.
# На первом этапе используется локальный preview без постоянного хранилища.

readonly REPOSITORY='loject/cheenhub'
readonly CHANNEL='native-preview'
readonly ASSET='cheenhub-native-linux-x86_64.tar.gz'
readonly RELEASE_URL="https://github.com/${REPOSITORY}/releases/download/${CHANNEL}"
readonly APP_DIR="${HOME}/.local/share/cheenhub"
readonly CONFIG_DIR="${HOME}/.config/cheenhub"
readonly UNIT_DIR="${HOME}/.config/systemd/user"
readonly SERVICE='cheenhub.service'

fail() { printf 'Ошибка: %s\n' "$*" >&2; exit 1; }
info() { printf '==> %s\n' "$*"; }

requirements() {
    [[ "$(uname -s)" == Linux && "$(uname -m)" == x86_64 ]] || fail 'Поддерживается только Linux x86_64.'
    [[ "${EUID}" -ne 0 ]] || fail 'Запускайте от обычного пользователя без sudo.'
    local command
    for command in curl tar sha256sum mktemp openssl systemctl; do
        command -v "$command" >/dev/null 2>&1 || fail "Не найдена утилита $command. Установите её системным пакетным менеджером."
    done
    systemctl --user show-environment >/dev/null 2>&1 || fail 'Нет пользовательской сессии systemd. Войдите по SSH обычным пользователем.'
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
    if [[ ! -e "$CONFIG_DIR/backend.env" ]]; then
        info 'Создаю локальную тестовую конфигурацию.'
        (umask 077; cat > "$CONFIG_DIR/backend.env" <<CONFIG
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
    for attempt in {1..15}; do
        if systemctl --user is-active --quiet "$SERVICE" && \
           curl --noproxy '*' --silent --fail --max-time 2 http://127.0.0.1:3000/health >/dev/null; then
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
    INSTALL_TEMP_DIR="$(mktemp -d)"
    trap 'if [[ -n "${INSTALL_TEMP_DIR:-}" ]]; then rm -rf -- "$INSTALL_TEMP_DIR"; fi' EXIT
    info 'Скачиваю последнюю нативную сборку CheenHub с GitHub Releases.'
    curl -fLsS --retry 3 --connect-timeout 15 "$RELEASE_URL/$ASSET" -o "$INSTALL_TEMP_DIR/$ASSET" \
        || fail 'Нативный релиз пока не опубликован. Запустите Build native CheenHub Linux archive в GitHub Actions.'
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
        mv "$INSTALL_TEMP_DIR/$name" "$release"
    else
        (cd "$release" && sha256sum --check --status SHA256SUMS) || fail 'Ранее установленный релиз повреждён.'
        info "Релиз $name уже скачан."
    fi
    write_config
    write_unit
    if [[ -L "$APP_DIR/current" ]]; then
        old_link="$(readlink "$APP_DIR/current")"
    elif [[ -e "$APP_DIR/current" ]]; then
        fail 'Путь current существует и не является ссылкой. Отказываюсь перезаписывать его.'
    fi
    next_link="$APP_DIR/.current-next-$$"
    ln -s "$release" "$next_link"
    mv -Tf "$next_link" "$APP_DIR/current"
    systemctl --user daemon-reload
    systemctl --user enable "$SERVICE" >/dev/null
    info 'Запускаю сервис.'
    if systemctl --user restart "$SERVICE" && healthcheck; then
        info 'Готово: http://127.0.0.1:3000'
        info 'Настройки: ~/.config/cheenhub/backend.env'
        printf '%s\n' 'Внимание: AUTH_STORE=inmemory — данные не сохраняются после рестарта.'
    else
        printf 'Новая версия не запустилась.\n' >&2
        if [[ -n "$old_link" ]]; then
            ln -s "$old_link" "$next_link"
            mv -Tf "$next_link" "$APP_DIR/current"
            systemctl --user restart "$SERVICE" || true
            info 'Возвращён предыдущий релиз.'
        else
            systemctl --user stop "$SERVICE" || true
            rm -f "$APP_DIR/current"
        fi
        systemctl --user status "$SERVICE" --no-pager >&2 || true
        exit 1
    fi
}

usage() {
    cat <<'HELP'
CheenHub Native Installer (Linux x86_64)
  install.sh                 Интерактивное меню
  install.sh install         Установить или обновить из GitHub Releases
  install.sh update          То же, что install
  install.sh status          Состояние сервиса
  install.sh restart         Перезапустить сервис
  install.sh stop            Остановить сервис
  install.sh logs            Последние записи журнала
HELP
}

main() {
    local action="${1:-}"
    if [[ $# -gt 1 ]]; then usage; exit 2; fi
    if [[ -z "$action" && -t 0 ]]; then
        printf '\n=== CheenHub ===\n1) Установить / обновить\n2) Статус\n3) Перезапустить\n4) Остановить\n5) Логи\n6) Выход\n'
        read -r -p 'Выберите действие [1]: ' action
        action="${action:-1}"
    fi
    action="${action:-install}"
    case "$action" in
        1|install|update) install_or_update ;;
        2|status) systemctl --user status "$SERVICE" --no-pager ;;
        3|restart) systemctl --user restart "$SERVICE" ;;
        4|stop) systemctl --user stop "$SERVICE" ;;
        5|logs) journalctl --user -u "$SERVICE" --no-pager -n 80 ;;
        6|exit) exit 0 ;;
        -h|--help|help) usage ;;
        *) usage; exit 2 ;;
    esac
}

main "$@"
