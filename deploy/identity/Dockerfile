# syntax=docker/dockerfile:1
# id.kaikichat.com: the identity server, a release build. Built by Coolify on
# the server from the repository root.
FROM rust:1.97.1-slim-trixie@sha256:8e8cf8f7fd54a2d23d5a743b3a03f56e26b6c774276c33fa0595111704ebb15c AS build
RUN apt-get update \
    && apt-get install -y --no-install-recommends pkg-config libssl-dev \
    && rm -rf /var/lib/apt/lists/*
WORKDIR /src
COPY Cargo.toml Cargo.lock ./
COPY crates ./crates
COPY services ./services
COPY vendor ./vendor
COPY apps/desktop/src-tauri/Cargo.toml apps/desktop/src-tauri/Cargo.toml
COPY apps/desktop/src-tauri/src apps/desktop/src-tauri/src
COPY apps/desktop/src-tauri/build.rs apps/desktop/src-tauri/build.rs
# Two jobs: the server runs other projects and must keep its memory.
ENV CARGO_BUILD_JOBS=2
RUN --mount=type=cache,id=chat-cargo-registry,target=/usr/local/cargo/registry,sharing=locked \
    --mount=type=cache,id=chat-identity-target,target=/src/target \
    cargo build --locked --release -p agentic-identity-server \
    && install target/release/agentic-identity-server /usr/local/bin/agentic-identity-server

FROM debian:trixie-slim
RUN apt-get update \
    && apt-get install -y --no-install-recommends ca-certificates \
    && rm -rf /var/lib/apt/lists/*
COPY --from=build /usr/local/bin/agentic-identity-server /usr/local/bin/agentic-identity-server
COPY deploy/identity/start.sh /usr/local/bin/start.sh
EXPOSE 8080
ENTRYPOINT ["/usr/local/bin/start.sh"]
