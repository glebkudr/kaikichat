# Native checkpoint selection test review

Reviewer: separate, originally context-free `/root/node_test_critic`.
Baseline: `ebad95d`; production unchanged until final review returned.

Final verdict: **ACCEPT**.

The reviewer accepted read-only preview, exact normalized metadata, immutable selection,
restart durability, precise ACL denials and the native scenario using actual commands,
independent signatures and a separate daemon. RED matched missing implementation.

Nonblocking suggestion: also test duplicate-key JSON and an extra `now` field through
the new native preview bridge. Strict duplicate-key validation is already exercised
by the new Core test. No critical missing scenarios were identified.

The native fixture has a synthetic current timestamp and proves native acceptance
and persistence under explicitly selected test attestors. Live Anvil funding remains
an independent mandatory gate; this fixture does not claim live finality or spending.
