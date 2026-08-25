FROM ubuntu:24.04

RUN apt-get update \
    && apt-get install -y --no-install-recommends ca-certificates \
    && rm -rf /var/lib/apt/lists/*

COPY deploy/bin/cheenhub_backend /usr/local/bin/cheenhub_backend
COPY deploy/bin/cheenhub_metrics_proxy /usr/local/bin/cheenhub_metrics_proxy
COPY deploy/bin/cheenhub_migrations /usr/local/bin/cheenhub_migrations

EXPOSE 3000/tcp 3000/udp

CMD ["cheenhub_backend"]
