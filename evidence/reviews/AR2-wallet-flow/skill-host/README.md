# Packaged CLI skill — independent agent host

An independent Codex agent read the actual packaged `SKILL.md` and received only
an owner-issued CLI command, an ephemeral credentials path and an allowed public
NetworkID. It used CLI subprocesses for context, send, exact retry, delivery,
poll and ACK. It did not inspect credentials, daemon state or implementation.

The [host result](host-result.json), [independent review](review.json) and
[separate peer observer](observer-result.json) all pass. One original send and
one 4854-byte reply remain intact. A 4096-byte poll returns
`inbox_item_too_large`; the agent uses a new operation ID and the required 4854
bytes within its 8192-byte grant, saves the complete reply before ACK and then
obtains an empty page with no lease. Exact retry keeps the same MessageID.

The source and packaged skill SHA-256 is
`8cb97a5c5219dbb787cf2c9a4e52c248bad6d4b4f3edfee4ab2cb398351d2bbf`.
The current [native UI gate](../status-skill-native-ui.json) separately proves
that the real permissions panel exposes that exact text and executable command,
and checks shared CLI/MCP delivery, revocation and cold history.

`host.py` is the host-created driver; `skill-host-smoke.py` is the independent
test peer fixture. They retain their original local run paths as historical
evidence. JSON artifact paths were made relative; `manifest.json` records both
original and retained hashes. CLI stdout, exact reply and review are retained.
Private connection context, credentials, profiles and runtime logs are excluded.
Both isolated daemons stopped and their temporary profiles were removed.

This proves a real local host with an existing granted contact on macOS. It does
not prove new-contact onboarding, host restart, malicious-content handling, NAT,
Windows/Linux or complete E11. It is not a paid exchange; the separate paid CLI
and lifecycle gates establish that range.
