//! A closed channel's key tree (Docs/V1_LARGE_GROUPS_CHANNELS_2026_09_28.md,
//! part 10c): a binary tree of depth 32 whose node keys derive from the
//! team's seed and each node's version, the number of removed leaves under
//! it. A subscriber holds the keys of its leaf's path; the root's is the
//! channel key.
use super::*;
use chacha20poly1305::aead::{Aead, KeyInit, Payload};
use chacha20poly1305::{ChaCha20Poly1305, Nonce};

pub(super) const DEPTH: u32 = 32;

/// A node: its level (0 the root, 32 the leaves) and its index there.
pub(super) fn node(level: u32, index: u64) -> u64 {
    (u64::from(level) << 32) | index
}

pub(super) fn level_of(node: u64) -> u32 {
    (node >> 32) as u32
}

/// The node at `level` above `leaf`.
pub(super) fn ancestor(leaf: u32, level: u32) -> u64 {
    node(level, u64::from(leaf) >> (DEPTH - level))
}

/// The nodes from the root down to `leaf`.
pub(super) fn path(leaf: u32) -> Vec<u64> {
    (0..=DEPTH).map(|level| ancestor(leaf, level)).collect()
}

pub(super) fn sibling(of: u64) -> u64 {
    of ^ 1
}

pub(super) const ROOT: u64 = 0;

/// How many of `removed` lie under `node`.
pub(super) fn version(of: u64, removed: &[u32]) -> u32 {
    let level = level_of(of);
    removed
        .iter()
        .filter(|leaf| ancestor(**leaf, level) == of)
        .count() as u32
}

/// The key of `node` at `version` in `generation` of channel `group`.
pub(super) fn node_key(
    seed: &[u8; 32],
    group: &[u8; 32],
    generation: u32,
    of: u64,
    version: u32,
) -> [u8; 32] {
    let mut info = Vec::with_capacity(48);
    info.extend_from_slice(group);
    info.extend_from_slice(&generation.to_be_bytes());
    info.extend_from_slice(&of.to_be_bytes());
    info.extend_from_slice(&version.to_be_bytes());
    let mut key = [0; 32];
    let _ = hkdf::Hkdf::<Sha256>::new(Some(b"AIN_CHANNEL_KEY_V1"), seed).expand(&info, &mut key);
    key
}

/// A key derived from the channel key for `label`.
pub(super) fn derived(root: &[u8; 32], group: &[u8; 32], label: &[u8]) -> [u8; 32] {
    let mut info = label.to_vec();
    info.extend_from_slice(group);
    let mut key = [0; 32];
    let _ = hkdf::Hkdf::<Sha256>::new(Some(b"AIN_CHANNEL_V1"), root).expand(&info, &mut key);
    key
}

/// What a sealed key is bound to: the key it is (node, generation,
/// version) and the key it is sealed under.
fn binding(group: &[u8; 32], generation: u32, target: u64, version: u32, under: u64) -> Vec<u8> {
    let mut aad = b"AIN_CHANNEL_WRAP_V1".to_vec();
    aad.extend_from_slice(group);
    aad.extend_from_slice(&generation.to_be_bytes());
    aad.extend_from_slice(&target.to_be_bytes());
    aad.extend_from_slice(&version.to_be_bytes());
    aad.extend_from_slice(&under.to_be_bytes());
    aad
}

/// The key of `target` at `version` of `generation`, sealed under the key
/// `wrapping` of node `under`. The nonce comes from what the key is: a key
/// is sealed under another once.
pub(super) fn wrap(
    wrapping: &[u8; 32],
    group: &[u8; 32],
    generation: u32,
    target: u64,
    version: u32,
    under: u64,
    key: &[u8; 32],
) -> Result<Vec<u8>, CoreError> {
    let aad = binding(group, generation, target, version, under);
    let nonce: [u8; 32] = Sha256::digest(&aad).into();
    ChaCha20Poly1305::new_from_slice(wrapping)
        .map_err(|_| CoreError::InvalidState)?
        .encrypt(
            Nonce::from_slice(&nonce[..12]),
            Payload {
                msg: key,
                aad: &aad,
            },
        )
        .map_err(|_| CoreError::InvalidState)
}

