FINAL REVISE — independent reviewer /root/common_context_test_critic, no context inherited.

Blocking: the second-chain test uses transport seed [71;32], while its independent
vector uses [72;32]. Both exact byte compatibility and verification therefore fail.
Use the corresponding transport consistently and retain both assertions.
Nonblocking: share the registrar fixture module instead of importing it twice.
Useful additions: well-formed stored roster with corrupted proof; genuine weak
transport signature; positive hints with foreign committee/epoch denied authority.
All four test/spec and 89 production hashes matched; no files changed or runtime
checks performed. Multi-key restoration/revisions and renewal timelines are coherent.
