# syntax=docker/dockerfile:1
# The testnet's nodes: agentic-node, a release build, and the script that runs
# the holders; and the network's welcome agent (welcome.py), which drives
# `kaiki`. Built by Coolify on the server from the repository root.
FROM rust:1.97.1-slim-trixie@sha256:8e8cf8f7fd54a2d23d5a743b3a03f56e26b6c774276c33fa0595111704ebb15c AS build
RUN apt-get update \
    && apt-get install -y --no-install-recommends pkg-config libssl-dev \
    && rm -rf /var/lib/apt/lists/*
WORKDIR /src
COPY Cargo.toml Cargo.lock ./
COPY crates ./crates
# The node library embeds the owner's skill (integrations/agent-skill/kaiki).
COPY integrations/agent-skill ./integrations/agent-skill
COPY services ./services
COPY vendor ./vendor
COPY apps/desktop/src-tauri/Cargo.toml apps/desktop/src-tauri/Cargo.toml
COPY apps/desktop/src-tauri/src apps/desktop/src-tauri/src
COPY apps/desktop/src-tauri/build.rs apps/desktop/src-tauri/build.rs
# Two jobs: the server runs other projects and must keep its memory.
ENV CARGO_BUILD_JOBS=2
RUN --mount=type=cache,id=chat-cargo-registry,target=/usr/local/cargo/registry,sharing=locked \
    --mount=type=cache,id=chat-node-target,target=/src/target \
    cargo build --locked --release -p agentic-node --bin agentic-node --bin kaiki \
    && install target/release/agentic-node target/release/kaiki /usr/local/bin/

FROM debian:trixie-slim
RUN apt-get update \
    && apt-get install -y --no-install-recommends ca-certificates python3 \
    && rm -rf /var/lib/apt/lists/*
COPY --from=build /usr/local/bin/agentic-node /usr/local/bin/kaiki /usr/local/bin/
COPY deploy/node/run-nodes.sh deploy/node/node-info.py deploy/node/welcome.py deploy/node/welcome-kaiki /usr/local/bin/
ENTRYPOINT ["/usr/local/bin/run-nodes.sh"]
