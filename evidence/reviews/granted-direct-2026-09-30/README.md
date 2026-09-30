# A granted stamp sent directly; low trust while the chain is silent (2026-09-30)

The LAN research ([lan-without-internet-2026-09-30](../lan-without-internet-2026-09-30/README.md)
on branch `claude/priceless-chandrasekhar-8fc244`) found that a stamp from a
granted book, how the identity server funds every new user, was never
accepted on direct delivery: the recipient read its book from `BookShop`
only, and the grant was shown only to holders. Online the holders carried
the message instead; offline on a LAN nothing arrived. In the rig, Bob's
node asked the shop for the book 10 times in 600 managed seconds.

## Decisions

- A stamped direct delivery carries the grant of a granted book
  (`StampedDelivery.grant`; absent for a bought book, so nodes not updated
  yet still decode it). The recipient checks the grant as holders do: the
  issuer's rules for its day, then its notaries (`offer_grant`), refusing
  the delivery as `grant_pending` meanwhile.
- The owner's decision for a recipient that cannot check a stamp (asked:
  wait for the check, or accept): waiting means that offline nobody gets
  anything, and stamps defend against Sybil senders, which a LAN does not
  have. So a one-to-one contact's message whose stamp cannot be checked now
  is taken with low trust, visibly marked. Group entities never work this
  way (the owner confirmed); neither does a stranger's contact request.
- "Cannot check now" is per source: the last read of the stamp's book, or
  of its grant's issuer rules for the day, did not reach the chain. A
  node-wide "last chain read failed" was rejected in review: unrelated reads
  (the registry) fail on their own. A book the chain answered absent, and a
  grant whose rules were read while its notaries have not vouched, are
  refused.
- The mark is kept with the message in core (`Event::Text.low_trust`) and
  shown in the snapshot and the window's history (`lowTrust`), the owner's
  and agents' inboxes and `kaiki messages` (the key only when true). The
  window shows "Low trust" next to the time, with the reason as its title,
  in all 20 languages.
- Open for the owner: the mark stays when a holder-checked copy of the same
  message arrives later.

## Tests first

Backend tests were written before the code and reviewed by an independent
critic: REVISE twice (the node-wide "unreachable" flag, which the rig's
failing registry reads would have set; a stranger test whose second attempt
fell outside the failed read's memory; missing: a known book checked with
the chain down, the unrecorded slot, the JSON shape over IPC, the agents'
inbox, the free network), then ACCEPT.

`crates/node/src/reproducer_mailbox_swarm_tests.rs`, section "direct
delivery pays":

- `a_granted_stamp_sent_directly_pays_once_its_notaries_vouch` (replaces the
  characterization `a_granted_stamp_sent_directly_is_refused`);
- `a_granted_stamp_its_notaries_saw_late_is_refused_directly`;
- `an_unreachable_chain_takes_a_contacts_granted_stamp_with_low_trust`;
- `a_contacts_unknown_book_is_taken_with_low_trust_only_while_the_chain_is_unreachable`;
- `an_unreachable_chain_takes_no_strangers_request_unchecked` (guard);
- `a_bought_books_stamped_delivery_keeps_the_wire_of_nodes_before_grants`;
- `a_stamped_delivery_counts_only_for_its_own_mailbox_and_stamp` also checks
  that a free network marks nothing (guard).

`crates/core/tests/support/mailbox_swarm.rs`:
`a_message_taken_without_checking_its_stamp_is_shown_with_low_trust`.
`apps/desktop/tests/chat-shell.test.tsx`: "low trust".

Mutations, each caught: the sender leaves the grant out (3 grant tests
fail); low trust switched off (2 fail); strangers taken with low trust (the
stranger test fails); the window without the mark (its test fails).

## Results

- `cargo test --locked -p agentic-node --lib`: 268 passed, 3 ignored.
- `cargo test --locked -p agentic-core`: 164 + 11 passed.
- Workspace clippy with `-D warnings` and `cargo fmt --check`: clean.
- The window: vitest 131 passed (the translation checks included), `tsc`,
  `vite build`.
- Native `swarm_native::native_lan_without_internet` (debug build, one Mac),
  run in a throwaway worktree of this branch merged with
  `claude/priceless-chandrasekhar-8fc244`, its unknown-book step changed as
  in [lan-scenario.patch](lan-scenario.patch). Carol writes Bob, who never
  read her bought book, while his chain RPC hangs: before, the message stayed
  "Queued"; now it is taken with low trust **16.2 s** after sending (the RPC's
  10 s timeout, then the sender's backoff), "Delivered" at Carol at once.
  Bob gets the chain back: her next message is checked, without the mark,
  12.1 s after his restart. Report: [native-lan.json](native-lan.json).
  The native scenario pays with bought books; the grant path is covered by
  the rig.
- Native `swarm_native::native_contacts_and_groups` on this branch: passed
  (287 s): contacts, direct and swarm delivery with bought books, groups.

## Screenshots

The visual fixture (`apps/desktop/tests/visual.tsx`) has a message received
with low trust: [dark, English](chat-low-trust-dark-en.png),
[light, Russian](chat-low-trust-light-ru.png),
[dark, Arabic](chat-low-trust-dark-ar.png).

## When both branches land

`native_lan_without_internet` (on `claude/priceless-chandrasekhar-8fc244`)
still expects the old behavior at "Bob never read Carol's book": apply
[lan-scenario.patch](lan-scenario.patch) there, and update that README's
"A book from a grant is never accepted directly" and limits 2 and 3.
