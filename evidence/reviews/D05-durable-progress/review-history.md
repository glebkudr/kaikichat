# Durable recipient progress review history

Baseline: 05feb4733daacfd1ebbd8bf05c8b23b8a0c712af. Full V1 remains open.

Core R1: REVISE. Actual progress INSERT/UPDATE faults did not exclude progress-first/message-second commits. Required actual final-message INSERT failure on a multi-message page.
Core R2: FINAL ACCEPT. Sixth test injects SQLCipher final-message INSERT failure, checks consistent persisted prefix/cursor, reopens with fault, removes it and retries exact messages with dedup. Positive controls verify text, author, ownership and count. RED: 34 missing API E0599 errors. Manifest 943a32382dc30700ad282eec6c1bbe2a49a2af84b47e6fe20ec2f6d48ded2377.
Core R3: FINAL ACCEPT. Sole fixture correction 11900 -> 11000 four-byte characters leaves room for signed MLS overhead under the existing 48000-byte packet cap. Actual first-five-fit/six-exceed-262144 assertions remain unchanged. Manifest dfc83dfe7ff429fcc00652b6b1da9d16454de994dd22adc82afaf63aa0723f04.

Runtime test first run failed during setup because 47600-byte text plus signed MLS overhead exceeded the custody packet cap. Retained as backlog-r1-red.log and backlog-r1-invalid-packet-*; not a behavioral RED claim.
Runtime corrected baseline run created 128 actual MLS envelopes, 5748073 bytes, 26 nonempty pages. First two pages imported 10 originals. The third real request at cursor10 was held. After recipient restart the signed cursor was0: captured [0,5,10,0]. Expected10 assertion failed. Sender stopped and Bob observer allowlist applied. Retained backlog-r2-red.log, backlog-r2-red-evidence.json and backlog-r2-red-trace.json.
Runtime R1 independent review pending. No production implemented at this record.

Runtime R1: REVISE. Total lookups>=2 could count startup work instead of proving the 16-page visit boundary. Runtime R2: FINAL ACCEPT after recording the first restarted request lookup baseline and correlating captured request counts with later lookups; 17th resumed request requires a later lookup. Manifest cf5da9011420a8b3ae258a5ce7886798574ae0c684321228d2da2b0cd1aac8e8.
Core/Runtime production implemented after both acceptances. Core six new tests GREEN; all112 Core conversations tests and51 frontend tests passed.
Core R4: FINAL ACCEPT for exactly three clone-to-slice::from_ref test-only lint corrections. Reversing them restores acceptedR3 hash. Manifest40b1acd6f04114ebf4537c49cad7eeab054ccf88b48d73d63e888a00a83a95a9. Clippy subsequently passed.
