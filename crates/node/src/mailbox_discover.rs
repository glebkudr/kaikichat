//! The discovery service at the node (spec/discovery-v1.md): as a client,
//! what the owner CLI sends the service is signed here and its stamps are
//! spent here; as the service's node, the stamps it is paid with and the
//! books its searchers show are checked here.
use super::mailbox_holder::{Refusal, Statement};
use super::*;
use agentic_mailbox_swarm::discover::{card_operation, lookup_operation};
use agentic_mailbox_swarm::stamp::{Stamp, StampError};
use agentic_protocol::directory::{Card, KINDS, Request as Signed};
use sha2::{Digest, Sha256};

#[derive(Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub(super) struct StampJson {
    book: String,
    index: u32,
    operation: String,
    signature: String,
}

impl StampJson {
    fn stamp(&self) -> Option<Stamp> {
        Some(Stamp {
            book: hex::decode(&self.book).ok()?.try_into().ok()?,
            index: self.index,
            operation: hex::decode(&self.operation).ok()?.try_into().ok()?,
            signature: hex::decode(&self.signature).ok()?.try_into().ok()?,
            holders: None,
        })
    }
}

pub(super) fn stamp_json(stamp: &Stamp) -> Value {
    json!({
        "book": hex::encode(stamp.book),
        "index": stamp.index,
        "operation": hex::encode(stamp.operation),
        "signature": hex::encode(stamp.signature),
    })
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub(super) struct Redeem {
    stamps: Vec<StampJson>,
    /// Grants of the stamps' books, for books this node does not know.
    #[serde(default)]
    grants: Vec<agentic_grant_book::GrantBook>,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub(super) struct BookStatus {
    book: String,
    #[serde(default)]
    grant: Option<agentic_grant_book::GrantBook>,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub(super) struct Consent {
    action: String,
    kind: String,
    service: String,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub(super) struct HandleJson {
    kind: String,
    digest: String,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub(super) struct Stamps {
    handles: Vec<HandleJson>,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub(super) struct CardRequest {
    kind: String,
    #[serde(default)]
    group_id: Option<String>,
    #[serde(default)]
    name: Option<String>,
    about: String,
    #[serde(default)]
    tags: Vec<String>,
    #[serde(default)]
    langs: Vec<String>,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub(super) struct Withdrawal {
    card_id: String,
}

type Answer = std::result::Result<Value, (&'static str, String)>;

/// How long a card made for the service is made again unchanged: less than
/// the service's link TTL, within which a card must first be shown.
const CARD_REUSE_SECS: u64 = 600;

fn invalid(message: &str) -> (&'static str, String) {
    ("invalid_request", message.to_owned())
}

/// Stamps pay only once a book is known; this is why they could not.
fn book_error(error: CoreError) -> (&'static str, String) {
    let code = match error {
        CoreError::MailboxBookMissing | CoreError::MailboxBookExpired => "book_required",
        CoreError::MailboxBookExhausted => "book_exhausted",
        _ => "invalid_request",
    };
    (code, error.to_string())
}

impl Runtime {
    /// Check stamps paying the discovery service: each against its book,
    /// its slot recorded as a notary does (the same operation again is the
    /// same spend, another one a double spend that blocks the book) and put
    /// on record with the slot's notaries.
    pub(super) fn redeem_stamps(&mut self, input: Redeem) -> Answer {
        if !self.chain_configured() {
            return Err(("chain_not_configured", "the node reads no chain".into()));
        }
        let now = clock::wall().map_err(|e| ("unavailable", e.to_string()))?;
        // A grant's book is learned once its notaries vouch for it; until
        // then its stamps are unknown here.
        for grant in input.grants {
            if self.mailbox_holder.book_terms(&grant.id()).is_none() {
                let _ = self.offer_grant(grant);
            }
        }
        let mut results = vec![];
        for wire in &input.stamps {
            let Some(stamp) = wire.stamp() else {
                results.push(json!("malformed"));
                continue;
            };
            results.push(json!(self.redeem(&stamp, now)));
        }
        Ok(json!({ "results": results }))
    }

    fn redeem(&mut self, stamp: &Stamp, now: u64) -> &'static str {
        let Some(terms) = self.mailbox_holder.book_terms(&stamp.book) else {
            self.book_wanted(stamp.book, false);
            return "unknown_book";
        };
        if self.mailbox_holder.is_blocked(&stamp.book) {
            return "blocked";
        }
        match stamp.verify(&NETWORK_DOMAIN, &terms, now) {
            Ok(()) => {}
            Err(StampError::Index) => return "index",
            Err(StampError::Expired) => return "expired",
            Err(_) => return "signature",
        }
        match self
            .mailbox_holder
            .notarize(&Statement::Ticket(stamp.clone()), now)
        {
            Ok(noted) => match noted.first {
                Statement::Ticket(first) if first.operation != stamp.operation => "conflict",
                _ => {
                    self.notarize_later(Statement::Ticket(stamp.clone()));
                    "ok"
                }
            },
            Err(Refusal::Blocked) => "blocked",
            Err(_) => "storage",
        }
    }

    /// What this node knows of a book a searcher shows.
    pub(super) fn book_status(&mut self, input: BookStatus) -> Answer {
        let book: [u8; 32] = hex::decode(&input.book)
            .ok()
            .and_then(|b| b.try_into().ok())
            .ok_or_else(|| invalid("book is 32 bytes of hex"))?;
        let now = clock::wall().map_err(|e| ("unavailable", e.to_string()))?;
        if self.mailbox_holder.is_blocked(&book) {
            return Ok(json!({"state": "blocked"}));
        }
        let terms = match self.mailbox_holder.book_terms(&book) {
            Some(terms) => Some(terms),
            None => match input.grant.filter(|g| g.id() == book) {
                Some(grant) if self.grant_rules_current(grant.server, grant.day) => {
                    self.mailbox_holder.check_grant(&grant, now).ok().map(|()| {
                        agentic_mailbox_swarm::stamp::BookTerms {
                            key: grant.book,
                            count: grant.count,
                            valid_until: grant.expiry,
                        }
                    })
                }
                _ => {
                    self.book_wanted(book, false);
                    None
                }
            },
        };
        Ok(match terms {
            None => json!({"state": "unknown"}),
            Some(terms) if now >= terms.valid_until => json!({"state": "ended"}),
            Some(terms) => json!({"state": "active", "key": hex::encode(terms.key)}),
        })
    }

    /// A consent to the discovery service at `service`, signed by this
    /// profile's root key.
    pub(super) fn discover_consent(&mut self, input: Consent, now: u64) -> Answer {
        if !KINDS.contains(&input.kind.as_str()) {
            return Err(invalid("kind is google or github"));
        }
        let body = match input.action.as_str() {
            "link" => Signed::Link {
                kind: input.kind,
                service: input.service,
            },
            "unlink" => Signed::Unlink {
                kind: input.kind,
                service: input.service,
            },
            _ => return Err(invalid("action is link or unlink")),
        };
        let wire = self
            .core
            .directory_document(body.encode(), now)
            .map_err(|e| invalid(&e.to_string()))?;
        Ok(json!({ "consent": hex::encode(wire) }))
    }

    /// Stamps paying for a lookup of `handles` today (UTC), one each; the
    /// same lookup again the same day is paid with the same stamps.
    pub(super) fn discover_stamps(&mut self, input: Stamps, now: u64) -> Answer {
        let day = now / 86_400;
        let mut stamps = vec![];
        for handle in &input.handles {
            let digest: [u8; 32] = hex::decode(&handle.digest)
                .ok()
                .and_then(|d| d.try_into().ok())
                .ok_or_else(|| invalid("a digest is 32 bytes of hex"))?;
            if !KINDS.contains(&handle.kind.as_str()) {
                return Err(invalid("kind is google or github"));
            }
            stamps.push(
                self.core
                    .stamp_operation(
                        lookup_operation(&NETWORK_DOMAIN, &handle.kind, &digest, day),
                        now,
                    )
                    .map_err(book_error)?,
            );
        }
        let grants = self.stamp_grants(&stamps);
        Ok(json!({
            "day": day,
            "stamps": stamps.iter().map(stamp_json).collect::<Vec<_>>(),
            "grants": grants,
        }))
    }

    /// The grants of the books `stamps` are of, for the service's node.
    fn stamp_grants(&self, stamps: &[Stamp]) -> Vec<agentic_grant_book::GrantBook> {
        let mut books: Vec<[u8; 32]> = stamps.iter().map(|s| s.book).collect();
        books.sort_unstable();
        books.dedup();
        books
            .iter()
            .filter_map(|book| self.core.mailbox_book_grant(book).ok().flatten())
            .collect()
    }

    /// A card of an open group or a public channel of this profile, or of
    /// the profile, signed by its root key, with the ten stamps that
    /// publish it.
    pub(super) fn discover_card(&mut self, input: CardRequest, now: u64) -> Answer {
        let card = match input.kind.as_str() {
            "group" | "channel" => {
                let id = input
                    .group_id
                    .ok_or_else(|| invalid("a group card names its group"))?;
                let group = self.core.group(&id).map_err(mailbox_groups::group_error)?;
                if group.role != "owner" {
                    return Err(("not_allowed", "only the owner publishes a group".into()));
                }
                if group.access != "public" {
                    return Err(("group_private", "only an open group is published".into()));
                }
                let group_id = hex::decode(&id)
                    .ok()
                    .and_then(|g| g.try_into().ok())
                    .ok_or_else(|| invalid("group id"))?;
                let name = input.name.unwrap_or(group.name);
                if group.kind == "channel" {
                    Card::Channel {
                        group_id,
                        name,
                        about: input.about,
                        tags: input.tags,
                        langs: input.langs,
                    }
                } else {
                    Card::Group {
                        group_id,
                        name,
                        about: input.about,
                        tags: input.tags,
                        langs: input.langs,
                    }
                }
            }
            "profile" => Card::Profile {
                name: match input.name {
                    Some(name) => name,
                    None => self
                        .core
                        .snapshot()
                        .ok()
                        .and_then(|s| s.identity)
                        .map(|i| i.name)
                        .ok_or_else(|| invalid("no profile"))?,
                },
                about: input.about,
                tags: input.tags,
                langs: input.langs,
            },
            _ => return Err(invalid("kind is group, channel or profile")),
        };
        if card.check().is_err() {
            return Err((
                "bad_card",
                "a name, a short about, up to 8 lowercase tags and 4 languages".into(),
            ));
        }
        // The same card made again soon is the one made before: its
        // operations, and so its stamps, are the same.
        let body = Signed::Card(card).encode();
        self.discover_cards
            .retain(|_, (_, made)| now < made.saturating_add(CARD_REUSE_SECS));
        let wire = match self.discover_cards.get(&body) {
            Some((wire, _)) => wire.clone(),
            None => {
                let wire = self
                    .core
                    .directory_document(body.clone(), now)
                    .map_err(|e| invalid(&e.to_string()))?;
                self.discover_cards.insert(body, (wire.clone(), now));
                wire
            }
        };
        let id: [u8; 32] = Sha256::digest(&wire).into();
        let mut stamps = vec![];
        for index in 0..10 {
            stamps.push(
                self.core
                    .stamp_operation(card_operation(&NETWORK_DOMAIN, &id, index), now)
                    .map_err(book_error)?,
            );
        }
        let grants = self.stamp_grants(&stamps);
        Ok(json!({
            "card": hex::encode(wire),
            "id": hex::encode(id),
            "stamps": stamps.iter().map(stamp_json).collect::<Vec<_>>(),
            "grants": grants,
        }))
    }

    pub(super) fn discover_withdrawal(&mut self, input: Withdrawal, now: u64) -> Answer {
        let wire = self
            .core
            .directory_document(
                Signed::Withdraw {
                    card: input.card_id,
                }
                .encode(),
                now,
            )
            .map_err(|e| invalid(&e.to_string()))?;
        Ok(json!({ "withdrawal": hex::encode(wire) }))
    }

    /// A pass to search with: a random nonce in place of a peer.
    pub(super) fn discover_pass(&mut self, now: u64) -> Answer {
        let mut nonce = [0; 32];
        random::fill(&mut nonce).map_err(|e| ("unavailable", e.to_string()))?;
        let access = self
            .core
            .mailbox_access(nonce, agentic_mailbox_swarm::address::period(now), now)
            .map_err(|e| invalid(&e.to_string()))?
            .ok_or(("book_required", "no active book to show".into()))?;
        Ok(json!({ "pass": {
            "book": hex::encode(access.pass.book),
            "peer": hex::encode(access.pass.peer),
            "day": access.pass.day,
            "signature": hex::encode(access.pass.signature),
            "grant": access.grant,
        }}))
    }
}
