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

release_tag="$(cargo run --quiet -p xtask -- release-version print-tag)"
: "${CHEENHUB_APP_VERSION:=${release_tag}-$(git rev-parse --short HEAD 2>/dev/null || printf local)}"
export CHEENHUB_APP_VERSION

dx build --release --platform web --package cheenhub_client --bin cheen_hub --locked --debug-symbols false
