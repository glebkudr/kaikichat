# Bans, doors, open groups and channels, native — 2026-09-29

Three ignored scenarios in `crates/node/tests/support/swarm_native.rs`
(they need anvil, forge and cast from the build wrapper), each on its own
local anvil chain with the contracts deployed and ten bonded holders, every
profile with a book bought on that chain:

```
python3 scripts/build-storage.py run cargo test -p agentic-node --test processes -- --ignored --exact swarm_native::native_bans_doors_and_open_groups --nocapture
python3 scripts/build-storage.py run cargo test -p agentic-node --test processes -- --ignored --exact swarm_native::native_public_channel --nocapture
python3 scripts/build-storage.py run cargo test -p agentic-node --test processes -- --ignored --exact swarm_native::native_closed_channel --nocapture
```

The three ran at the same time on one Mac (30 holders and 13 profiles);
the times below are wall-clock seconds from the full reports in
[native.json](native.json). Design:
[Docs/V1_LARGE_GROUPS_CHANNELS_2026_09_28.md](../../../Docs/V1_LARGE_GROUPS_CHANNELS_2026_09_28.md).

## Bans, doors and an open group — passed in 420 s

Alice owns a group of Bob and Carol; Bob is an admin. Dave is a stranger
to it who lets no stranger in by himself.

| Step | Seconds |
|---|---|
| holder directories complete | 30.6 |
| every card published | 5.6 |
| Bob and Carol joined the group Alice made of their ids | 24.7 |
| Bob made an admin (all three at the next epoch) | 5.6 |
| Bob bans Carol: the commit decided at Alice and Bob | 1.0 |
| Carol's node sees itself out | 4.0 |
| Bob reads what Alice wrote after the ban; Carol never does | 4.5 |
| Alice adding Carol back is refused (`banned`) | — |
| Alice lifts the ban (the list is empty at both) | 6.0 |
| Alice opens the group to everyone | 5.0 |
| Dave follows it and reads Bob's post by Bob's certificate | 3.5 |
| Carol, unbanned, knocks at the open door and is let in at once | 31.2 |
| Dave reads Carol's post by the certificate the batch gave her | 28.7 |
| a second group, by request: Dave's application listed at Bob's | 8.1 |
| Bob accepts; the next batch lets Dave in (all three see him) | 42.3 |
| the group reads Dave's first message | 4.0 |

Both doors' batches were made once (`door.batches` 1 at Alice and at Bob).

## A public channel — passed in 443 s

Alice owns the channel "News", Bob is its team (an admin), retention
90 days. Dave follows before the posts, Erin after the archive is laid.
Three posts of 23 KB each (Alice, Bob, Alice) fill an archive part.

| Step | Seconds |
|---|---|
| holder directories complete | 30.1 |
| Bob in the team | 24.8 |
| retention set to 90 days (a commit) | 7.6 |
| Dave reads the three posts (his first read after following) | 2.5 |
| a part of two posts (47 386 bytes) laid by Alice's node and stored at a quorum | 169.0 (171.6 after the last post) |
| Erin, following later, reads the three posts and the archive part | 2.5 |

`channel_storage` at Alice: retention 90, one part, 47 386 bytes,
2 stamps a month, one part added in the last month. Erin's follow shows
kind `channel` and retention 90. The part waited for the team node's
archive check (every 5 minutes); closing a part by its age (25 days) and
laying it again stay with the managed-time rig.

## A closed channel — passed in 1874 s

Alice owns the channel "Club", Bob is its team. Carol keeps her keys
waiting for her own decision (manual policy); Dave and Erin take theirs.

| Step | Seconds |
|---|---|
| holder directories complete | 30.1 |
| Bob in the team | 25.4 |
| keys given to Dave, Erin and Carol; Dave and Erin publish keys of their own; Carol's wait | 90.8 |
| the first post read by Dave, Erin and Bob | 246.9 |
| Bob, an admin, takes Erin off (the commit at Alice and Bob) | 5.5 |
| Bob's node publishes the key update; Dave takes it | 297.8 |
| Dave reads the next post; Erin does not | 303.3 |
| Alice takes Bob off the team | 1.5 |
| Alice's node reseeds onto the subscribers' own keys by itself (hard reseed), Dave takes it, Carol is given keys again | 301.8 |
| Carol accepts; Dave and Carol read the post written after it; Bob and Erin never do | 303.4 |

At the end: Alice's node made one hard reseed (Bob's none), took three
subscribers' own keys (Dave, Erin, and Carol's after she accepted), and
Dave took two key updates. Carol reads only what was written after her keys
were given (one post). The steps of about 300 s are the subscribers' pace:
a closed channel's subscriber reads every 5 minutes, as designed (part 4).

## Found

- No defect in the product. The first attempt of the first scenario failed
  when Dave knocked right after the group went "by request": the door's card
  was not stored yet and `join_group` answered `card_not_found` (its message
  speaks of "intro mailboxes"). The scenario now knocks again while the card
  is looked up or missing, as an owner or agent would; the rerun found the
  card on the second try (1 s).
- Every profile counted 12–16 `access` refusals at start: holders answer
  `access_required` or `unknown_book` until they read the new book from the
  chain, then serve it.

Not covered natively: the 25-day close and re-lay of archive parts, the
return after 30 days offline, groups of hundreds (all in the rig).
