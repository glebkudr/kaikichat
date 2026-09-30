# Independent backend test review

Reviewer: `/root/successor_policy_test_critic`, created without inherited context.
Production changes began only after the final **ACCEPT**.

The first review returned **REVISE**: corrupt retained evidence was tested only
through lookup, and proposal verifiers did not independently reject altered
received entries under valid authority. Tests now exercise malformed serialized
records and damaged QCs through lookup, proposal, verification and genuine-QC
finalization, with unchanged stored bytes/revisions. Ordinary parent and closing
payload mutations isolate received-entry validation.

The final review accepted both fixes and the suggested positive nonempty suffix,
suffix duplicate, and later-time exact retry preserving original QC/time. It
found no further blocking business scenarios in the application-policy contract.

RED: the revised suite failed with 33 missing-method E0599 errors before any
production change. The extracted shared helpers preserved all seven existing
spent-history tests. These results establish tests-first evidence; runtime GREEN
is recorded separately after implementation.

This review does not accept peer bootstrap, public client lineage, real funded
multi-node consensus or the full V1 release. Those remain required work.
