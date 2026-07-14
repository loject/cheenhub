# syntax=docker/dockerfile:1.7

FROM rust:1.92-bookworm AS source
WORKDIR /app

FROM source AS backend-builder
COPY .cargo ./.cargo
COPY Cargo.toml Cargo.lock Dioxus.toml ./
COPY build_support ./build_support
COPY xtask ./xtask
COPY crates ./crates
RUN --mount=type=cache,id=cheenhub-cargo-registry,target=/usr/local/cargo/registry,sharing=locked \
    --mount=type=cache,id=cheenhub-cargo-git,target=/usr/local/cargo/git,sharing=locked \
    --mount=type=cache,id=cheenhub-backend-target,target=/app/target,sharing=locked \
    cargo build --release --locked -p cheenhub_backend -p cheenhub_migrations \
    && cp /app/target/release/cheenhub_backend /usr/local/bin/cheenhub_backend \
    && cp /app/target/release/cheenhub_migrations /usr/local/bin/cheenhub_migrations

FROM source AS web-tools
RUN --mount=type=cache,id=cheenhub-cargo-registry,target=/usr/local/cargo/registry,sharing=locked \
    --mount=type=cache,id=cheenhub-cargo-git,target=/usr/local/cargo/git,sharing=locked \
    --mount=type=cache,id=cheenhub-cargo-make-target,target=/tmp/cargo-make-target,sharing=locked \
    --mount=type=cache,id=cheenhub-wasm-bindgen-cli-target,target=/tmp/wasm-bindgen-cli-target,sharing=locked \
    --mount=type=cache,id=cheenhub-dioxus-cli-target,target=/tmp/dioxus-cli-target,sharing=locked \
    rustup target add wasm32-unknown-unknown \
    && CARGO_TARGET_DIR=/tmp/cargo-make-target cargo install cargo-make --version 0.37.24 --locked \
    && CARGO_TARGET_DIR=/tmp/wasm-bindgen-cli-target cargo install wasm-bindgen-cli --version 0.2.120 --locked \
    && CARGO_TARGET_DIR=/tmp/dioxus-cli-target cargo install dioxus-cli --version 0.7.5 --locked

FROM web-tools AS web-builder
COPY .cargo ./.cargo
COPY Cargo.toml Cargo.lock Dioxus.toml Makefile.toml ./
COPY build_support ./build_support
COPY xtask ./xtask
COPY crates ./crates
ARG CHEENHUB_API_BASE_URL
ARG CHEENHUB_APP_VERSION
ARG CHEENHUB_JWT_KEY_ID
ARG CHEENHUB_JWT_PUBLIC_KEY_BASE64
ARG CHEENHUB_REALTIME_URL
ARG CHEENHUB_REALTIME_CERT_SHA256
ENV CHEENHUB_API_BASE_URL=${CHEENHUB_API_BASE_URL}
ENV CHEENHUB_APP_VERSION=${CHEENHUB_APP_VERSION}
ENV CHEENHUB_JWT_KEY_ID=${CHEENHUB_JWT_KEY_ID}
ENV CHEENHUB_JWT_PUBLIC_KEY_BASE64=${CHEENHUB_JWT_PUBLIC_KEY_BASE64}
ENV CHEENHUB_REALTIME_URL=${CHEENHUB_REALTIME_URL}
ENV CHEENHUB_REALTIME_CERT_SHA256=${CHEENHUB_REALTIME_CERT_SHA256}
RUN --mount=type=cache,id=cheenhub-cargo-registry,target=/usr/local/cargo/registry,sharing=locked \
    --mount=type=cache,id=cheenhub-cargo-git,target=/usr/local/cargo/git,sharing=locked \
    --mount=type=cache,id=cheenhub-web-target,target=/app/target,sharing=locked \
    cargo make build-microphone-worker-wasm-release \
    && dx build --release --platform web --package cheenhub_client --bin cheen_hub --locked --debug-symbols false \
    && mkdir -p /app/web-public \
    && cp -a /app/target/dx/cheen_hub/release/web/public/. /app/web-public/

FROM debian:bookworm-slim AS backend-runtime
RUN apt-get update \
    && apt-get install -y --no-install-recommends ca-certificates \
    && rm -rf /var/lib/apt/lists/*
WORKDIR /app
COPY --from=backend-builder /usr/local/bin/cheenhub_backend /usr/local/bin/cheenhub_backend
COPY --from=backend-builder /usr/local/bin/cheenhub_migrations /usr/local/bin/cheenhub_migrations
EXPOSE 3000 4443/tcp 4443/udp
CMD ["cheenhub_backend"]

FROM nginx:1.27-alpine AS web-runtime
COPY deploy/nginx/default.conf /etc/nginx/conf.d/default.conf
COPY --from=web-builder /app/web-public /usr/share/nginx/html
EXPOSE 80
