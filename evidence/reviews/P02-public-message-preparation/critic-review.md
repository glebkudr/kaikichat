# Independent backend test review

Reviewer: `/root/public_postage_test_critic`, an independent agent originally
started without inherited context. No production edits preceded its R3 FINAL
ACCEPT. The reviewer did not edit files or run builds.

- **R1 REVISE:** real MLS/funding verification was meaningful; new composition
  needed current authority and compatible checkpoint refresh, canonical alias,
  exact pre-existing envelope after SQL failure, and corrupted association cases.
  Actual missing-API RED: `red-r1.log`, exit 101, 19 E0599 diagnostics.
- **R2 REVISE:** all requested scenarios were added. The SQL-state assertion used
  a helper limited to `l2/%`, making its custody-state collections empty. Actual
  missing-API RED: `red-r2.log`, exit 101, 29 E0599 diagnostics.
- **R3 FINAL ACCEPT:** direct SQL query reads `custody/envelopes/%`; an explicit
  precondition checks zero or one real row, then revision/bytes are compared.
  Five reviewed input hashes matched. Production implementation began afterwards.
- **R4 FINAL ACCEPT:** first compilation exposed an unsupported rusqlite `u64`
  FromSql target in the new test helper. The exact correction reads `i64` then
  converts to `u64`, matching the existing helper. No assertions were changed.
  `targeted-green-r3.log` is the FAILED compilation attempt (exit 101), despite
  the originally chosen filename; it is not a successful result.
- **R5 FINAL ACCEPT:** rustfmt only wrapped that closure into a block. Reverse
  replacement restored the exact R4 hash, and all five R5 hashes matched. The
  reviewer also read `targeted-r4.log`: 16 passed, zero failed/ignored, consisting
  of eight new and eight pre-existing wallet tests. That run used the formatted
  R5 sources; its filename reflects the pre-format revision.

Final verdict: **ACCEPT**. No blocking missing scenarios for this Core module.
An oversize test with a genuinely funded smaller resource class remains a
non-blocking improvement; the positive test checks actual wire bytes against the
independently verified funded class. There is no fabricated funding fixture.

This accepts the tests and recorded targeted run. General backend/frontend
regressions are recorded separately. It does not accept automatic sender
scheduling, network submission, UI/MCP wallet lifecycle, R10 or the full V1 app.