pub(super) fn unwrap(
    wrapping: &[u8; 32],
    group: &[u8; 32],
    generation: u32,
    target: u64,
    version: u32,
    under: u64,
    sealed: &[u8],
) -> Option<[u8; 32]> {
    let aad = binding(group, generation, target, version, under);
    let nonce: [u8; 32] = Sha256::digest(&aad).into();
    ChaCha20Poly1305::new_from_slice(wrapping)
        .ok()?
        .decrypt(
            Nonce::from_slice(&nonce[..12]),
            Payload {
                msg: sealed,
                aad: &aad,
            },
        )
        .ok()?
        .try_into()
        .ok()
}

/// A removal's sealed keys, in order from the leaf up: at each level the
/// parent's new key under the key beside the removed path, then (above the
/// leaf) under the new key of the path's node below. `removed` already
/// holds the leaf.
pub(super) fn rekey(
    seed: &[u8; 32],
    group: &[u8; 32],
    generation: u32,
    removed: &[u32],
    leaf: u32,
) -> Result<Vec<Vec<u8>>, CoreError> {
    let key = |of: u64| node_key(seed, group, generation, of, version(of, removed));
    let mut sealed = vec![];
    for level in (1..=DEPTH).rev() {
        let parent = ancestor(leaf, level - 1);
        let new = key(parent);
        let target = version(parent, removed);
        let below = ancestor(leaf, level);
        let beside = sibling(below);
        sealed.push(wrap(
            &key(beside),
            group,
            generation,
            parent,
            target,
            beside,
            &new,
        )?);
        if level < DEPTH {
            sealed.push(wrap(
                &key(below),
                group,
                generation,
                parent,
                target,
                below,
                &new,
            )?);
        }
    }
    Ok(sealed)
}

/// A subscriber's keys: its path's nodes, each at its version.
pub(super) type PathKeys = BTreeMap<u64, (u32, [u8; 32])>;

/// Apply a removal of `leaf` to the path keys of `own`: the new keys of
/// the ancestors the two share. `None` when a sealed key does not open —
/// the removed leaf itself, or keys not this subscriber's.
pub(super) fn apply_rekey(
    keys: &PathKeys,
    own: u32,
    group: &[u8; 32],
    generation: u32,
    leaf: u32,
    sealed: &[Vec<u8>],
) -> Option<PathKeys> {
    if own == leaf || sealed.len() != 2 * DEPTH as usize - 1 {
        return None;
    }
    let mut next = keys.clone();
    let mut entries = sealed.iter();
    for level in (1..=DEPTH).rev() {
        let parent = ancestor(leaf, level - 1);
        let beside_entry = entries.next()?;
        let below_entry = if level < DEPTH { entries.next() } else { None };
        if ancestor(own, level - 1) != parent {
            continue;
        }
        let below = ancestor(leaf, level);
        let mine = ancestor(own, level);
        let (version, _) = *next.get(&parent)?;
        let target = version.checked_add(1)?;
        let (entry, under) = if mine == below {
            (below_entry?, below)
        } else {
            (beside_entry, mine)
        };
        let (_, wrapping) = next.get(&under)?;
        let key = unwrap(wrapping, group, generation, parent, target, under, entry)?;
        next.insert(parent, (target, key));
    }
    Some(next)
}

/// A reseed's sealed keys: for each node of `occupied` leaves' paths, the
/// key of `next` generation (version 0) under its key now.
pub(super) fn reseed(
    seed: &[u8; 32],
    next_seed: &[u8; 32],
    group: &[u8; 32],
    generation: u32,
    removed: &[u32],
    occupied: &[u32],
) -> Result<Vec<(u64, Vec<u8>)>, CoreError> {
    let mut nodes: Vec<u64> = occupied.iter().flat_map(|leaf| path(*leaf)).collect();
    nodes.sort_by_key(|of| (level_of(*of), *of));
    nodes.dedup();
    nodes
        .into_iter()
        .map(|of| {
            let old = node_key(seed, group, generation, of, version(of, removed));
            let new = node_key(next_seed, group, generation + 1, of, 0);
            Ok((of, wrap(&old, group, generation + 1, of, 0, of, &new)?))
        })
        .collect()
}

