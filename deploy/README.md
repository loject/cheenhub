# CheenHub deployment

Краткая схема production-деплоя:

1. Сервер хранит состояние в Docker volumes: Postgres, Let's Encrypt и certbot webroot.
2. `deploy/compose.yml` запускает `db`, одноразовую миграцию, backend, nginx frontend и certbot.
3. Backend слушает HTTP API по TCP и WebTransport по UDP на общем внутреннем порту `3000`; compose публикует WebTransport по UDP `443`.
4. Nginx принимает TCP `80/443`, отдаёт frontend и проксирует `/api/*` в backend.
5. Frontend собирается как отдельный Docker image, потому что базовый URL сервиса и публичный JWT-ключ зашиваются на этапе сборки.

## Отдельный UDP-порт WebTransport для нативного запуска

Опциональный `WEBTRANSPORT_PORT` задаёт UDP-порт WebTransport независимо от
`BACKEND_PORT` (HTTP TCP) и `BACKEND_HTTPS_PORT` (HTTPS TCP). При отсутствии
переменной WebTransport продолжает использовать `BACKEND_PORT`; текущий Docker
Compose и проброс `443/udp → 3000/udp` не меняются.

При нативном развёртывании можно использовать `WEBTRANSPORT_PORT=443` и
`BACKEND_HTTPS_PORT=443` одновременно: TCP и UDP являются разными протоколами.
Запуск на порту ниже 1024 требует `CAP_NET_BIND_SERVICE` через systemd или
другой контролируемый механизм, без запуска backend от root.
Пока внешний UDP 443 занят Docker, нативный WebTransport на нём запускать нельзя.

## Подготовительный автономный backend-образ

В `Dockerfile` есть дополнительная цель `backend-standalone-runtime`. Она
наследует бинарники из `backend-runtime` и копирует готовый `web-public` из
`web-builder` в `/app/web-public`, где Axum ищет файлы сайта. Сборка прервётся,
если отсутствуют `index.html`, `landing.html`, `sw.js` или manifest PWA.

Для её сборки нужны те же `CHEENHUB_*` build-args, которые используются для
`web-runtime` в `.github/workflows/release-images.yml`. Это отдельная цель:
обычные `backend-runtime`, `web-runtime`, CI-релизы и production Compose
остаются без изменений. Переключение TCP `80/443` здесь не выполняется.

## Нативный Linux-архив без Docker (подготовительный этап)

Workflow `.github/workflows/publish-native-linux.yml` запускается вручную из
GitHub Actions. Он собирает `cheenhub_backend`, `cheenhub_migrations` и
`web-public` на Ubuntu 24.04 без Docker, затем прикладывает архив
`cheenhub-native-linux-x86_64-<sha>.tar.gz` и его файл `.sha256`.
Архив содержит `bin/`, `web-public/`, `BUILD_INFO` и `SHA256SUMS`.

Перед запуском задай `CHEENHUB_JWT_PUBLIC_KEY_BASE64` в переменных
репозитория, как и для обычной frontend-сборки. Артефакт можно скачать
со страницы выполнения workflow; проверяй архив через `sha256sum -c`
и содержимое через `sha256sum -c SHA256SUMS` после распаковки.
Архив рассчитан на Linux x86_64 с совместимыми системными библиотеками;
секретов и `.env.production` в нём нет. Production-деплой пока не меняется.

## Ручная проверка нативного запуска на другом порту

Для замены текущего Docker-запуска используем существующий backend и web-клиент
с адресом и публичным JWT-ключом, встроенными при сборке. Runtime-конфигурации
клиента нет. `install.sh` на этом этапе не используется: он остаётся отдельным preview.

Перед сборкой подготовь корневой `.env`. Для локальной проверки:

```dotenv
BACKEND_HOST=127.0.0.1
BACKEND_PORT=13000
BACKEND_HTTPS_PORT=13443
WEBTRANSPORT_PORT=13443
CHEENHUB_BASE_URL=https://localhost:13443
CHEENHUB_CLIENT_BASE_URL=https://localhost:13443
```

Остальные настройки бери из действующего `.env`: `AUTH_STORE=postgres`,
`DATABASE_URL`, приватный JWT-ключ backend и соответствующие ему публичный ключ
и `kid` frontend. Для существующего production не создавай новую JWT-пару.
В `WEBTRANSPORT_TLS_CERT_PATH` и `WEBTRANSPORT_TLS_KEY_PATH` укажи абсолютные пути
к сертификату и ключу: локально — к dev-сертификату, на сервере — к действующему
сертификату домена. Для локального HTTPS браузер должен доверять dev-сертификату.

