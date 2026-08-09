#!/usr/bin/env bash

set -euo pipefail

workspace_root="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "${workspace_root}"

sdk_root="${ANDROID_HOME:-${ANDROID_SDK_ROOT:-${HOME}/Android/Sdk}}"
adb_bin="${sdk_root}/platform-tools/adb"
production_env="${workspace_root}/.env.production"
bundle_out_dir="${workspace_root}/target/cheenhub-production-android"

export PATH="${HOME}/.local/bin:${PATH}"

if [[ ! -x "${adb_bin}" ]]; then
    printf 'ADB не найден в %s. Проверьте ANDROID_HOME.\n' "${sdk_root}" >&2
    exit 1
fi

"${workspace_root}/scripts/ensure-cargo-cli.sh" dioxus-cli dx 0.7.5

if [[ ! -f "${production_env}" ]]; then
    printf 'Не найден %s с production JWT-конфигурацией.\n' "${production_env}" >&2
    exit 1
fi

mapfile -t usb_devices < <("${adb_bin}" devices | awk '$2 == "device" && $1 !~ /^emulator-/ { print $1 }')
case "${#usb_devices[@]}" in
    0)
        printf 'Нет авторизованного USB-устройства. Включите USB debugging и подтвердите RSA-запрос на телефоне.\n' >&2
        exit 1
        ;;
    1)
        device_serial="${usb_devices[0]}"
        ;;
    *)
        printf 'Подключено несколько авторизованных USB-устройств: %s. Оставьте подключенным ровно одно.\n' "${usb_devices[*]}" >&2
        exit 1
        ;;
esac

device_abi="$("${adb_bin}" -s "${device_serial}" shell getprop ro.product.cpu.abi | tr -d '\r')"
if [[ "${device_abi}" != "arm64-v8a" ]]; then
    printf 'Устройство %s использует ABI %s, а production APK собирается для arm64-v8a.\n' \
        "${device_serial}" "${device_abi:-неизвестный}" >&2
    exit 1
fi

mapfile -d '' -t production_values < <(
    set +u
    source "${production_env}"
    printf '%s\0' \
        "${CHEENHUB_BASE_URL:-https://cheenhub.ru}" \
        "${CHEENHUB_REALTIME_CERT_SHA256:-}" \
        "${CHEENHUB_JWT_KEY_ID:-prod-ed25519-1}" \
        "${CHEENHUB_JWT_PUBLIC_KEY_BASE64:-}"
)

if [[ "${#production_values[@]}" -ne 4 ]]; then
    printf 'Не удалось прочитать client production-конфигурацию из %s.\n' "${production_env}" >&2
    exit 1
fi

if [[ -z "${production_values[3]}" ]]; then
    printf 'В %s отсутствует CHEENHUB_JWT_PUBLIC_KEY_BASE64.\n' "${production_env}" >&2
    exit 1
fi

case "${production_values[0]}" in
    https://* | http://*) ;;
    *)
        printf 'Некорректный CHEENHUB_BASE_URL в %s: ожидается http:// или https://.\n' \
            "${production_env}" >&2
        exit 1
        ;;
esac

release_tag="$(cargo run --quiet -p xtask -- release-version print-tag)"
commit_sha="$(git rev-parse --short=7 HEAD)"
if git describe --exact-match --tags --match "${release_tag}" HEAD >/dev/null 2>&1; then
    app_version="${release_tag}"
else
    app_version="${release_tag}-${commit_sha}"
fi

export ANDROID_HOME="${sdk_root}"
export ANDROID_SDK_ROOT="${sdk_root}"
export JAVA_HOME="${JAVA_HOME:-/opt/android-studio/jbr}"
export CMAKE_GENERATOR="Ninja"
export CHEENHUB_BASE_URL="${production_values[0]}"
export CHEENHUB_REALTIME_CERT_SHA256="${production_values[1]}"
export CHEENHUB_JWT_KEY_ID="${production_values[2]}"
export CHEENHUB_JWT_PUBLIC_KEY_BASE64="${production_values[3]}"
export CHEENHUB_APP_VERSION="${app_version}"

if [[ -z "${ANDROID_NDK_HOME:-}" && -d "${sdk_root}/ndk/27.2.12479018" ]]; then
    export ANDROID_NDK_HOME="${sdk_root}/ndk/27.2.12479018"
    export ANDROID_NDK_ROOT="${ANDROID_NDK_HOME}"
fi

device_model="$("${adb_bin}" -s "${device_serial}" shell getprop ro.product.model | tr -d '\r')"
printf 'Собираю CheenHub %s для %s (%s) с production-конфигурацией.\n' \
    "${app_version}" "${device_model:-Android}" "${device_serial}"

dx bundle \
    --android \
    --target aarch64-linux-android \
    --package cheenhub_client \
    --bin cheen_hub \
    --release \
    --no-default-features \
    --features mobile \
    --package-types apk \
    --out-dir "${bundle_out_dir}"

mapfile -t apk_files < <(find "${bundle_out_dir}" -maxdepth 1 -type f -name '*.apk' -print)
if [[ "${#apk_files[@]}" -ne 1 ]]; then
    printf 'Ожидался один APK в %s, найдено: %s.\n' "${bundle_out_dir}" "${#apk_files[@]}" >&2
    exit 1
fi

printf 'Устанавливаю %s через adb install -r. Данные приложения не удаляются.\n' "${apk_files[0]}"
set +e
install_output="$("${adb_bin}" -s "${device_serial}" install -r "${apk_files[0]}" 2>&1)"
install_status=$?
set -e
printf '%s\n' "${install_output}"

if [[ "${install_status}" -eq 0 ]]; then
    printf 'CheenHub %s установлен на %s.\n' "${app_version}" "${device_serial}"
    exit 0
fi

if [[ "${install_output}" == *"INSTALL_FAILED_UPDATE_INCOMPATIBLE"* ]]; then
    printf '%s\n' \
        'Подпись локального release APK не совпадает с подписью установленного pipeline APK.' \
        'Задача намеренно не удаляет приложение и его данные. Для обновления нужен APK, подписанный тем же production-ключом.' >&2
fi

exit "${install_status}"
