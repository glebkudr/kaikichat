# syntax=docker/dockerfile:1
# Published bootstrap image; install the current stable compiler before building.
FROM rust:1.97.1-slim-trixie@sha256:8e8cf8f7fd54a2d23d5a743b3a03f56e26b6c774276c33fa0595111704ebb15c
RUN apt-get update \
    && apt-cache policy python3 iproute2 iptables conntrack libssl-dev pkg-config \
    && apt-get install -y --no-install-recommends python3 iproute2 iptables conntrack libssl-dev pkg-config \
    && rm -rf /var/lib/apt/lists/*
RUN rustup toolchain install 1.98.1 --profile minimal \
    && rustup default 1.98.1 \
    && rustc --version
WORKDIR /workspace
COPY Cargo.toml Cargo.lock ./
COPY crates ./crates
# The node library embeds the owner's skill (integrations/agent-skill/kaiki).
COPY integrations/agent-skill ./integrations/agent-skill
# Workspace members and the patched libp2p-kad, as deploy/node/Dockerfile.
COPY services ./services
COPY vendor ./vendor
COPY apps/desktop/src-tauri/Cargo.toml apps/desktop/src-tauri/Cargo.toml
COPY apps/desktop/src-tauri/src apps/desktop/src-tauri/src
COPY apps/desktop/src-tauri/build.rs apps/desktop/src-tauri/build.rs
RUN --mount=type=cache,id=ain-nat-registry,target=/usr/local/cargo/registry \
    --mount=type=cache,id=ain-nat-linux198-target,target=/workspace/target \
    cargo build --locked --release -p agentic-node --bins --example inspect_invitation --example test_state_failure \
    && install target/release/kaiki-agentic-node target/release/agentic-mcp /usr/local/bin/ \
    && install target/release/examples/inspect_invitation /usr/local/bin/ain-inspect-invitation \
    && install target/release/examples/test_state_failure /usr/local/bin/ain-test-state-failure
COPY tests/network/fixture.py /usr/local/lib/ain-network-fixture.py
ARG AIN_SOURCE_HASH
LABEL org.agentic.network-test.source-hash=$AIN_SOURCE_HASH
CMD ["sleep", "infinity"]