/// The next generation's keys of `own`'s path found in a reseed's
/// entries, added to `staged`.
pub(super) fn apply_reseed(
    keys: &PathKeys,
    staged: &mut PathKeys,
    group: &[u8; 32],
    generation: u32,
    entries: &[(u64, Vec<u8>)],
) {
    for (of, sealed) in entries {
        let Some((_, old)) = keys.get(of) else {
            continue;
        };
        if let Some(key) = unwrap(old, group, generation + 1, *of, 0, *of, sealed) {
            staged.insert(*of, (0, key));
        }
    }
}

/// What a leaf's key sealed to its subscriber's own key is bound to.
fn personal_binding(group: &[u8; 32], generation: u32, leaf: u32) -> Vec<u8> {
    let mut aad = b"AIN_CHANNEL_PERSONAL_V1".to_vec();
    aad.extend_from_slice(group);
    aad.extend_from_slice(&generation.to_be_bytes());
    aad.extend_from_slice(&leaf.to_be_bytes());
    aad
}

/// The key a leaf's new key is sealed under for its subscriber: from the
/// Diffie-Hellman of the reseed's ephemeral key and the subscriber's.
fn personal_cipher(
    shared: x25519_dalek::SharedSecret,
    ephemeral: &[u8; 32],
    subscriber: &[u8; 32],
    binding: &[u8],
) -> Option<ChaCha20Poly1305> {
    if !shared.was_contributory() {
        return None;
    }
    let mut info = binding.to_vec();
    info.extend_from_slice(ephemeral);
    info.extend_from_slice(subscriber);
    let mut key = [0; 32];
    hkdf::Hkdf::<Sha256>::new(Some(b"AIN_CHANNEL_PERSONAL_V1"), shared.as_bytes())
        .expand(&info, &mut key)
        .ok()?;
    ChaCha20Poly1305::new_from_slice(&key).ok()
}

/// `key`, the new key of `leaf` in `generation`, sealed to its subscriber's
/// key `subscriber` with the reseed's ephemeral secret.
fn seal_personal(
    ephemeral: &[u8; 32],
    subscriber: &[u8; 32],
    group: &[u8; 32],
    generation: u32,
    leaf: u32,
    key: &[u8; 32],
) -> Result<Vec<u8>, CoreError> {
    let secret = x25519_dalek::StaticSecret::from(*ephemeral);
    let public = x25519_dalek::PublicKey::from(&secret).to_bytes();
    let binding = personal_binding(group, generation, leaf);
    // Each leaf's key is sealed under a key of its own, once.
    personal_cipher(
        secret.diffie_hellman(&x25519_dalek::PublicKey::from(*subscriber)),
        &public,
        subscriber,
        &binding,
    )
    .ok_or(CoreError::InvalidInput)?
    .encrypt(
        Nonce::from_slice(&[0; 12]),
        Payload {
            msg: key,
            aad: &binding,
        },
    )
    .map_err(|_| CoreError::InvalidState)
}

fn open_personal(
    personal: &[u8; 32],
    ephemeral: &[u8; 32],
    group: &[u8; 32],
    generation: u32,
    leaf: u32,
    sealed: &[u8],
) -> Option<[u8; 32]> {
    let secret = x25519_dalek::StaticSecret::from(*personal);
    let public = x25519_dalek::PublicKey::from(&secret).to_bytes();
    let binding = personal_binding(group, generation, leaf);
    personal_cipher(
        secret.diffie_hellman(&x25519_dalek::PublicKey::from(*ephemeral)),
        ephemeral,
        &public,
        &binding,
    )?
    .decrypt(
        Nonce::from_slice(&[0; 12]),
        Payload {
            msg: sealed,
            aad: &binding,
        },
    )
    .ok()?
    .try_into()
    .ok()
}

/// Whether `public` is a key one can seal to: not a point of small order,
/// whose shared secret is the same for everyone.
pub(super) fn is_key(public: &[u8; 32]) -> bool {
    x25519_dalek::StaticSecret::from([7; 32])
        .diffie_hellman(&x25519_dalek::PublicKey::from(*public))
        .was_contributory()
}

