# Independent backend test review

Reviewer: `/root/client_lineage_test_critic`, created without inherited context.
Final verdict: **ACCEPT** before lineage production changes.

First verdict **REVISE** identified two gaps: cold two-link validation needed a
corrupt earlier closing while the last closing remained valid; the SQL-failure
test needed to establish that the first original closing survived before retry.
Both were fixed. The first successful retry now carries an alternate valid QC and
later verification time while preserving the first original ClosingRecord.

The suggested receipt-signature mutation at unchanged context/journal was added
with a positive control. The reviewer accepted owner/target, direct skip,
conflicting-choice, role-free public state, expiry/revocation and existing
handover compatibility coverage. Peer bootstrap and funded consensus remained
explicitly outside this application test acceptance.

Initial and revised RED logs contain only missing-export E0432. Once production
exports existed, Rust revealed E0509 in the test's attempt to move StateValue's
bytes (the type implements Drop). The mechanical fix clones the same bytes;
the same independent critic confirmed **ACCEPT remains** before further work.
No assertion was removed or weakened. The final run passed all five client tests
plus the seven import and five successor-policy tests.

Raw failed and successful logs remain under output/ar1-client-lineage.
