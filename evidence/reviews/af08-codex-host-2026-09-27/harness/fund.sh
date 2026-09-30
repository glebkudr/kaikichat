#!/bin/bash
# fund.sh PROFILE: buy one book for the profile on the local chain.
set -e
p=$1; CAST=/Users/glebk/Code/chat/.local/toolchains/foundry-v1.8.1-darwin-arm64/cast
N=/tmp/af08/network.json
chain=$(python3 -c "import json;print(json.load(open('$N'))['chain'])")
shop=$(python3 -c "import json;print(json.load(open('$N'))['shop'])")
payer=$(python3 -c "import json;print(json.load(open('$N'))['payer'])")
for i in $(seq 60); do
  out=$(/tmp/af08/ag $p coins buy || true)
  echo "$out" | grep -q '"result"' && break; sleep 1
done
key=$(echo "$out" | python3 -c 'import json,sys;print(json.load(sys.stdin)["result"]["key"])')
salt=$(echo "$out" | python3 -c 'import json,sys;print(json.load(sys.stdin)["result"]["salt"])')
value=$(echo "$out" | python3 -c 'import json,sys;print(json.load(sys.stdin)["result"]["value"])')
$CAST send "$shop" "buy(address,bytes32)" "$key" "$salt" --value "$value" --private-key "$payer" --rpc-url "$chain" >/dev/null
for i in $(seq 120); do
  b=$(/tmp/af08/ag $p coins balance || true)
  echo "$b" | grep -q '"remaining":1000' && { echo "$p funded: $b"; exit 0; }; sleep 1
done
echo "$p: purchase not noticed: $b"; exit 1