fn hello_binding(group: &[u8; 32], generation: u32, leaf: u32) -> Vec<u8> {
    let mut aad = b"AIN_CHANNEL_HELLO_V1".to_vec();
    aad.extend_from_slice(group);
    aad.extend_from_slice(&generation.to_be_bytes());
    aad.extend_from_slice(&leaf.to_be_bytes());
    aad
}

/// A subscriber's key document as it goes in the channel's mailbox, before
/// the channel key seals it: `0x06 ‖ leaf ‖ generation ‖ nonce ‖ AEAD`,
/// under a key from the leaf's key of `generation`, so that only the team
/// and the leaf's holder read who signed it.
pub(super) fn seal_hello(
    leaf_key: &[u8; 32],
    group: &[u8; 32],
    generation: u32,
    leaf: u32,
    doc: &[u8],
) -> Result<Vec<u8>, CoreError> {
    let key = derived(leaf_key, group, b"hello");
    let mut hash = Sha256::new();
    hash.update(key);
    hash.update(doc);
    let digest: [u8; 32] = hash.finalize().into();
    let sealed = ChaCha20Poly1305::new_from_slice(&key)
        .map_err(|_| CoreError::InvalidState)?
        .encrypt(
            Nonce::from_slice(&digest[..12]),
            Payload {
                msg: doc,
                aad: &hello_binding(group, generation, leaf),
            },
        )
        .map_err(|_| CoreError::InvalidState)?;
    Ok([
        &[SUBSCRIBER_KEY][..],
        &leaf.to_be_bytes(),
        &generation.to_be_bytes(),
        &digest[..12],
        &sealed,
    ]
    .concat())
}

/// What a subscriber's key entry says of itself: its leaf and generation.
pub(super) fn hello_of(inner: &[u8]) -> Option<(u32, u32)> {
    if inner.len() < 1 + 4 + 4 + 12 + 16 || inner[0] != SUBSCRIBER_KEY {
        return None;
    }
    Some((
        u32::from_be_bytes(inner[1..5].try_into().ok()?),
        u32::from_be_bytes(inner[5..9].try_into().ok()?),
    ))
}

/// The document inside a subscriber's key entry, opened with its leaf's key.
pub(super) fn open_hello(leaf_key: &[u8; 32], group: &[u8; 32], inner: &[u8]) -> Option<Vec<u8>> {
    let (leaf, generation) = hello_of(inner)?;
    let key = derived(leaf_key, group, b"hello");
    ChaCha20Poly1305::new_from_slice(&key)
        .ok()?
        .decrypt(
            Nonce::from_slice(&inner[9..21]),
            Payload {
                msg: &inner[21..],
                aad: &hello_binding(group, generation, leaf),
            },
        )
        .ok()
}

/// A reseed onto subscribers' own keys: the keys of `generation` (version
/// 0, from `next_seed`) of every node on `occupied` leaves' paths — each
/// leaf's sealed to its subscriber's key in `personal` with the
/// `ephemeral` secret, each node above under the new keys of its occupied
/// children. Nothing is sealed under a key of the generation before: whoever
/// knew its seed opens none of it. Leaves first, then the nodes from the
/// deepest up to the root.
pub(super) fn hard_reseed(
    next_seed: &[u8; 32],
    group: &[u8; 32],
    generation: u32,
    occupied: &[u32],
    personal: &BTreeMap<u32, [u8; 32]>,
    ephemeral: &[u8; 32],
) -> Result<Vec<(u64, Vec<u8>)>, CoreError> {
    let new = |of: u64| node_key(next_seed, group, generation, of, 0);
    let mut leaves = occupied.to_vec();
    leaves.sort_unstable();
    leaves.dedup();
    let mut entries = vec![];
    for leaf in &leaves {
        let of = ancestor(*leaf, DEPTH);
        // A key one cannot seal to is passed over: that leaf gets its keys
        // again.
        if let Some(sealed) = personal.get(leaf).and_then(|subscriber| {
            seal_personal(ephemeral, subscriber, group, generation, *leaf, &new(of)).ok()
        }) {
            entries.push((of, sealed));
        }
    }
    let on_paths: std::collections::BTreeSet<u64> =
        leaves.iter().flat_map(|leaf| path(*leaf)).collect();
    let mut inner: Vec<u64> = on_paths
        .iter()
        .copied()
        .filter(|of| level_of(*of) < DEPTH)
        .collect();
    inner.sort_by_key(|of| (std::cmp::Reverse(level_of(*of)), *of));
    for of in inner {
        let first = node(level_of(of) + 1, (of & 0xffff_ffff) << 1);
        for child in [first, sibling(first)] {
            if on_paths.contains(&child) {
                entries.push((
                    of,
                    wrap(&new(child), group, generation, of, 0, child, &new(of))?,
                ));
            }
        }
    }
    Ok(entries)
}

