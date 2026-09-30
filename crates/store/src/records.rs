//! Incremental state fragments guarded by their parent's existing revision CAS.
use super::*;
use std::collections::BTreeMap;

const MAX_RECORDS: usize = 100_000;
const MAX_BYTES: usize = 32 * 1024 * 1024;
pub(super) const TABLES: &str = "CREATE TABLE IF NOT EXISTS state_records (
         namespace TEXT NOT NULL REFERENCES states(namespace) ON DELETE CASCADE,
         record_key BLOB NOT NULL CHECK(length(record_key)>0 AND length(record_key)<=4096),
         bytes BLOB NOT NULL CHECK(length(bytes)>0 AND length(bytes)<=33554432),
         PRIMARY KEY(namespace,record_key)
     );";

/// The logical state is its prefix followed by record bytes in binary key order.
/// No other writer or transaction is introduced by this representation.
pub struct StateRecords {
    pub state: StateValue,
    pub records: Vec<(Vec<u8>, Vec<u8>)>,
}
pub struct StateRecordChange {
    pub key: Vec<u8>,
    pub bytes: Option<Vec<u8>>,
}
impl Drop for StateRecordChange {
    fn drop(&mut self) {
        self.key.zeroize();
        if let Some(bytes) = &mut self.bytes {
            bytes.zeroize();
        }
    }
}
pub struct StateRecordBatch {
    pub namespace: String,
    pub changes: Vec<StateRecordChange>,
}

impl ProfileStore {
    /// Read structured state without assembling a second whole-state buffer.
    /// An ordinary inline state has no record fragments.
    pub fn state_records(&self, namespace: &str) -> Result<Option<StateRecords>, StoreError> {
        identifier(namespace)?;
        let saved = self.connection.query_row(
            "SELECT revision,bytes,EXISTS(SELECT 1 FROM state_records r WHERE r.namespace=states.namespace) FROM states WHERE namespace=?1",
            [namespace],
            |row| Ok((StateValue { revision: read_u64(row, 0)?, bytes: row.get(1)? }, row.get::<_, bool>(2)?)),
        ).optional()?;
        let Some((state, has_records)) = saved else {
            return Ok(None);
        };
        let mut size = state.bytes.len();
        if size > MAX_BYTES {
            return Err(StoreError::InvalidProfile);
        }
        let mut records = Vec::new();
        if has_records {
            let mut query = self.connection.prepare(
                "SELECT record_key,bytes FROM state_records WHERE namespace=?1 ORDER BY record_key LIMIT ?2",
            )?;
            let mut rows = query.query(params![namespace, (MAX_RECORDS + 1) as i64])?;
            while let Some(row) = rows.next()? {
                let key: Vec<u8> = row.get(0)?;
                let bytes: Vec<u8> = row.get(1)?;
                size = size
                    .checked_add(bytes.len())
                    .ok_or(StoreError::InvalidProfile)?;
                if key.is_empty()
                    || key.len() > 4096
                    || bytes.is_empty()
                    || size > MAX_BYTES
                    || records.len() == MAX_RECORDS
                {
                    return Err(StoreError::InvalidProfile);
                }
                records.push((key, bytes));
            }
        }
        Ok(Some(StateRecords { state, records }))
    }

