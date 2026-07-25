#!/usr/bin/env bash

set -euo pipefail

avd_name="${CHEENHUB_ANDROID_AVD:-CheenHub_API_35}"
emulator_serial="${CHEENHUB_ANDROID_SERIAL:-emulator-5554}"
sdk_root="${ANDROID_HOME:-${ANDROID_SDK_ROOT:-${HOME}/Android/Sdk}}"
emulator_bin="${sdk_root}/emulator/emulator"
adb_bin="${sdk_root}/platform-tools/adb"
emulator_log="${TMPDIR:-/tmp}/cheenhub-android-emulator.log"

if [[ ! -x "${emulator_bin}" || ! -x "${adb_bin}" ]]; then
    printf 'Android SDK не найден в %s. Проверьте ANDROID_HOME.\n' "${sdk_root}" >&2
    exit 1
fi

export ANDROID_HOME="${sdk_root}"
export ANDROID_SDK_ROOT="${sdk_root}"
export ANDROID_SERIAL="${emulator_serial}"
export JAVA_HOME="${JAVA_HOME:-/opt/android-studio/jbr}"

if [[ -z "${ANDROID_NDK_HOME:-}" && -d "${sdk_root}/ndk/27.2.12479018" ]]; then
    export ANDROID_NDK_HOME="${sdk_root}/ndk/27.2.12479018"
    export ANDROID_NDK_ROOT="${ANDROID_NDK_HOME}"
fi

if "${adb_bin}" -s "${emulator_serial}" get-state >/dev/null 2>&1; then
    running_avd="$("${adb_bin}" -s "${emulator_serial}" emu avd name 2>/dev/null | tr -d '\r' | head -n 1)"
    if [[ "${running_avd}" != "${avd_name}" ]]; then
        printf '%s уже занят эмулятором %s, ожидался %s.\n' \
            "${emulator_serial}" "${running_avd:-с неизвестным AVD}" "${avd_name}" >&2
        exit 1
    fi
else
    printf 'Запускаю Android-эмулятор %s (%s)...\n' "${avd_name}" "${emulator_serial}"
    "${emulator_bin}" \
        -avd "${avd_name}" \
        -port "${emulator_serial#emulator-}" \
        -gpu host \
        -no-snapshot-save \
        >"${emulator_log}" 2>&1 &
fi

printf 'Ожидаю загрузку Android...\n'
for _ in $(seq 1 180); do
    if [[ "$("${adb_bin}" -s "${emulator_serial}" shell getprop sys.boot_completed 2>/dev/null | tr -d '\r')" == "1" ]]; then
        break
    fi
    sleep 1
done

if [[ "$("${adb_bin}" -s "${emulator_serial}" shell getprop sys.boot_completed 2>/dev/null | tr -d '\r')" != "1" ]]; then
    printf 'Эмулятор не загрузился за 180 секунд. Журнал: %s\n' "${emulator_log}" >&2
    exit 1
fi

"${adb_bin}" -s "${emulator_serial}" shell input keyevent 82 >/dev/null 2>&1 || true
"${adb_bin}" -s "${emulator_serial}" reverse tcp:3000 tcp:3000
"${adb_bin}" -s "${emulator_serial}" reverse tcp:8080 tcp:8080
printf 'Порты backend и dev-сервера направлены в эмулятор через ADB.\n'
printf 'Android загружен. Собираю, устанавливаю и запускаю CheenHub...\n'

exec dx serve \
    --android \
    --device "${avd_name}" \
    --target x86_64-linux-android \
    --package cheenhub_client \
    --bin cheen_hub \
    --no-default-features \
    --features mobile
