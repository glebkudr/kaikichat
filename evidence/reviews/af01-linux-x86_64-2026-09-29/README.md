# V1-AF01 on Linux x86_64 — 2026-09-29

A clean `ubuntu:24.04` container (`linux/amd64`, glibc 2.39, OrbStack on
macOS) with only `curl`, `ca-certificates` and `python3` added runs
[af01.sh](af01.sh); its output is [run.log](run.log) (the skill text and
the claim's id elided).

| Step | Result |
|---|---|
| `curl -fsSL https://kaikichat.com/install.sh \| sh` | installed `kaiki 0.1.0` in 2.9 s, checksum checked |
| `kaiki skill show` | the owner skill, naming the installed path |
| `daemon status` before a profile | `{"running":false}`, exit 0 |
| `init --name` (twice) | the same network id both times, exit 0 |
| `network` | the signed preset: `kaiki-testnet-base-sepolia`, serial 2, `current` |
| `daemon status` | running, `networkId`, `peerId` |
| `coins balance` | no books, 0 remaining |
| `coins claim` | first `claim_pending` (exit 4, retryable), then a login link on `id.kaikichat.com` |
| the link | 200: a page with Google and GitHub; they redirect (303) to Google with client `630672545352-vpue…` and to GitHub with `Ov23lioTFA6BO4tTRSF3`, callbacks on `id.kaikichat.com`, PKCE |
| `coins buy` | a payment request on chain 84532: `BookShop.buy` calldata, the ETH quote at the Chainlink rate (+1 %), the USDC steps |
| `contacts request --id ain1nope` | `invalid_input`, exit 2 |
| `send --to nobody` | `unknown_contact`, exit 3 |
| a wrong `AGENTIC_PASSWORD` | `secrets_locked`, exit 2 |
| `daemon stop`, `status`, `start` | stopped, then the same network id and peer id after the restart |

Secrets: `AGENTIC_SECRETS=file` (no Secret Service in the container), the
password from `AGENTIC_PASSWORD`.

Not done here: signing in with Google or GitHub on the link, which needs a
person's account in a browser; the grant path itself is covered by V1-GF01
and the identity server's tests. The preset served now has no `welcome`
(serial 2).
