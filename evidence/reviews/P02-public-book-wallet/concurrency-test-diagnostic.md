# R2 test setup defect

Seven tests passed in green-attempt-1.log. The eighth incorrectly attempted two simultaneous ProfileStore opens followed by a barrier. The existing ProfileStore::open lifetime OS lock rejects the second with ProfileInUse before the barrier; the other thread waits forever. The exact known test process was terminated after 111 seconds; no production daemon or store lock was changed. This is a test setup defect, not application deadlock evidence.

R3 replaces that case with the actual ownership contract: second open rejects without changing state; after the first owner reserves and closes, its successor reopens, retries the exact prior stamp and allocates the next distinct index. Both independently authored fixture signatures are checked by a separate receiving Core. The spec now explicitly places actual concurrent client submission in the subsequent shared daemon integration gate.

Production draft remains paused pending R3 test critic. Original R2 RED log contained only 42 missing wallet method errors after external restoration of storage.
