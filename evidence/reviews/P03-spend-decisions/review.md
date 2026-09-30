# Independent backend test review

The existing no-context `common_context_test_critic` reviewed each test revision
and completed before implementation continued. Baseline: `0e5cb76`.

1. R1 **REVISE**: fixture fingerprint failed because a transitive helper scanned
   new spend test files while they were being authored; cryptographic seal mutation,
   vote-path prefix rejection, new-session restoration, stored-proof authentication
   and a genuine later-epoch authority control were missing.
2. R2 **REVISE**: all prior blockers corrected and real fixtures verified. The
   registrar helper still derived a private test scalar from registration index;
   actual added index 17 uses scalar 2030. The frozen random committee happened
   not to select that member, but fresh authoring could fail before the target check.
3. R3 **FINAL ACCEPT**, before production: actual SEC1 keys now resolve the known
   public-test registrar keyspace. A guaranteed full-16 Core control covers the
   added member independently of the random four-member selection; generic authority
   succeeds and incompatible paid policy fails. All 15 frozen hashes matched.
4. R4 **FINAL ACCEPT**, after nine tests passed: the only test change removed an
   unused `mut` from the simulated network oracle binding. Reversing that one-word
   edit exactly restored the accepted R3 hash. Other fourteen inputs and both
   production-file hashes remained unchanged; assertions/data/behavior did not change.

The critic independently verified both genuine receipts under the fixed image,
including complete journals, and ran the fixture authority positive control.
A previous Core route test using the shared registrar helper also passed.

Non-blocking missing cases: explicitly count proposals of both journals in the
four-engine scenario; independently expire a checked context before its service
lease. The test package is accepted for this bounded module; full P03/V1 is not.
