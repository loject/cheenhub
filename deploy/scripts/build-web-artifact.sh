#!/usr/bin/env bash
set -euo pipefail

env_file="${1:-.env.production}"

if [[ ! -f "$env_file" ]]; then
    echo "$env_file is missing. Run deploy/scripts/prepare-production-env.sh first." >&2
    exit 1
fi

set -a
# shellcheck disable=SC1090
source "$env_file"
set +a

crate_version="$(sed -n 's/^version = "\(.*\)"/\1/p' crates/client/Cargo.toml | head -n 1)"
: "${CHEENHUB_APP_VERSION:=v${crate_version}-$(git rev-parse --short HEAD 2>/dev/null || printf local)}"
export CHEENHUB_APP_VERSION

dx build --release --platform web --package cheenhub_client --locked --debug-symbols false
