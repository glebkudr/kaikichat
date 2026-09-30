from pathlib import Path
from mutation_runner import verify
ROOT=Path(__file__).resolve().parents[2]
p='crates/core/src/operator_identity.rs'
verify(ROOT,'L02-operator-identity',['core','store','crypto','capabilities','protocol-types','l2-types','l2-adapter'],
 'agentic-core','registry',14,[
 {'name':'reused-seed','path':p,'test':'operators::fresh_multiple_keys_are_private_idempotent_and_durable_before_any_checkpoint_head','changes':[
 ('seed: random_id()?,','seed: [33; 32],')]},
 {'name':'owner-retry','path':p,'test':'operators::fresh_multiple_keys_are_private_idempotent_and_durable_before_any_checkpoint_head','changes':[
 ('if entry.owner != owner {','if false && entry.owner != owner {')]},
 {'name':'stored-opening','path':p,'test':'operators::corrupt_or_rebound_secret_state_is_never_replaced_or_used_for_signing','changes':[
 ('|| e.commitment != e.commitment(stored.registry_domain)','|| false && e.commitment != e.commitment(stored.registry_domain)')]},
 {'name':'binding-lifetime','path':p,'test':'operators::restored_paid_keys_produce_exact_independent_canonical_wires_and_finite_leases','changes':[
 ('const BINDING_SECONDS: u64 = 60;','const BINDING_SECONDS: u64 = 600;')]},
])
