from pathlib import Path
from mutation_runner import verify
p='crates/l2-adapter/src/operator.rs'
binding='valid_signatures_cannot_substitute_purpose_member_root_role_transport_or_lifetime'
verify(Path(__file__).resolve().parents[2],'L02-operator-binding',
       ['l2-adapter','l2-types','protocol-types'],'agentic-l2-adapter','operators',6,[
    {'name':'role_body','path':p,'changes':[
        ('if expected.body()? != document.body() {','if false && expected.body()? != document.body() {')],'test':binding},
    {'name':'operator_author','path':p,'changes':[
        ('if document.author() != &member.node_key().0','if false')],'test':binding},
    {'name':'weak_transport','path':p,'changes':[
        ('if transport.is_weak() {','if false && transport.is_weak() {')],'test':binding},
    {'name':'future_issuance','path':p,'changes':[
        ('            || document.issued_at() > now','')],'test':binding},
    {'name':'effective_lease','path':p,'changes':[
        ('valid_until: expires_at.min(snapshot.valid_until()),','valid_until: expires_at,')],
        'test':'binding_snapshot_and_checkpoint_have_independent_finite_expiry_bounds'},
])