Сборка на локальной машине с установленными Rust/Dioxus:

```bash
cargo build --release --locked -p cheenhub_backend -p cheenhub_migrations
CHEENHUB_APP_VERSION="$(cargo run --quiet --locked -p xtask -- release-version print-tag)" cargo make build-web
bash deploy/scripts/package-native-linux.sh dist
```

Frontend build script читает корневой `.env`; backend читает `.env` при запуске.
Секреты и сертификаты не входят в архив. После распаковки помести `.env` рядом
с `bin/` и `web-public/`, затем из корня распакованного релиза выполни
`bin/cheenhub_backend`. Backend сам раздаёт web-клиент по HTTPS; HTTP на `13000`
перенаправляет все запросы на HTTPS с сохранением пути и метода.

Для проверки на сервере поменяй оба URL на `https://cheenhub.ru:13443`,
`BACKEND_HOST` на `0.0.0.0` и повтори сборку frontend. Используй доступный с хоста
адрес существующего production Postgres и открой TCP/UDP `13443`.
Проверяй `/health`, вход, WebTransport и голос; HTTP-редирект сам по себе
не подтверждает готовность. Текущие Nginx/Docker и их порты `80/443` до окончания
проверки не переключаются. Другой адрес или JWT-ключ требуют пересборки frontend.

Вместо локальной сборки можно запустить `Build native CheenHub Linux archive`
на `main` в GitHub Actions, указав `base_url=https://cheenhub.ru:13443`.
Публичный JWT-ключ и `kid` берутся из переменных репозитория и должны совпадать
с действующим backend. Скачай полученный архив и SHA-256, проверь контрольную
сумму и запусти распакованный backend с описанным выше серверным `.env`.
Workflow собирает архив; настройка БД, сертификатов и запуск на сервере выполняются отдельно.

## Установка и обновление одной командой (нативный preview)

После ручного запуска workflow `Build native CheenHub Linux archive` на `main`
архив публикуется в обновляемом prerelease `native-preview`. Установщик
скачивает архив и SHA-256 самостоятельно; Git, Docker и Rust ему не нужны.
Запускать от обычного пользователя Linux x86_64 с работающим `systemd --user`:

```bash
bash <(curl -fsSL https://raw.githubusercontent.com/loject/cheenhub/main/install.sh)
```

Выбери `1` для установки или обновления. Повторный запуск открывает то же
меню; для неинтерактивного обновления передай `update`. Сервис работает на
`http://127.0.0.1:3000`, журнал: `journalctl --user -u cheenhub -f`.
Конфигурация в `~/.config/cheenhub/backend.env` сохраняется при обновлениях;
в случае неудачного запуска устанавливается предыдущая версия.

На этом этапе это только локальный preview с `AUTH_STORE=inmemory`: учётные
записи не сохраняются после рестарта, а frontend собирается с указанным в
GitHub Actions публичным URL. Для полноценного self-hosting ещё потребуются
настройка динамического адреса frontend, PostgreSQL и HTTPS. Установка не
затрагивает запущенный Docker production.

## Первичная подготовка сервера

На сервере нужен Docker Compose v2 и git checkout репозитория, например `/opt/cheenhub`. Deploy workflow считает репозиторий единственным source of truth и перед запуском миграций синхронизирует server checkout с commit'ом workflow.

```bash
deploy/scripts/prepare-production-env.sh cheenhub.ru .env.production
```

Скрипт создаст `.env.production` с паролем Postgres и JWT-ключами. Проверь значения после генерации; секреты не коммитятся. Для образов из GitHub Container Registry укажи полные image references:

```dotenv
CHEENHUB_BACKEND_IMAGE_REF=ghcr.io/<owner>/<repo>/backend:v1.0.0
CHEENHUB_WEB_IMAGE_REF=ghcr.io/<owner>/<repo>/web:v1.0.0
```

Скопируй Firebase service account на сервер, не добавляя его в Git:

```bash
install -m 600 /path/to/firebase-service-account.json deploy/secrets/firebase-service-account.json
```

Compose монтирует этот файл только для чтения в backend-контейнер как `/run/secrets/fcm-service-account.json`.

