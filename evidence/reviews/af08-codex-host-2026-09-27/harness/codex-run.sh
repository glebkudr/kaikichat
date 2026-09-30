#!/bin/bash
# codex-run.sh NAME WORKDIR SANDBOX [extra codex args...] < prompt
name=$1 dir=$2 sandbox=$3; shift 3
PATH="/tmp/af08/bin:$PATH" exec codex exec -m gpt-6-sol -c 'model_reasoning_effort="xhigh"' \
  -c 'approval_policy="never"' -c 'sandbox_workspace_write.network_access=true' \
  -c 'mcp_servers.playwright.enabled=false' -c 'mcp_servers.adloop.enabled=false' -c 'mcp_servers.node_repl.enabled=false' \
  -C "$dir" -s "$sandbox" --json -o "/tmp/af08/runs/$name.final.md" "$@" - \
  > "/tmp/af08/runs/$name.jsonl" 2> "/tmp/af08/runs/$name.stderr"
