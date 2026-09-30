# Local owner desktop history

The desktop must show recent messages after a conversation exceeds 1000 stored
events, and provide explicit access to earlier text. Full-message snapshots are
not suitable for periodic UI polling or multi-conversation previews.

The local owner IPC exposes:

* `desktop_overview { after?: conversationId }`: at most 32 conversations ordered
  by ID, identity/network status and `nextAfter`. Each conversation contains only
  its latest text preview (256 Unicode scalar values, then `…` if truncated),
  normal delivery status, and the full unread incoming-text count. Empty/control-
  only conversations have no preview. Each response is independently bounded;
  the desktop adapter gathers successive contact pages.
* `conversation_history { conversationId, before?: messageId }`: the latest 50
  text messages before an exclusive local message cursor, in ascending local
  receipt sequence. The whole serialized page is at most 512 KiB, so large texts
  can reduce its message count. Full bodies are never truncated. `nextBefore`
  names the oldest returned text when more text remains, otherwise it is null.
  A cursor must name a text in the requested conversation. Unknown/malformed or
  other-conversation cursors are errors, not an empty success.
* `desktop_revision {}`: daemon instance, a connection-local database write
  counter, and network status. Tauri compares this small value every 250 ms and
  emits `core:changed` only on a transition. A restarted daemon has a different
  instance. Revision is an invalidation hint, not a durable message cursor.

The two view commands are explicitly listed in the main-window Tauri capability
and enforce the existing local owner window/origin checks. Revision is internal
to the native bridge. Agent calls cannot invoke these owner commands, including
with an otherwise valid grant for the selected conversation. Existing scoped
`inbox.poll`/`inbox.ack` and legacy snapshot contracts are retained.

SQLCipher uses guarded JSON event-kind expression indexes for recent text and
unread queries. Existing opaque records remain valid store input. Indexes are
created transactionally when opening existing or new profiles; bodies, MLS state,
operations and outbox are not rewritten. Views neither mark messages read nor
acknowledge transport or agent work. Local order follows receipt sequence, not a
claim about total ordering across remote machines.

The UI loads the selected history separately, preserves full text and delivery
states, and offers an older-page button. Requests that finish after switching
conversations cannot mix their histories. Older-page failures retain messages
and cursor for retry. Overlapping live pages merge by message ID; a disjoint live
page resets the visible window and exposes its older cursor so intervening history
is still reachable. Prepending older messages preserves the scroll position;
until the owner explicitly loads an older page, live updates replace the bounded
recent window rather than accumulating the entire log. Already loaded older
queued messages refresh their delivery status through bounded history requests.
initial history, successful local sends and live arrivals at the bottom scroll
to the latest message. No transport/control event appears as chat text.

Acceptance uses actual MLS events, SQLCipher reopen, mixed existing control events,
large Unicode bodies, permission denials, and a hidden WKWebView scenario with
1051 actual native messages, recipient daemon restart, all 22 UI pages with exact
IDs/text, and a reply. This does not claim missing-range authenticity, R10 storage
placement/repair, or full V1 completion.
