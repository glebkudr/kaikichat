//! Staged record persistence over the existing OpenMLS memory provider.
use super::*;
use std::collections::BTreeMap;
use std::sync::{Arc, OnceLock};

// Secrets remain non-Debug and non-Serialize. Unchanged encoded records are
// shared across staged states; only explicit compatibility reads assemble CBOR.
pub struct SecretState {
    identity: Vec<u8>,
    signer_public: Vec<u8>,
    prefix: Zeroizing<Vec<u8>>,
    records: BTreeMap<Vec<u8>, Arc<Zeroizing<Vec<u8>>>>,
    serialized: OnceLock<Zeroizing<Vec<u8>>>,
    /// Records of the whole profile's state, as its prefix counts them.
    count: u64,
    scope: Scope,
}

/// What of its profile's records a state holds.
#[derive(Clone, Copy, PartialEq, Eq)]
enum Scope {
    /// All of them.
    Whole,
    /// The profile's own records alone.
    Own,
    /// The profile's own and one group's.
    Group([u8; 32]),
}

/// Labels of openmls_memory_storage 0.6 whose records are the profile's own,
/// whatever group: its signature key, key packages, leaf encryption keys and
/// PSKs.
const OWN_LABELS: [&[u8]; 8] = [
    b"KeyPackage",
    b"Psk",
    b"EncryptionKeyPair",
    b"SignatureKeyPair",
    b"RetainedKeyPackageMaterial",
    b"RetainedKeyPackageEpoch",
    b"VcEmulationEpochState",
    b"VcOperationTree",
];
/// Labels whose records are one group's, keyed by the group's id first.
const GROUP_LABELS: [&[u8]; 16] = [
    b"Tree",
    b"GroupContext",
    b"InterimTranscriptHash",
    b"ConfirmationTag",
    b"MlsGroupJoinConfig",
    b"OwnLeafNodes",
    b"GroupState",
    b"ProposalQueueRefs",
    b"OwnLeafNodeIndex",
    b"EpochSecrets",
    b"ResumptionPsk",
    b"MessageSecrets",
    b"EpochKeyPairs",
    b"ApplicationExportTree",
    b"VcEmulationBinding",
    b"RegisteredVcEmulationEpoch",
];
/// A group's queued proposals, keyed by the pair of its id and theirs.
const QUEUED_PROPOSAL: &[u8] = b"QueuedProposal";

/// The storage keys an operation reads and writes, as key prefixes: the
/// profile's own records and, with a group, that group's
/// (Docs/V1_LARGE_GROUPS_CHANNELS_2026_09_28.md, part 1, phase 1b).
pub fn record_scope(group: Option<[u8; 32]>) -> Vec<Vec<u8>> {
    let mut scope: Vec<Vec<u8>> = OWN_LABELS.iter().map(|label| label.to_vec()).collect();
    if let Some(id) = group {
        let id = group_id_json(&id);
        scope.extend(
            GROUP_LABELS
                .iter()
                .map(|label| [label, id.as_slice()].concat()),
        );
        scope.push([QUEUED_PROPOSAL, b"[", id.as_slice()].concat());
    }
    scope
}

/// A group id as openmls keys its records: the JSON of its `GroupId`.
fn group_id_json(id: &[u8; 32]) -> Vec<u8> {
    let bytes: Vec<String> = id.iter().map(u8::to_string).collect();
    format!("{{\"value\":{{\"vec\":[{}]}}}}", bytes.join(",")).into_bytes()
}

/// Drop what a deleted group left behind: openmls keeps its epoch key
/// pairs.
pub(super) fn drop_group_records(engine: &Engine, id: [u8; 32]) -> Result<(), CryptoError> {
    let own = record_scope(None);
    let prefixes: Vec<Vec<u8>> = record_scope(Some(id))
        .into_iter()
        .filter(|prefix| !own.contains(prefix))
        .collect();
    engine
        .provider
        .storage()
        .values
        .write()
        .map_err(state_error)?
        .retain(|key, _| !prefixes.iter().any(|prefix| key.starts_with(prefix)));
    Ok(())
}

impl Scope {
    fn prefixes(self) -> Option<Vec<Vec<u8>>> {
        match self {
            Self::Whole => None,
            Self::Own => Some(record_scope(None)),
            Self::Group(id) => Some(record_scope(Some(id))),
        }
    }
}

