#!/bin/sh
# Starts the identity server with its secrets, which Coolify passes in the
# environment, written to files it reads (in a tmpfs). GitHub sign-in is
# offered once both its client id and secret are set.
set -eu
umask 077
mkdir -p /run/secrets
printf '%s\n' "${ISSUER_KEY:?}" >/run/secrets/issuer-key
printf '%s\n' "${GOOGLE_CLIENT_SECRET:?}" >/run/secrets/google-client-secret
export AIN_ID_SIGNING_KEY_FILE=/run/secrets/issuer-key
export AIN_ID_GOOGLE_CLIENT_SECRET_FILE=/run/secrets/google-client-secret
if [ -n "${GITHUB_CLIENT_ID:-}" ] && [ -n "${GITHUB_CLIENT_SECRET:-}" ]; then
    printf '%s\n' "$GITHUB_CLIENT_SECRET" >/run/secrets/github-client-secret
    export AIN_ID_GITHUB_CLIENT_ID="$GITHUB_CLIENT_ID"
    export AIN_ID_GITHUB_CLIENT_SECRET_FILE=/run/secrets/github-client-secret
fi
unset ISSUER_KEY GOOGLE_CLIENT_SECRET GITHUB_CLIENT_SECRET
exec agentic-identity-server
