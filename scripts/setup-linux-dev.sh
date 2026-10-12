#!/bin/sh
set -eu

if [ "$(uname -s)" != Linux ]; then
    echo "Эта команда предназначена для Linux." >&2
    exit 1
fi

project_root=$(CDPATH= cd -- "$(dirname -- "$0")/.." && pwd)
applications_dir="${XDG_DATA_HOME:-$HOME/.local/share}/applications"
install -d "$applications_dir"
escaped_launcher=$(printf '%s' "$project_root/scripts/launch-linux-dev.sh" | sed 's/\\/\\\\\\\\/g; s/"/\\\\"/g; s/`/\\\\`/g; s/\$/\\\\$/g')
sed '/^Exec=/d' "$project_root/bundle/linux/ru.cheenhub.Development.desktop" \
    > "$applications_dir/ru.cheenhub.Development.desktop"
printf 'Exec=/bin/sh "%s"\n' "$escaped_launcher" \
    >> "$applications_dir/ru.cheenhub.Development.desktop"
chmod 644 "$applications_dir/ru.cheenhub.Development.desktop"
echo "Интеграция CheenHub для разработки настроена: $applications_dir/ru.cheenhub.Development.desktop"
echo "Запускайте cargo make dev-stack-desktop. Установка пакета не требуется."
