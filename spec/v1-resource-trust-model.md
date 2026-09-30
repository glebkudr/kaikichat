# V1 resource and trust model

The executable values are recorded in
`Docs/agentic_internet_v1_execution_plan/v1-plan-2026-09-14/resource-trust-manifest.json`.
That manifest distinguishes protocol limits from testnet parameters and from
economic decisions that are still external inputs.

A message consumes the already funded resource class. Its body, signed wire,
paid retention, target replicas and repair allowance are checked at admission;
an index or repair operation cannot silently mint another ticket. A holder may
return an empty advancing page when earlier claims are expired, but a holder's
`complete` flag never proves global history completeness. Missing references
remain exact repair work under the authenticated pointer and lease.

Trust is layered: the issuer profile fixes deployment economics, an EIP-1186
proof binds the funded batch to a supplied state root, the checkpoint/finality
policy authenticates that root, and the registry proof binds the selected
operator. Each layer fails closed on a changed root, epoch, identity, lease,
resource or clock. No subsidy, royalty, or provider claim grants governance or
storage authority.

The manifest intentionally leaves live chain deployment, campaign funding,
fee split, and independent failure-domain inventory as explicit decisions. A
local contract or component test can verify conservation rules, but it cannot
close those external testnet prerequisites.
