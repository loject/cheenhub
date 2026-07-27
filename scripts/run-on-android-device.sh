#!/usr/bin/env bash

set -euo pipefail

if (( $# == 0 )); then
    printf 'Не указана команда для запуска на Android-устройстве.\n' >&2
    exit 2
fi

sdk_root="${ANDROID_HOME:-${ANDROID_SDK_ROOT:-/home/loject/Android/Sdk}}"
adb_bin="${sdk_root}/platform-tools/adb"
script_dir="$(cd -- "$(dirname -- "${BASH_SOURCE[0]}")" && pwd)"
repository_root="$(cd -- "${script_dir}/.." && pwd)"
android_target_dir="${CHEENHUB_ANDROID_DEVICE_TARGET_DIR:-${repository_root}/target/android-device}"

if [[ ! -x "${adb_bin}" ]]; then
    printf 'ADB не найден: %s\n' "${adb_bin}" >&2
    exit 1
fi

mapfile -t physical_devices < <(
    "${adb_bin}" devices |
        awk '$2 == "device" && $1 !~ /^emulator-/ { print $1 }'
)

if [[ -n "${ANDROID_SERIAL:-}" ]]; then
    selected_device=""
    for device in "${physical_devices[@]}"; do
        if [[ "${device}" == "${ANDROID_SERIAL}" ]]; then
            selected_device="${device}"
            break
        fi
    done
    if [[ -z "${selected_device}" ]]; then
        printf 'ANDROID_SERIAL=%s не является подключённым физическим устройством.\n' \
            "${ANDROID_SERIAL}" >&2
        exit 1
    fi
elif (( ${#physical_devices[@]} == 1 )); then
    selected_device="${physical_devices[0]}"
elif (( ${#physical_devices[@]} == 0 )); then
    printf 'Подключённый физический Android-телефон не найден.\n' >&2
    printf 'Проверь USB debugging и подтверждение RSA на телефоне.\n' >&2
    exit 1
else
    printf 'Подключено несколько физических Android-устройств: %s\n' \
        "${physical_devices[*]}" >&2
    printf 'Укажи нужное через ANDROID_SERIAL.\n' >&2
    exit 1
fi

export ANDROID_SERIAL="${selected_device}"
export CARGO_TARGET_DIR="${android_target_dir}"
printf 'Запускаю CheenHub на физическом Android-устройстве %s.\n' "${selected_device}"
printf 'Использую изолированный каталог сборки: %s.\n' "${CARGO_TARGET_DIR}"
exec "$@" --device "${selected_device}"