/// Every key within `scope`, or a state that holds everything.
fn within(
    scope: Scope,
    mut keys: impl Iterator<Item = impl AsRef<[u8]>>,
) -> Result<(), CryptoError> {
    let Some(prefixes) = scope.prefixes() else {
        return Ok(());
    };
    if keys.all(|key| {
        prefixes
            .iter()
            .any(|prefix| key.as_ref().starts_with(prefix))
    }) {
        Ok(())
    } else {
        Err(CryptoError::OutOfScope)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct PersistenceStats {
    pub total_records: usize,
    pub changed_records: usize,
    pub deleted_records: usize,
    pub snapshot_bytes: usize,
    pub written_bytes: usize,
}

fn record_value<'a>(key: &[u8], bytes: &'a [u8]) -> Result<&'a [u8], CryptoError> {
    let mut decoder = Decoder::new(bytes);
    if decoder.array().map_err(state_error)? != Some(2)
        || decoder.bytes().map_err(state_error)? != key
    {
        return Err(CryptoError::InvalidState);
    }
    let value = decoder.bytes().map_err(state_error)?;
    if decoder.position() != bytes.len() {
        return Err(CryptoError::InvalidState);
    }
    Ok(value)
}

impl SecretState {
    /// Refuse an operation on a group this state did not load.
    pub(super) fn check_scope(&self, id: [u8; 32]) -> Result<(), CryptoError> {
        match self.scope {
            Scope::Whole => Ok(()),
            Scope::Group(loaded) if loaded == id => Ok(()),
            _ => Err(CryptoError::OutOfScope),
        }
    }
    /// The whole snapshot; a state that loaded part of its profile's
    /// records gives them alone, which no restore takes for the whole.
    pub fn as_bytes(&self) -> &[u8] {
        self.serialized
            .get_or_init(|| {
                let mut bytes = Zeroizing::new(Vec::with_capacity(self.snapshot_size()));
                bytes.extend_from_slice(&self.prefix);
                for record in self.records.values() {
                    bytes.extend_from_slice(record);
                }
                bytes
            })
            .as_slice()
    }
    pub fn record_prefix(&self) -> &[u8] {
        &self.prefix
    }
    fn snapshot_size(&self) -> usize {
        self.prefix.len()
            + self
                .records
                .values()
                .map(|record| record.len())
                .sum::<usize>()
    }
    /// Encoded record replacements and actual deletions; no provider-wide dump.
    pub fn record_changes<'a>(
        &'a self,
        previous: Option<&'a Self>,
    ) -> impl Iterator<Item = (&'a [u8], Option<&'a [u8]>)> + 'a {
        let writes = self
            .records
            .iter()
            .filter(move |(key, value)| {
                previous.is_none_or(|old| {
                    old.records.get(*key).is_none_or(|before| {
                        !Arc::ptr_eq(before, value) && before.as_slice() != value.as_slice()
                    })
                })
            })
            .map(|(key, value)| (key.as_slice(), Some(value.as_slice())));
        let deletes = previous
            .into_iter()
            .flat_map(|old| old.records.keys())
            .filter(move |key| !self.records.contains_key(*key))
            .map(|key| (key.as_slice(), None));
        writes.chain(deletes)
    }
    /// Payload/key byte counts for profiling without exposing any secret values.
    pub fn persistence_stats(&self, previous: Option<&Self>) -> PersistenceStats {
        let mut stats = PersistenceStats {
            total_records: self.records.len(),
            changed_records: 0,
            deleted_records: 0,
            snapshot_bytes: self.snapshot_size(),
            written_bytes: self.prefix.len(),
        };
        for (key, value) in self.record_changes(previous) {
            stats.changed_records += 1;
            stats.deleted_records += usize::from(value.is_none());
            stats.written_bytes += key.len() + value.map_or(0, <[u8]>::len);
        }
        stats
    }
    pub(super) fn engine(&self) -> Result<Engine, CryptoError> {
        // ponytail: the upstream memory provider still loads all records for an
        // isolated operation; use a lazy provider if read/copy cost dominates.
        let engine = Engine {
            provider: OpenMlsRustCrypto::default(),
            identity: self.identity.clone(),
            signer_public: self.signer_public.clone(),
        };
        {
            let mut values = engine
                .provider
                .storage()
                .values
                .write()
                .map_err(state_error)?;
            for (key, bytes) in &self.records {
                values.insert(key.clone(), record_value(key, bytes)?.to_vec());
            }
        }
        Ok(engine)
    }
    pub(super) fn from_engine(
        engine: Engine,
        previous: Option<&Self>,
    ) -> Result<Self, CryptoError> {
        let mut values = engine
            .provider
            .storage()
            .values
            .write()
            .map_err(state_error)?;
        if values.len() > 100_000 {
            return Err(CryptoError::TooLarge);
        }
        let size = values
            .iter()
            .try_fold(1024usize, |size, (key, value)| {
                size.checked_add(key.len())?
                    .checked_add(value.len())?
                    .checked_add(16)
            })
            .ok_or(CryptoError::TooLarge)?;
        if size > MAX_STATE || values.keys().any(|key| key.is_empty() || key.len() > 4096) {
            return Err(CryptoError::TooLarge);
        }
        // A state of part of the profile writes within that part, and
        // counts the whole: what it had, less what went, plus what came.
        let scope = previous.map_or(Scope::Whole, |old| old.scope);
        within(scope, values.keys())?;
        let count = match previous {
            Some(old) if old.scope != Scope::Whole => {
                let gone = old
                    .records
                    .keys()
                    .filter(|key| !values.contains_key(*key))
                    .count();
                let came = values
                    .keys()
                    .filter(|key| !old.records.contains_key(*key))
                    .count();
                old.count
                    .checked_sub(gone as u64)
                    .and_then(|count| count.checked_add(came as u64))
                    .ok_or(CryptoError::InvalidState)?
            }
            _ => values.len() as u64,
        };
        if count > 100_000 {
            return Err(CryptoError::TooLarge);
        }
        let mut prefix = Zeroizing::new(Vec::new());
        Encoder::new(&mut *prefix)
            .array(4)
            .map_err(state_error)?
            .u8(1)
            .map_err(state_error)?
            .bytes(&engine.identity)
            .map_err(state_error)?
            .bytes(&engine.signer_public)
            .map_err(state_error)?
            .array(count)
            .map_err(state_error)?;
        let mut records = BTreeMap::new();
        for (key, mut value) in values.drain() {
            let old = previous.and_then(|old| old.records.get(&key));
            let record = if let Some(old) = old
                && record_value(&key, old)? == value
            {
                Arc::clone(old)
            } else {
                let mut bytes = Zeroizing::new(Vec::new());
                Encoder::new(&mut *bytes)
                    .array(2)
                    .map_err(state_error)?
                    .bytes(&key)
                    .map_err(state_error)?
                    .bytes(&value)
                    .map_err(state_error)?;
                Arc::new(bytes)
            };
            value.zeroize();
            records.insert(key, record);
        }
        Ok(Self {
            identity: engine.identity.clone(),
            signer_public: engine.signer_public.clone(),
            prefix,
            records,
            serialized: OnceLock::new(),
            count,
            scope,
        })
    }
    /// The whole state from all its records.
    pub(super) fn restore_records(
        prefix: &[u8],
        records: impl Iterator<Item = (Vec<u8>, Vec<u8>)>,
    ) -> Result<Self, CryptoError> {
        Self::restore_part(prefix, records, Scope::Whole)
    }
    /// The profile's own records and, with `group`, that group's: what one
    /// operation needs.
    pub(super) fn restore_scope(
        prefix: &[u8],
        records: impl Iterator<Item = (Vec<u8>, Vec<u8>)>,
        group: Option<[u8; 32]>,
    ) -> Result<Self, CryptoError> {
        Self::restore_part(prefix, records, group.map_or(Scope::Own, Scope::Group))
    }
    fn restore_part(
        prefix: &[u8],
        records: impl Iterator<Item = (Vec<u8>, Vec<u8>)>,
        scope: Scope,
    ) -> Result<Self, CryptoError> {
        check_size(prefix, 1024)?;
        let mut decoder = Decoder::new(prefix);
        if decoder.array().map_err(state_error)? != Some(4)
            || decoder.u8().map_err(state_error)? != 1
        {
            return Err(CryptoError::InvalidState);
        }
        let identity = decoder.bytes().map_err(state_error)?.to_vec();
        let signer_public = decoder.bytes().map_err(state_error)?.to_vec();
        let count = decoder
            .array()
            .map_err(state_error)?
            .ok_or(CryptoError::InvalidState)?;
        if identity.is_empty()
            || identity.len() > 256
            || signer_public.len() != 32
            || count > 100_000
            || decoder.position() != prefix.len()
        {
            return Err(CryptoError::InvalidState);
        }
        let mut state = Self {
            identity,
            signer_public,
            prefix: Zeroizing::new(prefix.to_vec()),
            records: BTreeMap::new(),
            serialized: OnceLock::new(),
            count,
            scope,
        };
        let prefixes = scope.prefixes();
        let mut size = prefix.len();
        for (key, bytes) in records {
            let bytes = Zeroizing::new(bytes);
            size = size.checked_add(bytes.len()).ok_or(CryptoError::TooLarge)?;
            if size > MAX_STATE || state.records.len() as u64 >= count {
                return Err(CryptoError::TooLarge);
            }
            check_size(&key, 4096)?;
            if prefixes
                .as_ref()
                .is_some_and(|prefixes| !prefixes.iter().any(|prefix| key.starts_with(prefix)))
            {
                return Err(CryptoError::OutOfScope);
            }
            record_value(&key, &bytes)?;
            if state.records.insert(key, Arc::new(bytes)).is_some() {
                return Err(CryptoError::InvalidState);
            }
        }
        if scope == Scope::Whole && state.records.len() as u64 != count {
            return Err(CryptoError::InvalidState);
        }
        Ok(state)
    }
}

impl Drop for SecretState {
    fn drop(&mut self) {
        for (mut key, _) in std::mem::take(&mut self.records) {
            key.zeroize();
        }
    }
}
