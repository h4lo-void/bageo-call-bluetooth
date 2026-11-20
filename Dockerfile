# SPDX-License-Identifier: AGPL-3.0-only OR LicenseRef-Bageo-Commercial
# SPDX-FileCopyrightText: h4lo-void <https://github.com/h4lo-void/bageo-call>

# Bageo agent in a container (optional; the start scripts run it
# without Docker). Build from the repository root:
#
#   docker compose up -d --build
#
# The Bluetooth computer connects to this container by IP address: in the web
# console open System -> "Pair a phone by IP address" -> "Get a pairing code",
# then on the Bluetooth computer run "bageo-bluetooth link <this host's IP>
# <code>" (see bluetooth/README.md). It uses the console's port (8787, or the
# host port you map to it), so no other port is needed.
FROM rust:bookworm AS builder
WORKDIR /src
COPY Cargo.toml Cargo.lock ./
COPY crates ./crates
# A low default keeps small hosts out of memory trouble; raise it with
#   docker compose build --build-arg CARGO_BUILD_JOBS=4
ARG CARGO_BUILD_JOBS=2
ENV CARGO_BUILD_JOBS=${CARGO_BUILD_JOBS}
RUN --mount=type=cache,target=/usr/local/cargo/registry \
    --mount=type=cache,target=/usr/local/cargo/git \
    --mount=type=cache,target=/src/target \
    cargo build --release -p agent \
    && cp /src/target/release/agent /usr/local/bin/agent

FROM debian:bookworm-slim
RUN apt-get update \
    && apt-get install -y --no-install-recommends ca-certificates curl tzdata \
    && rm -rf /var/lib/apt/lists/*
WORKDIR /app
COPY --from=builder /usr/local/bin/agent /usr/local/bin/agent
COPY assets /app/assets

ENV AGENT_HOST=0.0.0.0 \
    AGENT_PORT=8787 \
    ASSETS_DIR=/app/assets \
    CALLS_DATA_DIR=/data/calls
VOLUME ["/data/calls"]
EXPOSE 8787

HEALTHCHECK --interval=30s --timeout=5s --start-period=20s --retries=3 \
    CMD curl -fsS -o /dev/null http://127.0.0.1:8787/health || exit 1

CMD ["agent"]
