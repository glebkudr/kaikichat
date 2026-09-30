# Unattended native tests without login-Keychain prompts

The user reported repeated password dialogs for
`net.agenticinternet.desktop.e2e`. The paid GUI run was stopped; no test process
remained. Earlier helper/ACL approaches did not reliably prevent dialogs across
desktop restarts, so E2E automation no longer uses the login Keychain at all.

Only debug builds with the explicit `e2e` feature use `E2eFileStore`, storing
exactly 64 random test bytes in a private `0600` `.e2e-secrets` file inside the
isolated `0700` profile. Reads are bound to that profile's canonical account.
Missing, exposed, truncated or substituted files fail closed. Existing files
cannot be overwritten. The ordinary host still persists before daemon creation
and refuses to generate a replacement key for an existing database.

Normal builds continue using `net.agenticinternet.desktop` in system Keychain.
No production secret, login password, Keychain ACL or global security preference
was changed. Release compilation rejects the E2E feature. Native test windows
and daemons stop before their temporary profiles and keys are removed.

Independent backend-test-critic accepted the real restart/identity, lost/exposed/
truncated key and exact raw seed tests before implementation. The initial RED
was the absent `E2eFileStore` API. A setup correction reused the existing private
directory helper; it did not weaken the shared-directory refusal. Accepted final
test SHA: `3141a36ab347631f3ce0c32afea50d55114bbb3781fca900c6b39d48f1c9fe2c`.

The real Keychain roundtrip remains an ignored test with an explicit interactive
label. Run it only when interaction is acceptable; unattended checks do not run
it. File-store acceptance is not a claim of renewed OS-Keychain acceptance.

The [native gate](e2e-vault-native-ui.json) passes on a rebuilt, ad-hoc signed
bundle: UI profiles/contact/message/reply, CLI/MCP/revoke, network/trust restart
regressions and 1051 messages across 22 history pages. All 754 inputs and actual
bundled binaries stayed unchanged. Screenshots were inspected against the prior
native reference. Native processes and temporary profiles were cleaned up.
[Targeted checks](e2e-vault-checks.json): 12 backend, 23 frontend, Clippy and fmt;
one explicitly interactive Keychain test is ignored. The first build's error
conversion failure and initial private-directory setup failure are retained.

The subsequent [paid GUI gate](GUI_ACCEPTANCE.md) also passes through repeated
native restarts, actual UI payment/budget/send and loss/recovery on the same vault
bundle. No native test process remained after teardown.