    /// Read a structured state's parent and those of its records whose key
    /// begins with one of `prefixes`, in key order, each once: what one
    /// operation needs of it. The byte bound holds for what is read.
    pub fn state_records_in(
        &self,
        namespace: &str,
        prefixes: &[Vec<u8>],
    ) -> Result<Option<StateRecords>, StoreError> {
        identifier(namespace)?;
        if prefixes.is_empty()
            || prefixes.len() > 64
            || prefixes
                .iter()
                .any(|prefix| prefix.is_empty() || prefix.len() > 4096)
        {
            return Err(StoreError::InvalidInput);
        }
        let saved = self
            .connection
            .query_row(
                "SELECT revision,bytes FROM states WHERE namespace=?1",
                [namespace],
                |row| {
                    Ok(StateValue {
                        revision: read_u64(row, 0)?,
                        bytes: row.get(1)?,
                    })
                },
            )
            .optional()?;
        let Some(state) = saved else {
            return Ok(None);
        };
        let mut size = state.bytes.len();
        if size > MAX_BYTES {
            return Err(StoreError::InvalidProfile);
        }
        let mut records = BTreeMap::new();
        let mut query = self.connection.prepare_cached(
            "SELECT record_key,bytes FROM state_records WHERE namespace=?1 AND record_key>=?2 AND (?3 IS NULL OR record_key<?3) ORDER BY record_key LIMIT ?4",
        )?;
        for prefix in prefixes {
            let end = following(prefix);
            let mut rows =
                query.query(params![namespace, prefix, end, (MAX_RECORDS + 1) as i64])?;
            while let Some(row) = rows.next()? {
                let key: Vec<u8> = row.get(0)?;
                if records.contains_key(&key) {
                    continue;
                }
                let bytes: Vec<u8> = row.get(1)?;
                size = size
                    .checked_add(bytes.len())
                    .ok_or(StoreError::InvalidProfile)?;
                if key.is_empty()
                    || key.len() > 4096
                    || bytes.is_empty()
                    || size > MAX_BYTES
                    || records.len() == MAX_RECORDS
                {
                    return Err(StoreError::InvalidProfile);
                }
                records.insert(key, bytes);
            }
        }
        Ok(Some(StateRecords {
            state,
            records: records.into_iter().collect(),
        }))
    }
}

/// The least key after every key that begins with `prefix`, if any.
fn following(prefix: &[u8]) -> Option<Vec<u8>> {
    let mut end = prefix.to_vec();
    while let Some(last) = end.pop() {
        if last < u8::MAX {
            end.push(last + 1);
            return Some(end);
        }
    }
    None
}

pub(super) fn validate(
    states: &[StateChange],
    batches: &[StateRecordBatch],
) -> Result<(), StoreError> {
    let mut namespaces = HashSet::new();
    for batch in batches {
        let parent = states
            .iter()
            .find(|state| state.namespace == batch.namespace)
            .ok_or(StoreError::InvalidInput)?;
        if !namespaces.insert(&batch.namespace) || batch.changes.len() > MAX_RECORDS {
            return Err(StoreError::InvalidInput);
        }
        let mut size = parent.bytes.len();
        let mut keys = HashSet::new();
        for change in &batch.changes {
            size = size
                .checked_add(change.key.len())
                .and_then(|size| size.checked_add(change.bytes.as_ref().map_or(0, Vec::len)))
                .ok_or(StoreError::InvalidInput)?;
            if change.key.is_empty()
                || change.key.len() > 4096
                || !keys.insert(&change.key)
                || change.bytes.as_ref().is_some_and(Vec::is_empty)
                || size > MAX_BYTES
            {
                return Err(StoreError::InvalidInput);
            }
        }
    }
    Ok(())
}

pub(super) fn apply(connection: &Connection, batch: &StateRecordBatch) -> Result<(), StoreError> {
    let mut put = connection.prepare_cached(
        "INSERT INTO state_records(namespace,record_key,bytes) VALUES(?1,?2,?3) ON CONFLICT(namespace,record_key) DO UPDATE SET bytes=excluded.bytes WHERE state_records.bytes<>excluded.bytes",
    )?;
    let mut remove = connection
        .prepare_cached("DELETE FROM state_records WHERE namespace=?1 AND record_key=?2")?;
    for change in &batch.changes {
        match &change.bytes {
            Some(bytes) => {
                put.execute(params![batch.namespace, change.key, bytes])?;
            }
            None => {
                remove.execute(params![batch.namespace, change.key])?;
            }
        }
    }
    Ok(())
}
