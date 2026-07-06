# CheenHub deployment

Краткая схема production-деплоя:

1. Сервер хранит состояние в Docker volumes: Postgres, Let's Encrypt и certbot webroot.
2. `deploy/compose.yml` запускает `db`, одноразовую миграцию, backend, nginx frontend и certbot.
3. Backend слушает HTTP API на внутреннем `3000` и WebTransport на `4443`; compose публикует WebTransport по UDP `443`.
4. Nginx принимает TCP `80/443`, отдаёт frontend и проксирует `/api/*` в backend.
5. Frontend собирается как отдельный Docker image, потому что URL API, URL realtime и публичный JWT-ключ зашиваются на этапе сборки.

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
- `GHCR_READ_TOKEN` - опционально, нужен для приватных GHCR packages.
- `GHCR_USERNAME` - опционально, по умолчанию используется actor workflow.

Workflow подключается по SSH, проверяет что `compose_project_dir` является чистым git checkout, делает `git fetch` и `git checkout --detach` на commit workflow, затем использует `deploy/compose.migrate.yml`: делает `docker compose pull migrate`, поднимает `db` и запускает одноразовый service `migrate` из указанного backend image. После успешной миграции workflow использует `deploy/compose.yml`, подтягивает `backend`, `web`, `certbot` и перезапускает `backend`, `web`, `certbot` через `up -d --no-deps`.

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
CHEENHUB_API_BASE_URL=https://cheenhub.ru/api
CHEENHUB_REALTIME_URL=https://cheenhub.ru/realtime
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