## Метрики хоста

Compose запускает отдельный `metrics-proxy` из backend-образа. Только этот контейнер получает
Docker socket; backend обращается к нему по изолированной внутренней сети и получает уже
агрегированные CPU, RAM, network-счётчики сервисов `backend`, `web` и `db` и занятое место
на накопителе хоста.

Занятое место на диске измеряется через read-only bind mount `/var/lib/cheenhub-hostfs:/host-fs`.
Содержимое директории не важно — `statvfs` читает статистику файловой системы хоста, на которой
она лежит; если директория отсутствует, Docker создаст её автоматически. Если точка монтирования
недоступна, proxy отдаёт метрики без disk-сегмента, и дашборд показывает его как недоступный.

Proxy не предоставляет Docker Engine API, не принимает идентификаторы контейнеров от клиента и
имеет только два HTTP-маршрута: проверку готовности и чтение очищенного снимка метрик. Внешние
порты у сервиса отсутствуют. Не публикуй порт `9100` и не подключай к сети `host-metrics` другие
контейнеры.

## Отправка писем

Backend поддерживает SMTP и Gmail API по HTTPS. Транспорт, таймаут и учетные данные
хранятся в БД и меняются владельцем хоста в интерфейсе без перезапуска backend.
Если OAuth Client ID или Client Secret Gmail не записаны в БД, backend использует
`GOOGLE_OAUTH_CLIENT_ID` и `GOOGLE_OAUTH_CLIENT_SECRET` из окружения. Значения из БД
имеют приоритет и служат override.

Первый пользователь существующей установки автоматически становится владельцем хоста при
миграции. Если пользователей ещё нет, владельцем станет первый зарегистрировавшийся пользователь.
Для Gmail создай OAuth client типа `Web application`, включи Gmail API и укажи в
Google Cloud callback URI, показанный на странице настроек. После сохранения client ID и client
secret нажми «Подключить Gmail» и пройди обычную авторизацию Google.

Временно SMTP-пароль, OAuth client secret и Gmail refresh token хранятся в БД открытым текстом
по явно принятому решению. Ограничь доступ к БД и резервным копиям; API никогда не возвращает
эти значения. Для постоянной отправки OAuth consent screen должен быть переведён из Testing в
Production с учётом действующих ограничений Google.

Владение хостом не связано с владением сервером внутри CheenHub.

SSL выпускается через HTTP-01 challenge:

```bash
COMPOSE_FILES="-f deploy/compose.yml" deploy/scripts/init-letsencrypt.sh
```

## Деплой

Для registry-образов:

```bash
docker compose --env-file .env.production -f deploy/compose.yml pull
docker compose --env-file .env.production -f deploy/compose.yml up -d db
docker compose --env-file .env.production -f deploy/compose.yml up --force-recreate migrate
docker compose --env-file .env.production -f deploy/compose.yml up -d --no-deps backend web certbot
```

## Штатное завершение backend при обновлении

По `SIGTERM` backend прекращает приём новых подключений, дорабатывает активные HTTP-запросы и закрывает realtime-транспорты. Процесс ждёт отправки QUIC-закрытия и завершения realtime-обработчиков перед выходом.

- `GET /health` возвращает `200` во время работы и `503` при завершении. Healthcheck отмечает состояние контейнера; исключение его из маршрутизации требует поддержки со стороны внешнего балансировщика.
- Активные WebTransport- и WebSocket-сессии закрываются с причиной плановой остановки и кодом `1013`. Клиент выполняет штатное переподключение через существующий ping-watchdog и backoff.
- После восстановления соединения клиент повторно входит в серверную голосовую комнату, если пользователь не отменил вход и не выбрал другую цель. Личные звонки восстанавливаются модулем звонков по снимку активных звонков.

Compose ждёт `stop_grace_period: 30s`. После завершения HTTP ожидание realtime ограничено 25 секундами: по таймауту backend пишет предупреждение и завершает оставшиеся задачи. Долгие HTTP-запросы могут исчерпать окно Compose раньше этого срока.

Голос прерывается на время остановки и запуска контейнера. Длительность зависит от времени запуска backend, доступности зависимостей и задержки переподключения клиента; конкретный предел простоя не гарантируется.

## TLS renewal и WebTransport

