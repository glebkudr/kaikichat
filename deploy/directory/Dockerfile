# syntax=docker/dockerfile:1
# The discovery service (spec/discovery-v1.md), a release build. Built by
# Coolify on the server from the repository root, like the identity server.
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
    --mount=type=cache,id=chat-directory-target,target=/src/target \
    cargo build --locked --release -p agentic-directory \
    && install target/release/agentic-directory /usr/local/bin/agentic-directory

FROM debian:trixie-slim
RUN apt-get update \
    && apt-get install -y --no-install-recommends ca-certificates \
    && rm -rf /var/lib/apt/lists/*
COPY --from=build /usr/local/bin/agentic-directory /usr/local/bin/agentic-directory
COPY deploy/directory/start.sh /usr/local/bin/start.sh
EXPOSE 8081
ENTRYPOINT ["/usr/local/bin/start.sh"]
