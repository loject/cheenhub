#!/usr/bin/env bash

set -euo pipefail

sdk_root="${ANDROID_HOME:-${ANDROID_SDK_ROOT:-${HOME}/Android/Sdk}}"
adb_bin="${sdk_root}/platform-tools/adb"

if [[ ! -x "${adb_bin}" ]]; then
    printf 'ADB не найден в %s. Проверьте ANDROID_HOME.\n' "${sdk_root}" >&2
    exit 1
fi

export ANDROID_HOME="${sdk_root}"
export ANDROID_SDK_ROOT="${sdk_root}"
export JAVA_HOME="${JAVA_HOME:-/opt/android-studio/jbr}"

if [[ -z "${ANDROID_NDK_HOME:-}" && -d "${sdk_root}/ndk/27.2.12479018" ]]; then
    export ANDROID_NDK_HOME="${sdk_root}/ndk/27.2.12479018"
    export ANDROID_NDK_ROOT="${ANDROID_NDK_HOME}"
fi

if [[ -n "${CHEENHUB_ANDROID_DEVICE:-}" ]]; then
    device_serial="${CHEENHUB_ANDROID_DEVICE}"
    if [[ "$("${adb_bin}" -s "${device_serial}" get-state 2>/dev/null || true)" != "device" ]]; then
        printf 'USB-устройство %s не найдено или не авторизовано в ADB.\n' "${device_serial}" >&2
        exit 1
    fi
else
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
            printf 'Подключено несколько USB-устройств: %s. Задайте CHEENHUB_ANDROID_DEVICE.\n' "${usb_devices[*]}" >&2
            exit 1
            ;;
    esac
fi

device_abi="$("${adb_bin}" -s "${device_serial}" shell getprop ro.product.cpu.abi | tr -d '\r')"
if [[ "${device_abi}" != arm64-v8a ]]; then
    printf 'Устройство %s использует ABI %s, а задача рассчитана на arm64-v8a.\n' \
        "${device_serial}" "${device_abi:-неизвестный}" >&2
    exit 1
fi

export ANDROID_SERIAL="${device_serial}"
"${adb_bin}" -s "${device_serial}" reverse tcp:3000 tcp:3000 >/dev/null
"${adb_bin}" -s "${device_serial}" reverse tcp:8080 tcp:8080 >/dev/null

device_model="$("${adb_bin}" -s "${device_serial}" shell getprop ro.product.model | tr -d '\r')"
printf 'Запускаю CheenHub на USB-устройстве %s (%s).\n' "${device_model:-Android}" "${device_serial}"

exec dx serve \
    --android \
    --device "${device_serial}" \
    --target aarch64-linux-android \
    --package cheenhub_client \
    --bin cheen_hub \
    --no-default-features \
    --features mobile
