#!/usr/bin/env bash

set -euo pipefail

if [[ "$#" -ne 3 ]]; then
    printf 'Использование: %s <crate> <binary> <version>.\n' "$0" >&2
    exit 2
fi

crate_name="$1"
binary_name="$2"
expected_version="$3"
install_root="${CARGO_INSTALL_ROOT:-${HOME}/.local}"

has_expected_version() {
    local binary_path="$1"
    local expected_version_pattern="${expected_version//./\\.}"
    local version_output

    version_output="$("${binary_path}" --version 2>/dev/null)" || return 1
    [[ "${version_output}" =~ (^|[[:space:]])v?${expected_version_pattern}($|[[:space:]]) ]]
}

if binary_path="$(command -v "${binary_name}" 2>/dev/null)" \
    && has_expected_version "${binary_path}"; then
    exit 0
fi

if ! command -v cargo >/dev/null 2>&1; then
    printf 'Cargo не найден: невозможно установить %s %s.\n' \
        "${crate_name}" "${expected_version}" >&2
    exit 1
fi

printf '%s %s не найден. Устанавливаю его в %s.\n' \
    "${crate_name}" "${expected_version}" "${install_root}"
cargo install \
    --locked \
    --force \
    --root "${install_root}" \
    --version "${expected_version}" \
    "${crate_name}"

installed_binary="${install_root}/bin/${binary_name}"
if [[ ! -x "${installed_binary}" ]] || ! has_expected_version "${installed_binary}"; then
    printf '%s %s установлен некорректно: %s не найден или имеет другую версию.\n' \
        "${crate_name}" "${expected_version}" "${installed_binary}" >&2
    exit 1
fi
