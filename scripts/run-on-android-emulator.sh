#!/usr/bin/env bash

set -euo pipefail

if (( $# == 0 )); then
    printf 'Не указана команда для запуска в Android-эмуляторе.\n' >&2
    exit 2
fi

sdk_root="${ANDROID_HOME:-${ANDROID_SDK_ROOT:-/home/loject/Android/Sdk}}"
adb_bin="${sdk_root}/platform-tools/adb"
emulator_bin="${sdk_root}/emulator/emulator"
avd_name="${CHEENHUB_ANDROID_EMULATOR_AVD:-CheenHub_API_35}"
minimum_api=30
script_dir="$(cd -- "$(dirname -- "${BASH_SOURCE[0]}")" && pwd)"
repository_root="$(cd -- "${script_dir}/.." && pwd)"
android_target_dir="${CHEENHUB_ANDROID_EMULATOR_TARGET_DIR:-${repository_root}/target/android-emulator}"

if [[ ! -x "${adb_bin}" ]]; then
    printf 'ADB не найден: %s\n' "${adb_bin}" >&2
    exit 1
fi
if [[ ! -x "${emulator_bin}" ]]; then
    printf 'Android Emulator не найден: %s\n' "${emulator_bin}" >&2
    exit 1
fi

selected_emulator=""
while read -r serial; do
    [[ -n "${serial}" ]] || continue
    running_avd="$(
        { "${adb_bin}" -s "${serial}" emu avd name 2>/dev/null || true; } |
            tr -d '\r' |
            head -n 1
    )"
    if [[ "${running_avd}" == "${avd_name}" ]]; then
        selected_emulator="${serial}"
        break
    fi
done < <("${adb_bin}" devices | awk '$1 ~ /^emulator-/ { print $1 }')

emulator_pid=""
emulator_log=""
if [[ -z "${selected_emulator}" ]]; then
    if ! "${emulator_bin}" -list-avds | grep -Fxq "${avd_name}"; then
        printf 'Android AVD %s не найден.\n' "${avd_name}" >&2
        printf 'Доступные AVD:\n' >&2
        "${emulator_bin}" -list-avds >&2
        exit 1
    fi

    if [[ -n "${CHEENHUB_ANDROID_EMULATOR_PORT:-}" ]]; then
        emulator_port="${CHEENHUB_ANDROID_EMULATOR_PORT}"
        if [[ ! "${emulator_port}" =~ ^[0-9]+$ ]] ||
            (( emulator_port < 5554 || emulator_port > 5682 || emulator_port % 2 != 0 )); then
            printf 'CHEENHUB_ANDROID_EMULATOR_PORT должен быть чётным портом 5554-5682.\n' >&2
            exit 1
        fi
    else
        emulator_port=""
        for ((candidate = 5554; candidate <= 5682; candidate += 2)); do
            if ! "${adb_bin}" devices |
                awk '$1 ~ /^emulator-/ { print $1 }' |
                grep -Fxq "emulator-${candidate}"; then
                emulator_port="${candidate}"
                break
            fi
        done
        if [[ -z "${emulator_port}" ]]; then
            printf 'Не найден свободный Android emulator port в диапазоне 5554-5682.\n' >&2
            exit 1
        fi
    fi

    selected_emulator="emulator-${emulator_port}"
    emulator_log="${TMPDIR:-/tmp}/cheenhub-android-emulator-${emulator_port}.log"
    printf 'Запускаю Android AVD %s как %s.\n' "${avd_name}" "${selected_emulator}"
    nohup "${emulator_bin}" \
        -avd "${avd_name}" \
        -port "${emulator_port}" \
        -netdelay none \
        -netspeed full \
        </dev/null >"${emulator_log}" 2>&1 &
    emulator_pid=$!
fi

printf 'Ожидаю загрузку Android-эмулятора %s.\n' "${selected_emulator}"
boot_completed=""
deadline=$((SECONDS + 180))
while (( SECONDS < deadline )); do
    if [[ -n "${emulator_pid}" ]] && ! kill -0 "${emulator_pid}" 2>/dev/null; then
        printf 'Android AVD %s завершился во время запуска.\n' "${avd_name}" >&2
        if [[ -n "${emulator_log}" ]]; then
            tail -n 80 "${emulator_log}" >&2
        fi
        exit 1
    fi

    boot_completed="$(
        {
            "${adb_bin}" -s "${selected_emulator}" \
                shell getprop sys.boot_completed 2>/dev/null || true
        } |
            tr -d '\r'
    )"
    if [[ "${boot_completed}" == "1" ]]; then
        break
    fi
    sleep 1
done

if [[ "${boot_completed}" != "1" ]]; then
    printf 'Android AVD %s не загрузился за 180 секунд.\n' "${avd_name}" >&2
    if [[ -n "${emulator_log}" ]]; then
        tail -n 80 "${emulator_log}" >&2
    fi
    exit 1
fi

api_level="$(
    "${adb_bin}" -s "${selected_emulator}" shell getprop ro.build.version.sdk |
        tr -d '\r'
)"
if [[ ! "${api_level}" =~ ^[0-9]+$ ]] || (( api_level < minimum_api )); then
    printf 'CheenHub требует эмулятор API %d+, но %s использует API %s.\n' \
        "${minimum_api}" "${avd_name}" "${api_level:-unknown}" >&2
    exit 1
fi

export ANDROID_SERIAL="${selected_emulator}"
export CARGO_TARGET_DIR="${android_target_dir}"
printf 'Запускаю CheenHub в %s (API %s, %s).\n' \
    "${avd_name}" "${api_level}" "${selected_emulator}"
printf 'Использую изолированный каталог сборки: %s.\n' "${CARGO_TARGET_DIR}"
exec "$@" --device "${selected_emulator}"
