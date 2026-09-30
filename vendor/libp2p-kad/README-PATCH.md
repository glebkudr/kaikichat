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

The project regression lives in
`crates/node/src/service_throughput_tests.rs`:
`kad_resets_release_slots_and_same_connection_recovers_after_admission_window`.
It uses the real handler, TCP/Noise/Yamux, normal admission and signed records;
the test observer controls only when Reset is delivered. It is run as an
`agentic-node` test, with the workspace's existing test dependencies.

See `Docs/kad-reset-verification.md` for commands, acceptance criteria and
follow-up work. The patch has not been compiled or tested in the authoring task
because the user requested instructions only for execution. Remove this vendor
patch only after an upstream release contains the fix and the same regression,
unchanged A04 and H11 pass with that release.