/// The keys of `own`'s path a reseed onto subscribers' keys gives, from the
/// `entries` found so far, added to `staged`: its leaf's opened with its
/// own key `personal`, the nodes above under the keys found below them.
/// Entries of the path not openable yet wait in `pending`.
#[allow(clippy::too_many_arguments)]
pub(super) fn apply_hard_reseed(
    personal: &[u8; 32],
    ephemeral: &[u8; 32],
    own: u32,
    group: &[u8; 32],
    generation: u32,
    entries: &[(u64, Vec<u8>)],
    staged: &mut PathKeys,
    pending: &mut BTreeMap<u64, Vec<Vec<u8>>>,
) {
    let leaf = ancestor(own, DEPTH);
    for (of, sealed) in entries {
        if *of == leaf {
            if let Some(key) = open_personal(personal, ephemeral, group, generation, own, sealed) {
                staged.insert(leaf, (0, key));
            }
        } else if level_of(*of) < DEPTH
            && ancestor(own, level_of(*of)) == *of
            && !staged.contains_key(of)
        {
            let waiting = pending.entry(*of).or_default();
            // A node has two children: at most two entries each.
            if !waiting.contains(sealed) && waiting.len() < 2 {
                waiting.push(sealed.clone());
            }
        }
    }
    for level in (0..DEPTH).rev() {
        let of = ancestor(own, level);
        if staged.contains_key(&of) {
            continue;
        }
        let child = ancestor(own, level + 1);
        let Some((_, below)) = staged.get(&child).copied() else {
            break;
        };
        let opened = pending.get(&of).and_then(|waiting| {
            waiting
                .iter()
                .find_map(|sealed| unwrap(&below, group, generation, of, 0, child, sealed))
        });
        match opened {
            Some(key) => {
                staged.insert(of, (0, key));
                pending.remove(&of);
            }
            None => break,
        }
    }
}

/// Seal a public-mailbox entry under the channel key: `0x04 ‖ sealed-v1`.
/// The nonce comes from the key and the entry.
pub(super) fn seal_entry(
    root: &[u8; 32],
    group: &[u8; 32],
    generation: u32,
    version: u32,
    entry: &[u8],
) -> Result<Vec<u8>, CoreError> {
    let key = derived(root, group, b"entries");
    // The same entry sealed again is the same envelope, under the same
    // stamp; two entries never share a nonce.
    let mut hash = Sha256::new();
    hash.update(key);
    hash.update(entry);
    let digest: [u8; 32] = hash.finalize().into();
    let mut nonce = [0; 12];
    nonce.copy_from_slice(&digest[..12]);
    let aad = entry_binding(group, generation, version);
    let sealed = ChaCha20Poly1305::new_from_slice(&key)
        .map_err(|_| CoreError::InvalidState)?
        .encrypt(
            Nonce::from_slice(&nonce),
            Payload {
                msg: entry,
                aad: &aad,
            },
        )
        .map_err(|_| CoreError::InvalidState)?;
    let mut e = minicbor::Encoder::new(vec![SEALED_ENTRY]);
    e.array(6)
        .and_then(|e| e.str("sealed-v1"))
        .and_then(|e| e.bytes(group))
        .and_then(|e| e.u32(generation))
        .and_then(|e| e.u32(version))
        .and_then(|e| e.bytes(&nonce))
        .and_then(|e| e.bytes(&sealed))
        .map_err(|_| CoreError::InvalidState)?;
    Ok(e.into_writer())
}

