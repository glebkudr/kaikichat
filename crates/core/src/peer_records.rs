//! Bounded routing hints. A root signature proves authorship, never operator independence.
use super::*;
const CACHE: &str = "network/peer-records";
const OWN: &str = "network/own-record";
const CAPACITY: usize = 64;
const MAX_CACHE_BYTES: usize = 640 * 1024;
const MAX_OWN_BYTES: usize = 10 * 1024;

/// Retained with each confirmed contact even when its routing hint expires or is evicted.
#[derive(Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub(super) struct RouteVersion {
    sequence: u64,
    issued_at: u64,
    id: [u8; 32],
}
impl RouteVersion {
    fn of(record: &NodeRecord) -> Self {
        Self {
            sequence: record.sequence,
            issued_at: record.issued_at,
            id: record.id,
        }
    }
    fn admits(&self, candidate: &Self) -> bool {
        if self.sequence == 0 && candidate.sequence == 0 {
            return candidate.issued_at > self.issued_at || candidate == self;
        }
        candidate.sequence > self.sequence || candidate == self
    }
}

#[derive(Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct StoredRecord {
    peer: String,
    issued_at: u64,
    expires_at: u64,
    wire: String,
}
impl StoredRecord {
    fn of(record: &NodeRecord) -> Self {
        Self {
            peer: record.peer_id.clone(),
            issued_at: record.issued_at,
            expires_at: record.expires_at,
            wire: hex::encode(&record.wire),
        }
    }
    fn decode(&self, core: &AppCore) -> Result<NodeRecord, CoreError> {
        if self.wire.len() > 8192 {
            return Err(CoreError::InvalidState);
        }
        let wire = hex::decode(&self.wire).map_err(|_| CoreError::InvalidState)?;
        // Verify at issuance to retain the anti-rollback barrier after expiry. Callers decide
        // whether this record is still eligible for discovery at the actual current time.
        let record = core
            .verify_node_record(&wire, &self.peer, self.issued_at)
            .map_err(|_| CoreError::InvalidState)?;
        if record.issued_at != self.issued_at || record.expires_at != self.expires_at {
            return Err(CoreError::InvalidState);
        }
        Ok(record)
    }
}
#[derive(Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct CacheEntry {
    record: StoredRecord,
    seen: u64,
}
#[derive(Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct Cache {
    version: u8,
    entries: BTreeMap<String, CacheEntry>,
}
impl Default for Cache {
    fn default() -> Self {
        Self {
            version: 1,
            entries: BTreeMap::new(),
        }
    }
}

impl AppCore {
    /// Trusted node publisher only. Persist sequence before returning the signed wire.
    /// Unchanged routes reuse their record until the last half of its one-day lifetime.
    pub fn publish_node_record(
        &mut self,
        peer: &str,
        addresses: Vec<String>,
        now: u64,
    ) -> Result<Vec<u8>, CoreError> {
        let saved = self.store.state(OWN)?;
        let mut sequence = 1;
        if let Some(saved) = &saved {
            if saved.bytes.len() > MAX_OWN_BYTES {
                return Err(CoreError::InvalidState);
            }
            let stored: StoredRecord =
                serde_json::from_slice(&saved.bytes).map_err(|_| CoreError::InvalidState)?;
            let previous = stored.decode(self)?;
            if previous.author != self.store.identity()?.public_key || previous.sequence == 0 {
                return Err(CoreError::InvalidState);
            }
            if previous.peer_id == peer
                && previous.addresses == addresses
                && now >= previous.issued_at
                && previous.expires_at.saturating_sub(now) > transport_binding::LIFETIME / 2
            {
                return Ok(previous.wire);
            }
            sequence = previous
                .sequence
                .checked_add(1)
                .ok_or(CoreError::InvalidState)?;
        }
        let wire = self.sign_node_record(peer, &addresses, Some(sequence), now)?;
        let record = self.verify_node_record(&wire, peer, now)?;
        let bytes = serde_json::to_vec(&StoredRecord::of(&record)).map_err(invalid)?;
        if bytes.len() > MAX_OWN_BYTES {
            return Err(CoreError::InvalidInput);
        }
        self.store.commit_states(vec![StateChange {
            namespace: OWN.into(),
            expected_revision: saved.map_or(0, |s| s.revision),
            bytes,
        }])?;
        Ok(wire)
    }

