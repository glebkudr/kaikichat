# Fresh native paid GUI — accepted local flow

[Candidate 6](gui-native-candidate-6.json) passes the complete declared
[contract](GUI_TEST_CONTRACT.md) on the rebuilt macOS debug E2E bundle. Its
[754 inputs](gui-native-candidate-6-inputs.json) and all five actual bundled
binaries are unchanged before/after; source hash:
`1b0833cc363e78777af5647778f9a50c326e36aae35cfc3fb9e58796fa129633`.

Two empty profiles create identities and a contact through hidden WKWebViews.
The sender selects trust and registry files, prepares the exact visible purchase,
and an independent Anvil signer pays its decoded transaction. Cold restart, RPC
failure and verified refresh preserve that purchase. Budget and retention are
saved through UI; two ordinary composer messages each acquire one ticket.

Six independently verified QC signatures bind 20 original data receipts,
20 index promises and 200 location acknowledgments. The real native message view
shows storage 10/10 and completed publication while recipient delivery remains
queued. Both originals subsequently recover with the sender absent after real
data/index loss, missing-trust refusal, both SQL import failures and cold retry.
No owner sender-work/retrieval call or legacy prover performs this flow.

The payment details and stored-message screenshots were viewed beside the prior
payment/status references. The visible URI/transaction remain in scrollable fields;
the message view keeps storage and delivery separate. Retained images and exact
UI action observations are listed in [the visual manifest](gui-native-visuals.json).

Automation uses private temporary E2E keys, with no system Keychain access or
password prompts. All owned native windows/daemons stop before profiles and keys
are removed; cleanup errors are empty. [Vault checks](E2E_SECRETS.md) additionally
cover 12 backend and 23 frontend tests and the full ordinary native regression.

Candidates 1–5 remain failed evidence: helper Keychain ACL, an invitation script,
the harness confusing a wallet error with a transport error, asynchronous file
input loading, and repeated Keychain authorization on restart. The final vault
change resolves the last problem without changing ordinary-build Keychain use.
Fixture corrections were independently reviewed; none weakens the funding,
receipt, history or failure oracle.

This establishes local macOS UI/payment/send/recovery. External-wallet product
compatibility, new-agent contact/host/NAT E11, over-128 retained history, MLS
epochs/Welcome, autonomous R10 repair and three-platform V1 remain open. The
report's inherited `successfulSenderRetirement: false` means this GUI fixture
does not independently inspect retirement; its separate accepted evidence is
[the lifecycle gate](status-native-candidate-3.json).
