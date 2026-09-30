# Wire the durable index into the real daemon

1. Define bounded index publication/read and location-query messages on the
   existing custody transport. Authenticate the actual Noise peer, current
   admission for new index promises and historical evidence for existing claims.
   Keep the descriptor/QC common bundle separate from bounded location pages.
2. Publish the index and locations from the durable sender stages. Persist success
   before removing work; do not use an unconfirmed put or direct recipient ACK as
   an index-storage fact.
3. Prove absent-sender discovery through a genuinely disjoint data roster, then
   restart/cache loss, corrupt or incomplete pages and unavailable index nodes.
   Match fetched ciphertext against the descriptor before processing it.
4. Add book/epoch links and explicit completeness/gap semantics, then safe
   successful sender retirement and autonomous data/index repair. Same-position
   transport replacement needs a bounded explicit policy rather than overwriting
   the original live claim silently.

Keep the full 67-card / 22-E2E / three-platform V1 scope. New backend tests precede
production and require an independent context-free critic. Use affected clusters
until the final complete-plan suite.
