# Doors and channels in the window (2026-09-28)

What the owner's window gained for big groups and channels
(Docs/V1_LARGE_GROUPS_CHANNELS_2026_09_28.md, spec/groups-v1.md).

- A group's "Who reads" chooses one of three: open to everyone (after the
  owner confirms it stays public), by request at its door, members only.
- "Asking to join" lists requests at the door for the owner and admins, with
  the note a stranger wrote shown as text and a mark for members coming back
  after a long absence; let in or turn away each one.
- "Make a group" also makes a channel: its kind, its name, its team (the
  owner's admins) and who reads it (anyone, by request, only those given
  keys). A public channel is made only once the owner confirms it stays public.
- A public channel shows its history: how long posts are kept (30, 90, 180,
  365 days or for ever), the archive parts stored and the stamps a month they
  cost, with a warning that keeping for ever grows every month.
- A closed channel shows its subscribers: give keys to an ID, take them back
  (the channel moves to new keys), new keys for everyone, and the limit that
  an admin taken off the team still knew the old keys at the time (since
  fixed: taking an admin off the team moves the channel to keys sealed to
  the subscribers' own).
- A channel lists its team and has no admin toggles: only the owner adds to
  the team; an admin bans an ID but adds nobody.
- Search finds channels as channels ("Channels" filter, "Read the channel"),
  and a followed channel says only its team writes there.
- Twenty languages; the Tauri shell forwards `door_requests`, `door_decide`,
  `channel_storage`, `channel_subscribe` and `join_group`.

Tests: `tests/channels.test.tsx` (6), `tests/discover.test.tsx` (channel
cards, a followed channel), the translation checks; 113 frontend tests, `tsc`,
`vite build`, 22 native desktop tests, clippy clean for `agentic-desktop`.

Screenshots from the component fixture (`tests/visual.html`, headless
Chrome; `?chat=NAME&members`, `?open=new-group&channel`, `?open=discover&demo`):
[a public channel, dark, Russian](news-dark-ru.png),
[the same, Arabic (RTL)](news-light-ar.png),
[a closed channel's subscribers](club-light-en.png),
[a group with a door and its requests](readers-light-en.png),
[making a channel](new-channel-light-en.png),
[making a channel, dark, Arabic](new-channel-dark-ar.png),
[a channel's card in search](discover-channel-dark-ru.png),
[a followed channel](follow-channel-light-en.png).