    fn peer_cache(&self) -> Result<(Cache, u64), CoreError> {
        let Some(saved) = self.store.state(CACHE)? else {
            return Ok((Cache::default(), 0));
        };
        if saved.bytes.len() > MAX_CACHE_BYTES {
            return Err(CoreError::InvalidState);
        }
        let cache: Cache =
            serde_json::from_slice(&saved.bytes).map_err(|_| CoreError::InvalidState)?;
        if cache.version != 1 || cache.entries.len() > CAPACITY {
            return Err(CoreError::InvalidState);
        }
        for (root, entry) in &cache.entries {
            if hex::encode(entry.record.decode(self)?.author) != *root
                || entry.seen > saved.revision
            {
                return Err(CoreError::InvalidState);
            }
        }
        Ok((cache, saved.revision))
    }

    /// Bounded, currently valid routing hints. These records grant no chat or agent authority.
    pub fn cached_node_records(&self, now: u64) -> Result<Vec<NodeRecord>, CoreError> {
        let (cache, _) = self.peer_cache()?;
        let mut records = Vec::new();
        for entry in cache.entries.values() {
            let record = entry.record.decode(self)?;
            if VerifiedDocument::decode(&record.wire, self.domain, now).is_ok() {
                records.push(record);
            }
        }
        Ok(records)
    }

    /// Trusted transport adapter validates supported multiaddrs and actual session PeerID first.
    /// False means duplicate, stale or conflicting sequence; no state is changed for a replay.
    pub fn remember_node_record(
        &mut self,
        wire: &[u8],
        actual_peer: &str,
        now: u64,
    ) -> Result<bool, CoreError> {
        let record = self.verify_node_record(wire, actual_peer, now)?;
        self.remember_verified_node_record(record, now)
    }

    pub(super) fn remember_verified_node_record(
        &mut self,
        record: NodeRecord,
        now: u64,
    ) -> Result<bool, CoreError> {
        let (mut cache, revision) = self.peer_cache()?;
        let (mut data, app_revision) = self.data()?;
        let root = hex::encode(record.author);
        let candidate = RouteVersion::of(&record);
        if let Some(existing) = cache.entries.get(&root)
            && !RouteVersion::of(&existing.record.decode(self)?).admits(&candidate)
        {
            return Ok(false);
        }
        for contact in data.contacts.values().filter(|c| c.root == record.author) {
            if contact
                .route_version
                .as_ref()
                .is_some_and(|v| !v.admits(&candidate))
            {
                return Ok(false);
            }
        }
        let mut app_changed = false;
        for contact in data
            .contacts
            .values_mut()
            .filter(|c| c.root == record.author)
        {
            if contact.route_version.as_ref() != Some(&candidate)
                || (!record.addresses.is_empty() && contact.addresses != record.addresses)
            {
                // Withdrawing currently confirmed publication is not identity revocation.
                // Keep the last authenticated dial hint for retry; the cache below stores
                // the actual empty record, and transport/root checks still gate delivery.
                if !record.addresses.is_empty() {
                    contact.addresses.clone_from(&record.addresses);
                }
                contact.route_version = Some(candidate.clone());
                app_changed = true;
            }
        }
        let cache_changed = cache
            .entries
            .get(&root)
            .is_none_or(|e| e.record.wire != hex::encode(&record.wire));
        if !app_changed && !cache_changed {
            return Ok(false);
        }
        let mut changes = Vec::new();
        if app_changed {
            changes.push(data_change(&data, app_revision)?);
        }
        if cache_changed {
            cache.entries.retain(|_, e| e.record.expires_at > now);
            if !cache.entries.contains_key(&root) && cache.entries.len() >= CAPACITY {
                let oldest = cache
                    .entries
                    .iter()
                    .min_by_key(|(_, e)| e.seen)
                    .map(|(root, _)| root.clone())
                    .ok_or(CoreError::InvalidState)?;
                cache.entries.remove(&oldest);
            }
            cache.entries.insert(
                root,
                CacheEntry {
                    record: StoredRecord::of(&record),
                    seen: revision.checked_add(1).ok_or(CoreError::InvalidState)?,
                },
            );
            let bytes = serde_json::to_vec(&cache).map_err(invalid)?;
            if bytes.len() > MAX_CACHE_BYTES {
                return Err(CoreError::InvalidInput);
            }
            changes.push(StateChange {
                namespace: CACHE.into(),
                expected_revision: revision,
                bytes,
            });
        }
        self.store.commit_states(changes)?;
        Ok(true)
    }
}
