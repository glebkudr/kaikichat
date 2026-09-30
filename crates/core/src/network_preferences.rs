//! Durable owner routing preferences. Transport-specific validation belongs to the node adapter.
use super::*;
const STATE: &str = "network/preferences";
const MAX_BYTES: usize = 4096;

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct NetworkPreferences {
    pub relays: Vec<String>,
    pub relay_only: bool,
    pub auto_nat_peers: Vec<String>,
    #[serde(default)]
    pub bootstrap_peers: Vec<String>,
    #[serde(default)]
    pub lan_discovery: bool,
    #[serde(default)]
    pub dht_server: bool,
}
impl NetworkPreferences {
    fn validate(&self) -> Result<(), CoreError> {
        if self.relays.len() > 4
            || self.auto_nat_peers.len() > 4
            || self.bootstrap_peers.len() > 4
            || (self.relay_only && self.relays.is_empty())
        {
            return Err(CoreError::InvalidInput);
        }
        valid_addresses(&self.relays)?;
        valid_addresses(&self.auto_nat_peers)?;
        valid_addresses(&self.bootstrap_peers)
    }
}
#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct StoredNetworkPreferences {
    pub revision: u64,
    pub preferences: NetworkPreferences,
}
#[derive(Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct Stored {
    version: u8,
    preferences: NetworkPreferences,
}
impl AppCore {
    /// Trusted owner/node adapter only; this is not an agent broker method.
    pub fn network_preferences(&self) -> Result<Option<StoredNetworkPreferences>, CoreError> {
        let Some(state) = self.store.state(STATE)? else {
            return Ok(None);
        };
        if state.bytes.len() > MAX_BYTES {
            return Err(CoreError::InvalidState);
        }
        let stored: Stored =
            serde_json::from_slice(&state.bytes).map_err(|_| CoreError::InvalidState)?;
        if stored.version != 1 || stored.preferences.validate().is_err() {
            return Err(CoreError::InvalidState);
        }
        Ok(Some(StoredNetworkPreferences {
            revision: state.revision,
            preferences: stored.preferences,
        }))
    }
    /// Commit before changing live transport. A lost response may retry the exact same value.
    pub fn save_network_preferences(
        &mut self,
        preferences: NetworkPreferences,
        expected_revision: u64,
    ) -> Result<StoredNetworkPreferences, CoreError> {
        preferences.validate()?;
        let stored = self.network_preferences()?;
        let revision = stored.as_ref().map_or(0, |s| s.revision);
        if expected_revision <= revision
            && let Some(saved) = &stored
            && saved.preferences == preferences
        {
            return Ok(saved.clone());
        }
        if expected_revision != revision {
            return Err(agentic_store::StoreError::StateConflict.into());
        }
        let next = revision.checked_add(1).ok_or(CoreError::InvalidInput)?;
        let bytes = serde_json::to_vec(&Stored {
            version: 1,
            preferences: preferences.clone(),
        })
        .map_err(|_| CoreError::InvalidInput)?;
        if bytes.len() > MAX_BYTES {
            return Err(CoreError::InvalidInput);
        }
        self.store.commit_states(vec![StateChange {
            namespace: STATE.into(),
            expected_revision: revision,
            bytes,
        }])?;
        Ok(StoredNetworkPreferences {
            revision: next,
            preferences,
        })
    }
}
