from pathlib import Path
from mutation_runner import verify
p='crates/core/src/registry_selection.rs'
atomic='selection_and_clock_commit_atomically_and_failed_verification_clock_write_cannot_succeed'
verify(Path(__file__).resolve().parents[2],'L02-registry-core',
       ['core','store','crypto','capabilities','protocol-types','l2-types','l2-adapter'],
       'agentic-core','registry',8,[
    {'name':'install_clock','path':p,'changes':[
        ('self.save_checkpoint_states(checkpoint, now, changes)?;',
         'let old_time = checkpoint.stored.observed_at; self.save_checkpoint_states(checkpoint, old_time, changes)?;')], 'test':atomic},
    {'name':'verification_clock','path':p,'changes':[
        ('self.save_checkpoint(checkpoint, now)?;','let _ = self.save_checkpoint(checkpoint, now);')], 'test':atomic},
    {'name':'saved_binding','path':p,'changes':[
        ('bind_registry(&profile, checkpoint).map_err(|_| CoreError::InvalidState)?;','let _ = bind_registry(&profile, checkpoint);')],
     'test':'persisted_registry_trust_binding_and_profile_are_revalidated_before_use'},
    {'name':'code_replacement','path':p,'changes':[
        ('                    || profile.code_hash() != saved.profile.code_hash()','')],
     'test':'selection_requires_saved_chain_trust_and_cannot_replace_domain_or_code_on_retry'},
])
