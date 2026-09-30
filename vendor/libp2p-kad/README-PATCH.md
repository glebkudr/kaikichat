# libp2p-kad 0.48.0: wake inbound streams on Reset

This is the published libp2p-kad 0.48.0 source already used by this project,
with one production change in `src/handler.rs`:
`InboundSubstreamState::close` wakes the saved `WaitingBehaviour` waker after
changing the state to `Closing`.

Without the wake, a stream already suspended in `SelectAll` can remain there
after `HandlerIn::Reset`. Repeated admission refusals can occupy all 32 inbound
slots on a live connection, causing every subsequent Kad stream to be dropped
before a request reaches the behaviour. A normal response already wakes this
same saved waker; closing a refused request must do so too.

No wire format, protocol name, timeout, limit, query algorithm, or dependency
version is changed. The root `[patch.crates-io]` selects this source. The root
Cargo.lock retains the same dependency versions and treats this package as a
path dependency (registry source/checksum removed). The registry cache is not
modified. No package installation or dependency upgrade is involved.

Provenance:

- Published crate: `libp2p-kad` 0.48.0.
- Crate archive checksum: `13d3fd632a5872ec804d37e7413ceea20588f69d027a0fa3c46f82574f4dee60`.
- Upstream commit: `7b9a558e3188eaf20a14bd8388d5e9b6e2aa9a23`.
- Upstream path: `protocols/kad` in <https://github.com/libp2p/rust-libp2p>.
- License: MIT, original notices retained in source files and Cargo.toml.
- `src/`, `tests/`, Cargo.toml, Cargo.toml.orig, CHANGELOG.md, and
  .cargo_vcs_info.json were copied from the existing registry source. Registry
  cache markers and the crate's independent Cargo.lock were not copied.

## Status (2026-09-30)

The patch is in use: the node answers every Kad request it does not serve
with Reset — FIND_NODE with a key over 64 bytes or over the lookup
admission, and every GET_RECORD, PUT_RECORD and GET_PROVIDERS
(`crates/node/src/routing.rs`, [spec/dht-roles-v1.md](../../spec/dht-roles-v1.md)).
Without the wake, 32 such refusals on one connection would keep all its
inbound slots.

It is not covered by a test. Its regression test,
`kad_resets_release_slots_and_same_connection_recovers_after_admission_window`,
read signed service records; it was deleted with them on 2026-09-26, before
this repository's first commit, and no record shows it was ever run. The
native A04/H11 scenarios of the old verification plan are gone too.

Before relying on the patch or removing it:

1. Add a regression test on today's routing: more than 32 refused requests on
   one real TCP/Noise/Yamux connection, then a lookup on the same connection
   is still answered once the admission window refreshes. Check that it
   fails without the `waker.wake()` call in `InboundSubstreamState::close`.
2. Report the bug upstream with that minimal reproducer.
3. Remove this vendor patch only after an upstream release contains the fix
   and passes the same test.
