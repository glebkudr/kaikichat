# Connect the index server to durable sender and recipient workflows

The accepted server gate is driven through owner and independent raw Noise peers.
It does not replace the following mandatory work:

1. Durable sender stages publish the signed descriptor to the book's selected
   index nodes, authenticate each response against its actual Noise peer and
   persist index receipts and finite data-holder locations before advancing.
   Retry admission must keep the original paid operation and receipt evidence.
2. Latest mailbox pointers route to confirmed book-index nodes. Recipient jobs
   query authorized bounded index/location pages, verify shared original proof
   evidence and bind fetched ciphertext to the signed descriptor before import.
3. Prove automatic offline recovery of at least two messages with disjoint data
   rosters, unavailable index nodes, restart/cache loss, hostile pages and bounded
   continuation. Owner/raw-peer publication or retrieval cannot drive that gate.
4. Book/epoch links, explicit gaps/completeness, safe successful sender retirement
   and automatic data/index repair remain required. Frozen original proofs,
   entitlement limits and one paid operation per message remain intact.

Full V1 remains 67 cards / 22 E2E / three platforms. This slice does not close
AR-R03, R10, AR1 or V1. Use targeted clusters until the final plan-wide suite.
