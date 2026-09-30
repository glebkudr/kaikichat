use super::*;
use ed25519_dalek::SigningKey;
use zeroize::{Zeroize, Zeroizing};

#[derive(Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct ProvisionRuntimeRequest {
    pub operation_id: String,
    pub name: String,
    pub agent_id: [u8; 32],
    pub service_id: [u8; 32],
    pub conversation_ids: Vec<String>,
    pub actions: BTreeSet<Action>,
    pub expires_at: u64,
    pub max_data_bytes: u64,
}
/// Only this public projection may cross the owner UI boundary.
#[derive(Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct RuntimeInfo {
    pub grant_id: [u8; 32],
    pub name: String,
    pub principal: [u8; 32],
    pub agent_id: [u8; 32],
    pub service_id: [u8; 32],
    pub conversation_ids: Vec<String>,
    pub actions: BTreeSet<Action>,
    pub expires_at: u64,
    pub max_data_bytes: u64,
    pub status: &'static str,
}
/// Private host material. Intentionally has no Serialize or Debug implementation.
pub struct ProvisionedRuntime {
    pub runtime: RuntimeInfo,
    pub grant: RuntimeGrant,
    pub ownership_epoch: u64,
    pub signing_seed: Zeroizing<[u8; 32]>,
}
#[derive(Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct ProvisionRecord {
    version: u8,
    intent_hash: [u8; 32],
    grant_id: [u8; 32],
    ownership_epoch: u64,
    signing_seed: [u8; 32],
}
impl Drop for ProvisionRecord {
    fn drop(&mut self) {
        self.signing_seed.zeroize();
    }
}
fn info(registration: &Registration, now: u64) -> RuntimeInfo {
    let request = &registration.request;
    RuntimeInfo {
        grant_id: registration.grant.grant_id,
        name: request.name.clone(),
        principal: request.principal,
        agent_id: request.agent_id,
        service_id: request.service_id,
        conversation_ids: request.conversation_ids.clone(),
        actions: request.actions.clone(),
        expires_at: request.expires_at,
        max_data_bytes: request.max_data_bytes,
        status: if registration.revoked {
            "revoked"
        } else if request.expires_at <= now {
            "expired"
        } else {
            "active"
        },
    }
}
impl AppCore {
    /// Owner-only metadata, never available through the signed agent dispatcher.
    pub fn list_runtimes(&self, now: u64) -> Result<Vec<RuntimeInfo>, CoreError> {
        let (registry, _) = self.registry()?;
        Ok(registry.runtimes.values().map(|r| info(r, now)).collect())
    }
    pub fn provision_runtime(
        &mut self,
        request: ProvisionRuntimeRequest,
        now: u64,
    ) -> Result<ProvisionedRuntime, CoreError> {
        if request.operation_id.is_empty()
            || request.operation_id.len() > 128
            || request.operation_id.chars().any(char::is_control)
        {
            return Err(CoreError::InvalidInput);
        }
        let namespace = format!(
            "authorization/provision/{}",
            hex::encode(Sha256::digest(request.operation_id.as_bytes()))
        );
        let intent_hash: [u8; 32] =
            Sha256::digest(serde_json::to_vec(&request).map_err(invalid)?).into();
        let (registry, _) = self.registry()?;
        if let Some(state) = self.store.state(&namespace)? {
            let saved: ProvisionRecord =
                serde_json::from_slice(&state.bytes).map_err(|_| CoreError::InvalidState)?;
            if saved.version != 1 || saved.intent_hash != intent_hash {
                return Err(agentic_store::StoreError::IdempotencyConflict.into());
            }
            let registration = registry
                .runtimes
                .get(&hex::encode(saved.grant_id))
                .ok_or(CoreError::InvalidState)?;
            if registration.revoked
                || registration.request.expires_at <= now
                || saved.ownership_epoch != registry.ownership_epoch
            {
                return Err(CoreError::Unauthorized);
            }
            let key = SigningKey::from_bytes(&saved.signing_seed);
            if key.verifying_key().to_bytes() != registration.request.principal {
                return Err(CoreError::InvalidState);
            }
            return Ok(ProvisionedRuntime {
                runtime: info(registration, now),
                grant: registration.grant.clone(),
                ownership_epoch: saved.ownership_epoch,
                signing_seed: Zeroizing::new(saved.signing_seed),
            });
        }
        let mut seed = Zeroizing::new([0; 32]);
        getrandom::fill(&mut *seed).map_err(|_| CoreError::Randomness)?;
        let key = SigningKey::from_bytes(&seed);
        let grant_request = RuntimeGrantRequest {
            name: request.name,
            principal: key.verifying_key().to_bytes(),
            agent_id: request.agent_id,
            service_id: request.service_id,
            conversation_ids: request.conversation_ids,
            actions: request.actions,
            expires_at: request.expires_at,
            max_data_bytes: request.max_data_bytes,
        };
        let (grant, grant_state) = self.prepare_runtime_grant(grant_request.clone(), now)?;
        let saved = ProvisionRecord {
            version: 1,
            intent_hash,
            grant_id: grant.grant_id,
            ownership_epoch: registry.ownership_epoch,
            signing_seed: *seed,
        };
        self.store
            .commit_states(vec![grant_state, change(&namespace, 0, &saved)?])?;
        let runtime = info(
            &Registration {
                request: grant_request,
                grant: grant.clone(),
                device_epoch: 1,
                service_epoch: 1,
                revoked: false,
            },
            now,
        );
        Ok(ProvisionedRuntime {
            runtime,
            grant,
            ownership_epoch: registry.ownership_epoch,
            signing_seed: seed,
        })
    }
}
