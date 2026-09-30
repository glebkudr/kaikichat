# Linux x86_64 build and GUI check image for the desktop app (V1-GF02).
# On Apple silicon it runs under Rosetta: docker build --platform linux/amd64.
FROM --platform=linux/amd64 ubuntu:24.04
ENV DEBIAN_FRONTEND=noninteractive
RUN apt-get update && apt-get install -y --no-install-recommends \
      build-essential ca-certificates curl git pkg-config python3 xz-utils file \
      libwebkit2gtk-4.1-dev libgtk-3-dev libayatana-appindicator3-dev librsvg2-dev \
      libssl-dev libdbus-1-dev libxdo-dev patchelf \
      xvfb xauth x11-apps imagemagick dbus dbus-x11 gnome-keyring libsecret-tools procps \
      fonts-dejavu-core fonts-noto-core \
    && rm -rf /var/lib/apt/lists/*
ARG RUST=1.97.1
RUN curl -sSf https://sh.rustup.rs | sh -s -- -y --profile minimal --default-toolchain ${RUST}
ARG NODE=26.8.1
RUN curl -fsSL https://nodejs.org/dist/v${NODE}/node-v${NODE}-linux-x64.tar.xz | tar -xJ -C /opt \
    && ln -s /opt/node-v${NODE}-linux-x64/bin/node /usr/local/bin/node \
    && ln -s /opt/node-v${NODE}-linux-x64/bin/npm /usr/local/bin/npm
ENV PATH=/root/.cargo/bin:$PATH
WORKDIR /workspace
