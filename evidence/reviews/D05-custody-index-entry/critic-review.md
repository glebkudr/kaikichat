# Compact index descriptor test review

R1 FINAL REVISE before production: add a valid independently signed descriptor
for a damaged envelope with the exact matching hash/length/metadata, requiring
full envelope authentication; assert actual outbox equality after ACK/restart
and no mutation on the unprepared rejection. R2 adds both and asserts real SQL
corruption updates affect one row. Real MLS epoch-transition refusal is a
nonblocking remaining scenario; no epoch-lifecycle acceptance is claimed.

R2 FINAL ACCEPT before production. All six hashes match; both blocking issues resolved. Independent critic recomputed damaged-envelope SHA256 and verified the sole changed signature byte. No remaining blocking issues.

All 35 crypto and two targeted Core tests pass. Clippy first rejected one redundant production field name (fixed), then two unnecessary mutable test bindings because export is read-only. R3 removes only those two mut keywords; no assertion/fixture or production behavior changed.

R3 FINAL ACCEPT: restoring exactly two mut keywords reproduces the R2 hash. R4 adds strict owner IPC export to the existing real Noise-delivery/restart/archive test, before adding the owner production route.

R4 FINAL ACCEPT before owner IPC production. All seven hashes match; real unknown-method RED, strict owner boundaries and actual process/restart/envelope checks accepted. Optional typed forbidden-field values not required for acceptance.
