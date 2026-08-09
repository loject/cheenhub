# CheenHub

## Локальный запуск

Проект использует cargo-make для dev-задач и cargo-watch/Dioxus CLI для
локального dev stack.

Нужен Rust, установленный через rustup. Пользовательские Cargo-инструменты
устанавливаются в `~/.local/bin`; этот каталог должен находиться в `PATH`.

```bash
export PATH="$HOME/.local/bin:$PATH"
cargo install --locked --root "$HOME/.local" --version 0.37.24 cargo-make
cargo make setup
cargo make dev-stack
```

## Легенда комнат

\# - текстовая комната
& - текстовая и голосовая комната
~ - голосовая комната

## Архитектура связи

Медиа строится через SFU: клиент отправляет голос, видео и демонстрацию экрана
на сервер по WebTransport, а сервер пересылает нужные потоки участникам комнаты.