Backend проверяет `fullchain.pem` и `privkey.pem` каждые `WEBTRANSPORT_TLS_RELOAD_INTERVAL_SECONDS` секунд (по умолчанию 5). Одинаковый валидный candidate применяется после двух проверок только для новых WebTransport соединений; UDP listener и действующие сессии сохраняются.

`deploy/scripts/renew-letsencrypt.sh` выполняет certbot renewal и reload nginx. Backend сам применяет изменения cert/key для новых WebTransport соединений без restart.

Полная ссылка на Docker image называется **image reference**. Для GHCR она выглядит так:

```text
ghcr.io/<owner>/<repo>/backend:<tag>
ghcr.io/<owner>/<repo>/web:<tag>
```

Например:

```dotenv
CHEENHUB_BACKEND_IMAGE_REF=ghcr.io/loject/cheenhub/backend:v1.0.0
CHEENHUB_WEB_IMAGE_REF=ghcr.io/loject/cheenhub/web:v1.0.0
```

## Ручной деплой через GitHub Actions

Workflow `.github/workflows/run-migrations.yml` запускается вручную через `workflow_dispatch`, сначала применяет миграции, затем обновляет backend и frontend containers.

Inputs:

- `backend_image_ref` - опциональный полный image reference backend-образа, например `ghcr.io/<owner>/<repo>/backend:v1.0.0`; если оставить пустым, используется `ghcr.io/<owner>/<repo>/backend:latest`.
- `web_image_ref` - опциональный полный image reference frontend-образа, например `ghcr.io/<owner>/<repo>/web:v1.0.0`; если оставить пустым, используется `ghcr.io/<owner>/<repo>/web:latest`.
- `compose_project_dir` - путь к checkout проекта на Ubuntu-сервере, по умолчанию `/opt/cheenhub`.
- `env_file` - production env-файл относительно `compose_project_dir`, по умолчанию `.env.production`.

Secrets:

- `DEPLOY_SSH_HOST` - адрес Ubuntu-сервера.
- `DEPLOY_SSH_USER` - пользователь для SSH.
- `DEPLOY_SSH_PRIVATE_KEY` - приватный SSH-ключ для деплоя.
- `DEPLOY_SSH_PORT` - опционально, по умолчанию `22`.
- `FCM_SERVICE_ACCOUNT_JSON` - полный JSON service account Firebase; workflow передаёт его на сервер через stdin и сохраняет с правами `600` только в gitignored `deploy/secrets`.
- `GHCR_READ_TOKEN` - опционально, нужен для приватных GHCR packages.
- `GHCR_USERNAME` - опционально, по умолчанию используется actor workflow.

Workflow подключается по SSH, проверяет что `compose_project_dir` является чистым git checkout, делает `git fetch` и `git checkout --detach` на commit workflow, затем использует `deploy/compose.migrate.yml`: делает `docker compose pull migrate`, поднимает `db` и запускает одноразовый service `migrate` из указанного backend image. После успешной миграции workflow использует `deploy/compose.yml`, подтягивает и перезапускает `backend`, `web` и `certbot`.

Для локальной сборки на сервере можно добавить `deploy/compose.build.yml`, а для ручного frontend-артефакта оставить текущий overlay `deploy/compose.artifact.yml`.

## GitHub Actions release

Workflow `.github/workflows/release-images.yml`:

- собирает `backend-runtime` и `web-runtime` из корневого `Dockerfile`;
- публикует образы в GHCR:
  - `ghcr.io/<owner>/<repo>/backend:<tag>`;
  - `ghcr.io/<owner>/<repo>/web:<tag>`;
- на git-теге `v*` создаёт GitHub Release и прикрепляет `docker save` архивы этих образов.

Для production frontend image задай repository variables:

```text
CHEENHUB_DOMAIN=cheenhub.ru
CHEENHUB_BASE_URL=https://cheenhub.ru
CHEENHUB_JWT_KEY_ID=prod-ed25519-1
CHEENHUB_JWT_PUBLIC_KEY_BASE64=<public key from .env.production>
CHEENHUB_REALTIME_CERT_SHA256=
```

Релиз создаётся пушем тега:

```bash
git tag v1.0.0
git push origin v1.0.0
```

После релиза обнови `CHEENHUB_BACKEND_IMAGE_REF` и `CHEENHUB_WEB_IMAGE_REF` на сервере и перезапусти compose. Если GHCR package приватный, сначала выполни на сервере `docker login ghcr.io`.
