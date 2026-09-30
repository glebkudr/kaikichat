#!/bin/sh
# Starts the discovery service with its secrets, which Coolify passes in the
# environment, written to files it reads (in a tmpfs). What it is paid with
# is checked at testnet node 1, whose socket and owner token the nodes'
# container puts in the `directory` directory of their volume, mounted here
# read-only at /node. GitHub sign-in is offered once both its client id and
# secret are set.
set -eu
umask 077
mkdir -p /run/secrets
printf '%s\n' "${DIRECTORY_KEY:?}" >/run/secrets/directory-key
printf '%s\n' "${DIRECTORY_PEPPER:?}" >/run/secrets/directory-pepper
printf '%s\n' "${GOOGLE_CLIENT_SECRET:?}" >/run/secrets/google-client-secret
export AIN_DIR_SIGNING_KEY_FILE=/run/secrets/directory-key
export AIN_DIR_PEPPER_FILE=/run/secrets/directory-pepper
export AIN_DIR_GOOGLE_CLIENT_SECRET_FILE=/run/secrets/google-client-secret
export AIN_DIR_NODE_SOCKET=/node/ipc.sock
export AIN_DIR_NODE_TOKEN_FILE=/node/token
if [ -n "${GITHUB_CLIENT_ID:-}" ] && [ -n "${GITHUB_CLIENT_SECRET:-}" ]; then
    printf '%s\n' "$GITHUB_CLIENT_SECRET" >/run/secrets/github-client-secret
    export AIN_DIR_GITHUB_CLIENT_ID="$GITHUB_CLIENT_ID"
    export AIN_DIR_GITHUB_CLIENT_SECRET_FILE=/run/secrets/github-client-secret
fi
unset DIRECTORY_KEY DIRECTORY_PEPPER GOOGLE_CLIENT_SECRET GITHUB_CLIENT_SECRET
exec agentic-directory
