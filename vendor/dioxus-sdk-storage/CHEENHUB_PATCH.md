# Совместимость с Dioxus 0.8

Исходники: опубликованный crate `dioxus-sdk-storage 0.7.0`, репозиторий
https://github.com/DioxusLabs/sdk/tree/694399608b0bc53e3f3e2c58e75669d97a63cc76/packages/storage.
Commit и путь подтверждены исходным `.cargo_vcs_info.json`.
Лицензии MIT / Apache-2.0 сохранены из корня этого commit.

Изменён только Cargo.toml: зависимости `dioxus` и `dioxus-signals`
закреплены на `=0.8.0-alpha.1`; default features Dioxus выключены,
а необходимые `signals`, `hooks`, `macro`, `logger`, `launch` включены явно.
Rust-исходники, API и формат хранения оставлены без изменений.
Опубликованная версия SDK 0.7 иначе использует несовместимый тип Signal 0.7.

Патч временный. После выхода SDK с поддержкой выбранной версии Dioxus
удалить эту директорию, запись `[patch.crates-io]` и исключение workspace,
обновить lockfile и проверить чтение ранее сохранённых настроек.
