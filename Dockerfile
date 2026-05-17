FROM rust:1.92-bookworm AS source
WORKDIR /app
COPY Cargo.toml Cargo.lock Dioxus.toml ./
COPY build_support ./build_support
COPY crates ./crates

FROM source AS backend-builder
RUN cargo build --release --locked -p cheenhub_backend -p cheenhub_migrations

FROM source AS web-builder
RUN rustup target add wasm32-unknown-unknown \
    && cargo install dioxus-cli --version 0.7.5 --locked
ARG CHEENHUB_API_BASE_URL
ARG CHEENHUB_JWT_KEY_ID
ARG CHEENHUB_JWT_PUBLIC_KEY_BASE64
ARG CHEENHUB_REALTIME_URL
ARG CHEENHUB_REALTIME_CERT_SHA256
ENV CHEENHUB_API_BASE_URL=${CHEENHUB_API_BASE_URL}
ENV CHEENHUB_JWT_KEY_ID=${CHEENHUB_JWT_KEY_ID}
ENV CHEENHUB_JWT_PUBLIC_KEY_BASE64=${CHEENHUB_JWT_PUBLIC_KEY_BASE64}
ENV CHEENHUB_REALTIME_URL=${CHEENHUB_REALTIME_URL}
ENV CHEENHUB_REALTIME_CERT_SHA256=${CHEENHUB_REALTIME_CERT_SHA256}
RUN dx build --release --platform web --package cheenhub_client --locked --debug-symbols false

FROM debian:bookworm-slim AS backend-runtime
RUN apt-get update \
    && apt-get install -y --no-install-recommends ca-certificates \
    && rm -rf /var/lib/apt/lists/*
WORKDIR /app
COPY --from=backend-builder /app/target/release/cheenhub_backend /usr/local/bin/cheenhub_backend
COPY --from=backend-builder /app/target/release/cheenhub_migrations /usr/local/bin/cheenhub_migrations
EXPOSE 3000 4443/tcp 4443/udp
CMD ["cheenhub_backend"]

FROM nginx:1.27-alpine AS web-runtime
COPY deploy/nginx/default.conf /etc/nginx/conf.d/default.conf
COPY --from=web-builder /app/target/dx/cheenhub_client/release/web/public /usr/share/nginx/html
EXPOSE 80
