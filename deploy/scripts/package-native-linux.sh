#!/usr/bin/env bash
set -euo pipefail

# Собирает переносимый архив для нативного Linux-деплоя из готовых результатов Cargo/Dioxus.
# Ни Docker, ни production-конфигурация, ни секреты в архив не попадают.

if [[ $# -ne 1 ]]; then
    echo 'Usage: bash deploy/scripts/package-native-linux.sh OUTPUT_DIR' >&2
    exit 2
fi

if [[ "$(uname -s)" != Linux || "$(uname -m)" != x86_64 ]]; then
    echo 'Native Linux archive currently supports only linux-x86_64.' >&2
    exit 1
fi

commit="$(git rev-parse --verify HEAD)"
name="cheenhub-native-linux-x86_64-${commit:0:12}"
output_dir="$1"
mkdir -p "$output_dir"
output_dir="$(cd "$output_dir" && pwd)"

for file in \
    target/release/cheenhub_backend \
    target/release/cheenhub_migrations \
    target/dx/cheenhub/release/web/public/index.html \
    target/dx/cheenhub/release/web/public/landing.html \
    target/dx/cheenhub/release/web/public/sw.js \
    target/dx/cheenhub/release/web/public/manifest.webmanifest; do
    if [[ ! -s "$file" ]]; then
        echo "Missing or empty build output: $file" >&2
        exit 1
    fi
done

workdir="$(mktemp -d)"
trap 'rm -rf -- "$workdir"' EXIT
package_dir="$workdir/$name"
mkdir -p "$package_dir/bin" "$package_dir/web-public"
install -m 755 target/release/cheenhub_backend "$package_dir/bin/cheenhub_backend"
install -m 755 target/release/cheenhub_migrations "$package_dir/bin/cheenhub_migrations"
cp -a target/dx/cheenhub/release/web/public/. "$package_dir/web-public/"
printf 'commit=%s\nplatform=linux-x86_64\n' "$commit" > "$package_dir/BUILD_INFO"
printf 'base_url=%s\nrelease_tag=%s\n' "${CHEENHUB_BASE_URL:-}" \
    "${CHEENHUB_RELEASE_TAG:-}" >> "$package_dir/BUILD_INFO"

(
    cd "$package_dir"
    find bin web-public BUILD_INFO -type f -print0 \
        | LC_ALL=C sort -z \
        | xargs -0 sha256sum > SHA256SUMS
    sha256sum --check --status SHA256SUMS
)

archive="$name.tar.gz"
tar -C "$workdir" -czf "$output_dir/$archive" "$name"
(
    cd "$output_dir"
    sha256sum "$archive" > "$archive.sha256"
    sha256sum --check "$archive.sha256"
)
echo "Native archive: $output_dir/$archive"
echo "SHA-256:        $output_dir/$archive.sha256"