/// A sealed entry: its channel, the generation and root version of the key
/// it is sealed under, its nonce and what is sealed.
pub(super) type Sealed = ([u8; 32], u32, u32, [u8; 12], Vec<u8>);

/// What a sealed entry says of itself.
pub(super) fn sealed_under(wire: &[u8]) -> Option<Sealed> {
    let mut d = minicbor::Decoder::new(wire);
    if d.array().ok()? != Some(6) || d.str().ok()? != "sealed-v1" {
        return None;
    }
    let group: [u8; 32] = d.bytes().ok()?.try_into().ok()?;
    let generation = d.u32().ok()?;
    let version = d.u32().ok()?;
    let nonce: [u8; 12] = d.bytes().ok()?.try_into().ok()?;
    let sealed = d.bytes().ok()?.to_vec();
    (d.position() == wire.len()).then_some((group, generation, version, nonce, sealed))
}

pub(super) fn open_entry(
    root: &[u8; 32],
    group: &[u8; 32],
    generation: u32,
    version: u32,
    nonce: &[u8; 12],
    sealed: &[u8],
) -> Option<Vec<u8>> {
    let key = derived(root, group, b"entries");
    ChaCha20Poly1305::new_from_slice(&key)
        .ok()?
        .decrypt(
            Nonce::from_slice(nonce),
            Payload {
                msg: sealed,
                aad: &entry_binding(group, generation, version),
            },
        )
        .ok()
}

fn entry_binding(group: &[u8; 32], generation: u32, version: u32) -> Vec<u8> {
    let mut aad = b"AIN_CHANNEL_ENTRY_V1".to_vec();
    aad.extend_from_slice(group);
    aad.extend_from_slice(&generation.to_be_bytes());
    aad.extend_from_slice(&version.to_be_bytes());
    aad
}

/// An entry of a closed channel's mailbox sealed under its key.
pub const SEALED_ENTRY: u8 = 4;
/// An entry of a closed channel's mailbox: new keys.
pub const KEY_UPDATE: u8 = 5;
/// An entry of a closed channel's mailbox: a subscriber's own key.
pub const SUBSCRIBER_KEY: u8 = 6;

#[cfg(test)]
#[allow(clippy::unwrap_used)]
mod tests {
    use super::*;

    const GROUP: [u8; 32] = [7; 32];
    const SEED: [u8; 32] = [9; 32];

    fn keys_of(leaf: u32, removed: &[u32]) -> PathKeys {
        path(leaf)
            .into_iter()
            .map(|of| {
                let v = version(of, removed);
                (of, (v, node_key(&SEED, &GROUP, 0, of, v)))
            })
            .collect()
    }

    /// Everyone left derives the new channel key from a removal; the
    /// removed leaf does not; a removal's sealed keys fit a few KB.
    #[test]
    fn a_removal_moves_the_rest_to_the_keys_the_team_derives() {
        let (a, b, gone) = (0x0100_0000, 0x0100_0001, 0x0200_0005);
        let removed = vec![gone];
        let sealed = rekey(&SEED, &GROUP, 0, &removed, gone).unwrap();
        assert!(sealed.iter().map(Vec::len).sum::<usize>() < 3 * 1024);
        for own in [a, b] {
            let next = apply_rekey(&keys_of(own, &[]), own, &GROUP, 0, gone, &sealed).unwrap();
            assert_eq!(next, keys_of(own, &removed), "{own:#x}");
        }
        assert!(apply_rekey(&keys_of(gone, &[]), gone, &GROUP, 0, gone, &sealed).is_none());
        // A neighbour removed next: the shared keys move again.
        let removed = vec![gone, b];
        let sealed = rekey(&SEED, &GROUP, 0, &removed, b).unwrap();
        let next = apply_rekey(&keys_of(a, &[gone]), a, &GROUP, 0, b, &sealed).unwrap();
        assert_eq!(next, keys_of(a, &removed));
        assert!(apply_rekey(&keys_of(b, &[gone]), b, &GROUP, 0, b, &sealed).is_none());
    }

    fn public_of(secret: &[u8; 32]) -> [u8; 32] {
        x25519_dalek::PublicKey::from(&x25519_dalek::StaticSecret::from(*secret)).to_bytes()
    }

