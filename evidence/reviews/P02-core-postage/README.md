# Owned Core postage preparation evidence

This increment connects the existing SQLCipher funded wallet to the existing strict
local proving request. It creates no proof job, network authority or spend record.

Five tests were written first. The first independent review rejected an expiry
check that was confounded with registry admission expiry; the corrected tests use
end-1, end and a longer positive context at the same clock. They also explicitly
drop an unused opaque handle. FINAL ACCEPT preceded production changes. The API
compile RED and the earlier, distinct Anvil fixture timestamp failure are retained
and identified in evidence.json; the latter is not production RED.

The actual fixture generator made paid purchases on chains 31340/31341 and ran 16
existing Rust CLI compatibility checks. Public seeds 81/82 and historical clocks
are deterministic test data. All 5 new tests plus 471 total ordinary Rust, 7 model,
9 oracle and 40 frontend tests passed, with formatting/Clippy and TS/Vite. No
repeated genuine proving, full EVM, native/Linux or packaged application claim is
made. See evidence.json for exact log hashes and limits, source-hashes.json for the
211 unchanged source inputs and accepted-tests.json/accepted-contract.md for the
reviewed pre-implementation artifacts.

Current implementation: crates/core/src/postage_wallet.rs. Remaining work includes
bounded-history funded-anchor retention, common interval policy, daemon proving
and actual parent-death/cancellation/retry, fresh-authority receipt admission,
canonical global spend and the rest of V1.
