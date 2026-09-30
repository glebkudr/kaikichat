#!/bin/bash
# V1-AF01 on a clean Linux x86_64: install the CLI from kaikichat.com, make a
# profile, join the public testnet by the signed preset, ask for a grant,
# restart, and check the JSON envelope and exit codes.
set -u
export DEBIAN_FRONTEND=noninteractive
apt-get update -qq >/dev/null && apt-get install -y -qq curl ca-certificates python3 >/dev/null
step() { echo "=== $1"; }
run() { # run a command, print its stdout JSON and exit code
  local out code
  out=$("$@" 2>/tmp/err); code=$?
  echo "exit=$code $out" | cut -c1-900
  [ -s /tmp/err ] && sed 's/^/  stderr: /' /tmp/err | head -3
  return 0
}
uname -srm; . /etc/os-release; echo "$PRETTY_NAME"; ldd --version | head -1
step "install"
t0=$(date +%s.%N)
curl -fsSL https://kaikichat.com/install.sh | sh
echo "installSeconds=$(echo "$(date +%s.%N) - $t0" | bc 2>/dev/null || python3 -c "print($(date +%s.%N)-$t0)")"
export PATH="$HOME/.local/bin:$PATH"
kaiki --version
kaiki skill show | head -5
export AGENTIC_SECRETS=file AGENTIC_PASSWORD='af01-linux-pass' AGENTIC_DATA_DIR=/root/kaiki-profile
step "status before init (no profile)"
run kaiki daemon status
step "init"
run kaiki init --name "AF01 Linux"
step "init again with the same name (safe to repeat)"
run kaiki init --name "AF01 Linux"
step "network (the signed preset)"
t0=$(date +%s)
run kaiki network
step "daemon status"
run kaiki daemon status
step "coins balance"
run kaiki coins balance
step "coins claim (the identity server's login link; claim_pending is exit 4, ask again)"
for i in $(seq 1 30); do
  out=$(kaiki coins claim 2>/dev/null); code=$?
  [ $code -ne 4 ] && break
  echo "  try $i: exit=$code $(echo "$out" | cut -c1-120)"
  sleep 2
done
echo "exit=$code $out" | cut -c1-900
url=$(echo "$out" | python3 -c 'import json,sys; r=json.load(sys.stdin).get("result",{}); print(r.get("url") or r.get("loginUrl") or "")' 2>/dev/null)
if [ -n "$url" ]; then
  echo "claim link host: $(echo "$url" | cut -d/ -f3)"
  loc=$(curl -s -o /dev/null -w '%{http_code} %{redirect_url}' "$url")
  echo "claim link answers: $(echo "$loc" | cut -c1-160)"
fi
step "coins buy (payment request)"
run kaiki coins buy
step "contacts request to a malformed id (invalid_input, exit 2)"
run kaiki contacts request --id ain1nope --name X --operation-id bad-1
step "send to an unknown contact (exit 3)"
echo hi | run kaiki send --to nobody --operation-id s-1 --text-stdin
step "wrong password (secrets_locked, exit 2)"
AGENTIC_PASSWORD=wrong run kaiki daemon stop
step "restart"
id1=$(kaiki daemon status 2>/dev/null | python3 -c 'import json,sys; r=json.load(sys.stdin)["result"]; print(r.get("networkId") or r.get("identity",{}).get("networkId",""))' 2>/dev/null)
run kaiki daemon stop
run kaiki daemon status
run kaiki daemon start
sleep 3
id2=$(kaiki daemon status 2>/dev/null | python3 -c 'import json,sys; r=json.load(sys.stdin)["result"]; print(r.get("networkId") or r.get("identity",{}).get("networkId",""))' 2>/dev/null)
echo "same identity after restart: $([ -n "$id1" ] && [ "$id1" = "$id2" ] && echo yes || echo "no ($id1 / $id2)")"
run kaiki daemon status
run kaiki daemon stop
