# Deploying the discovery service

Not in `deploy/docker-compose.yml` yet: it needs, besides the release on
`main`,

- a domain (for example `directory.kaikichat.com`) routed to port 8081;
- Google OAuth: the redirect URI `https://<domain>/v1/oauth/google/callback`
  added to the identity server's client;
- GitHub OAuth: an app whose callback is
  `https://<domain>/v1/oauth/github/callback` (an OAuth app has one
  callback URL, so not the identity server's);
- two new secrets: `DIRECTORY_KEY` and `DIRECTORY_PEPPER`, 32 bytes of hex
  each; keep the pepper with the database for good;
- the nodes' volume: the nodes' container puts node 1's socket and owner
  token in its `directory` directory (`deploy/node/run-nodes.sh`); only it
  is mounted, read-only (a volume `subpath` needs Docker Engine 26 and
  Compose 2.26 or later; the server has 29 and 5). The token is node 1's full owner token: a service
  broken into can run node 1, not the other nodes.

The compose service (add it under `services:` of `deploy/docker-compose.yml`
and `chat-directory:` under `volumes:`); in Coolify its domain is
`https://directory.kaikichat.com:8081`, like the identity server's:

```yaml
  directory:
    build:
      context: .
      dockerfile: deploy/directory/Dockerfile
    expose:
      - "8081"
    restart: unless-stopped
    stop_signal: SIGINT
    environment:
      AIN_DIR_LISTEN: "0.0.0.0:8081"
      AIN_DIR_PUBLIC_URL: "https://directory.kaikichat.com"
      AIN_DIR_DOMAIN: "0xae2e3182ade817a3e726c29ef308eed15c6ec267ffc01a68da990e576fa0df18"
      AIN_DIR_DATABASE: "/data/directory.db"
      # The identity server's Google client, which lists this service's
      # callback too; its secret is the same Coolify variable.
      AIN_DIR_GOOGLE_CLIENT_ID: "630672545352-vpuegdiua39ke8dpu2mldv63lf258c68.apps.googleusercontent.com"
      GOOGLE_CLIENT_SECRET: "${GOOGLE_CLIENT_SECRET}"
      # The directory's own GitHub OAuth app.
      GITHUB_CLIENT_ID: "<its client id>"
      GITHUB_CLIENT_SECRET: "${DIRECTORY_GITHUB_CLIENT_SECRET}"
      DIRECTORY_KEY: "${DIRECTORY_KEY}"
      DIRECTORY_PEPPER: "${DIRECTORY_PEPPER}"
    volumes:
      - chat-directory:/data
      - type: volume
        source: chat-nodes
        target: /node
        read_only: true
        volume:
          subpath: directory
    tmpfs:
      - /run/secrets
    mem_limit: 256m
    depends_on:
      nodes:
        condition: service_healthy
    logging:
      driver: json-file
      options:
        max-size: "10m"
        max-file: "3"
```

The service reaches node 1 over its socket in the shared volume, so it
needs no network of the nodes. The server's Docker (29) and Compose (5) take
a volume `subpath`.

Behind Coolify's proxy every client comes from the proxy's address, so the
service limits searches by book, not by address.

Clients reach it with `kaiki daemon start --directory https://directory.kaikichat.com`.
