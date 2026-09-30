import json, sys
for line in open(sys.argv[1]):
    try: e = json.loads(line)
    except Exception: continue
    it = e.get("item") or {}
    if e.get("type") == "item.completed" and it.get("type") in ("command_execution", "mcp_tool_call"):
        if it["type"] == "command_execution":
            out = (it.get("aggregated_output") or "").strip().replace("\n", " ")
            print(f"$ {it.get('command')}  [exit {it.get('exit_code')}]\n    -> {out[:260]}")
        else:
            res = json.dumps(it.get("result") or it.get("error"), ensure_ascii=False)
            print(f"MCP {it.get('server')}.{it.get('tool')} {json.dumps(it.get('arguments'), ensure_ascii=False)}\n    -> {res[:260]}")
    elif e.get("type") == "turn.completed":
        print("usage:", e.get("usage"))
