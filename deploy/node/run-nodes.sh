#!/bin/bash
# The testnet's mailbox holders, run in one container on the host's network.
# Node N listens on BASE_PORT+N-1 (QUIC over UDP, and TCP) with its data in
# /data/node-N; its secrets are made on its first start and stay there. Every
# node serves the DHT, AutoNAT and relays, so clients can use it to join.
# Node 1 starts first; the others bootstrap from it and from known routes.
# Each node's route and unit commitment (to bond in NodeRegistry) are written
# to /data/published.json. When one node stops, the container stops.
# The network's welcome agent (welcome.py) starts after them, with its own
# profile in /data/welcome; it ends with the container, never stops it.
set -euo pipefail
: "${NODES:=10}" "${BASE_PORT:=4101}" "${PUBLIC_IP:?}"
: "${CHAIN_RPC:?}" "${CHAIN_ID:?}" "${BOOK_SHOP:?}" "${GRANT_ISSUER:?}" "${REGISTRY:?}"
chain=(--chain-rpc "$CHAIN_RPC" --chain-id "$CHAIN_ID" --book-shop "$BOOK_SHOP"
    --grant-issuer "$GRANT_ISSUER" --registry "$REGISTRY"
    --chain-confirmations "${CHAIN_CONFIRMATIONS:-5}")
# The operator pool, where the holders draw and their operator withdraws.
[ -n "${OPERATOR_POOL:-}" ] && chain+=(--operator-pool "$OPERATOR_POOL")
# The identity server: holders report grants spent twice and read revocations.
[ -n "${IDENTITY_SERVER:-}" ] && chain+=(--identity-server "$IDENTITY_SERVER")
umask 077
# Published anew by this run; the health check waits for it.
rm -f /data/published.json
# The monitor (deploy/monitor) asks every node for node_info. Its container
# mounts only this directory: the nodes' sockets (hard links) and owner
# tokens, none of their keys. Links of an earlier run go: a node started
# below links its own again.
mkdir -p /data/monitor
rm -f /data/monitor/node-*
pids=()
trap 'kill -TERM "${pids[@]}" ${welcome:-} 2>/dev/null; wait' TERM INT

# Up to four routes of other nodes known from earlier runs, and `$1` if given.
routes_for() {
    local self=$1 first=${2:-} found=()
    [ -n "$first" ] && found+=("$first")
    for file in /data/node-*/route; do
        [ -s "$file" ] && [ "$file" != "/data/node-$self/route" ] || continue
        local route
        route=$(cat "$file")
        [ "$route" != "$first" ] && found+=("$route")
        [ ${#found[@]} -ge 4 ] && break
    done
    printf '%s\n' "${found[@]}"
}

# Nodes listen on the public address only: listening on 0.0.0.0 on the host's
# network made each node's record list eight of the host's docker bridges
# (10.0.x.1) and never its public address, so a client knew how to reach only
# the nodes the preset gave it and never stored at a quorum.
start() {
    local n=$1 first=${2:-}
    local dir=/data/node-$n port=$((BASE_PORT + n - 1))
    mkdir -p "$dir"
    rm -f "$dir/ipc.sock"
    [ -s "$dir/secrets.json" ] || python3 -c 'import json, secrets
print(json.dumps({"masterKey": secrets.token_hex(32), "ownerToken": secrets.token_hex(32)}))' >"$dir/secrets.json"
    local args=(serve --profile "$dir/profile.db" --ipc "$dir/ipc.sock" --secrets-stdin
        --listen "/ip4/$PUBLIC_IP/udp/$port/quic-v1" --listen "/ip4/$PUBLIC_IP/tcp/$port"
        --dht-server --autonat-server --relay-server "${chain[@]}")
    while read -r route; do
        [ -n "$route" ] && args+=(--bootstrap "$route")
    done < <(routes_for "$n" "$first")
    kaiki-agentic-node "${args[@]}" <"$dir/secrets.json" > >(sed -u "s/^/[node-$n] /") 2>&1 &
    pids+=($!)
    node-info.py "$dir" "$PUBLIC_IP" "$port" >"$dir/info.json"
    python3 -c 'import json, sys; print(json.load(open(sys.argv[1]))["route"])' \
        "$dir/info.json" >"$dir/route"
    ln -f "$dir/ipc.sock" "/data/monitor/node-$n.sock"
    python3 -c 'import json, sys; print(json.load(open(sys.argv[1]))["ownerToken"])' \
        "$dir/secrets.json" >"/data/monitor/node-$n.token"
}

start 1
first=$(cat /data/node-1/route)
# The discovery service checks what it is paid with at node 1. Its
# container mounts only this directory: node 1's socket (a hard link) and
# owner token, none of the nodes' keys.
mkdir -p /data/directory
ln -f /data/node-1/ipc.sock /data/directory/ipc.sock
python3 -c 'import json, sys; print(json.load(open(sys.argv[1]))["ownerToken"])' \
    /data/node-1/secrets.json >/data/directory/token
for n in $(seq 2 "$NODES"); do
    start "$n" "$first"
done
python3 - "$NODES" "$BASE_PORT" <<'EOF'
import json, sys
nodes, base = int(sys.argv[1]), int(sys.argv[2])
published = []
for n in range(1, nodes + 1):
    info = json.load(open(f"/data/node-{n}/info.json"))
    published.append({"node": n, "port": base + n - 1, **info})
json.dump(published, open("/data/published.json", "w"), indent=2)
print(json.dumps({"published": published}))
EOF
chmod 644 /data/published.json

welcome.py /data/welcome 2>&1 &
welcome=$!

# One node stopping stops the container; Coolify starts it again.
status=0
wait -n "${pids[@]}" || status=$?
kill -TERM "${pids[@]}" "$welcome" 2>/dev/null || true
wait || true
exit "$status"