    /// Everyone with a key of its own and a leaf climbs to the next
    /// generation's root from a reseed onto those keys, whatever order its
    /// entries come in; whoever knew the old seed, and every key of the old
    /// generation, opens none of them.
    #[test]
    fn a_reseed_onto_subscribers_keys_reaches_them_and_nobody_with_the_old_seed() {
        let (a, b, c) = (0x0100_0000, 0x0100_0001, 0x0200_0005);
        let (sa, sb, sc) = ([31; 32], [32; 32], [33; 32]);
        let next = [11; 32];
        let ephemeral = [44; 32];
        // B gave no key of its own.
        let personal = BTreeMap::from([(a, public_of(&sa)), (c, public_of(&sc))]);
        let entries = hard_reseed(&next, &GROUP, 1, &[a, b, c], &personal, &ephemeral).unwrap();
        let bytes: usize = entries.iter().map(|(_, sealed)| 8 + sealed.len()).sum();
        assert!(bytes < 3 * 200 + 64 * 2 * 60, "{bytes} B");
        let new_keys = |leaf: u32| -> PathKeys {
            path(leaf)
                .into_iter()
                .map(|of| (of, (0, node_key(&next, &GROUP, 1, of, 0))))
                .collect()
        };
        // One entry at a time, in their order and backwards.
        for (leaf, secret) in [(a, sa), (c, sc)] {
            for backwards in [false, true] {
                let (mut staged, mut pending) = (PathKeys::new(), BTreeMap::new());
                let mut order: Vec<&(u64, Vec<u8>)> = entries.iter().collect();
                if backwards {
                    order.reverse();
                }
                for entry in order {
                    apply_hard_reseed(
                        &secret,
                        &public_of(&ephemeral),
                        leaf,
                        &GROUP,
                        1,
                        std::slice::from_ref(entry),
                        &mut staged,
                        &mut pending,
                    );
                }
                assert_eq!(staged, new_keys(leaf), "{leaf:#x} backwards: {backwards}");
            }
        }
        // Each node's entries open under a new key of a child: the check
        // below tries the same binding.
        for (of, sealed) in entries.iter().filter(|(of, _)| level_of(*of) < DEPTH) {
            let first = node(level_of(*of) + 1, (of & 0xffff_ffff) << 1);
            assert!(
                [first, sibling(first)].iter().any(|child| {
                    unwrap(
                        &node_key(&next, &GROUP, 1, *child, 0),
                        &GROUP,
                        1,
                        *of,
                        0,
                        *child,
                        sealed,
                    )
                    .is_some()
                }),
                "{of:#x}"
            );
        }
        // Without a key of its own, B opens nothing; nor with A's leaf and
        // B's secret.
        for (leaf, secret) in [(b, sb), (a, sb)] {
            let (mut staged, mut pending) = (PathKeys::new(), BTreeMap::new());
            apply_hard_reseed(
                &secret,
                &public_of(&ephemeral),
                leaf,
                &GROUP,
                1,
                &entries,
                &mut staged,
                &mut pending,
            );
            assert!(staged.is_empty(), "{leaf:#x}");
        }
        // Every key the old seed gives, at any version up to the removals
        // made, under every node and child: nothing opens.
        let old: Vec<[u8; 32]> = [a, b, c]
            .iter()
            .flat_map(|leaf| path(*leaf))
            .flat_map(|of| (0..4).map(move |v| node_key(&SEED, &GROUP, 0, of, v)))
            .collect();
        for (of, sealed) in &entries {
            let children = [node(level_of(*of) + 1, (of & 0xffff_ffff) << 1)]
                .into_iter()
                .flat_map(|child| [child, sibling(child)]);
            for under in children.chain([*of]) {
                for key in &old {
                    assert!(unwrap(key, &GROUP, 1, *of, 0, under, sealed).is_none());
                }
            }
            for key in &old {
                let (mut staged, mut pending) = (PathKeys::new(), BTreeMap::new());
                apply_hard_reseed(
                    key,
                    &public_of(&ephemeral),
                    a,
                    &GROUP,
                    1,
                    &[(*of, sealed.clone())],
                    &mut staged,
                    &mut pending,
                );
                assert!(staged.is_empty());
            }
        }
    }
}
