//! Discovery documents (spec/discovery-v1.md): what a profile sends the
//! discovery service, signed by its root key as documents of kind
//! `Directory`, the bindings the service signs, and the normal form of the
//! handles people are found by.
use crate::{DocumentKind, VerifiedDocument, WireError};
use minicbor::{Decoder, Encoder};
use sha2::{Digest, Sha256};

/// The kinds of accounts a profile binds.
pub const KINDS: [&str; 2] = ["google", "github"];
pub const MAX_TAGS: usize = 8;
const MAX_TAG: usize = 32;
pub const MAX_LANGS: usize = 4;
const MAX_ABOUT: usize = 500;
const MAX_NAME: usize = 64;
const MAX_SERVICE: usize = 256;

/// A card of the index: an open group's or a public channel's, by its
/// owner, or a profile's.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Card {
    Group {
        group_id: [u8; 32],
        name: String,
        about: String,
        tags: Vec<String>,
        langs: Vec<String>,
    },
    Channel {
        group_id: [u8; 32],
        name: String,
        about: String,
        tags: Vec<String>,
        langs: Vec<String>,
    },
    Profile {
        name: String,
        about: String,
        tags: Vec<String>,
        langs: Vec<String>,
    },
}

impl Card {
    fn parts(&self) -> (&str, &str, &[String], &[String]) {
        match self {
            Self::Group {
                name,
                about,
                tags,
                langs,
                ..
            }
            | Self::Channel {
                name,
                about,
                tags,
                langs,
                ..
            }
            | Self::Profile {
                name,
                about,
                tags,
                langs,
            } => (name, about, tags, langs),
        }
    }

    /// The card's own rules: a name like a contact's, a short about, a few
    /// tags of lowercase letters, digits and `-`, a few two-letter languages.
    pub fn check(&self) -> Result<(), WireError> {
        let (name, about, tags, langs) = self.parts();
        let tag_ok = |tag: &String| {
            (1..=MAX_TAG).contains(&tag.chars().count())
                && tag
                    .chars()
                    .all(|c| c.is_lowercase() || c.is_ascii_digit() || c == '-')
        };
        let lang_ok =
            |lang: &String| lang.len() == 2 && lang.chars().all(|c| c.is_ascii_lowercase());
        let valid = name.trim() == name
            && !name.is_empty()
            && name.chars().count() <= MAX_NAME
            && !name.chars().any(char::is_control)
            && about.chars().count() <= MAX_ABOUT
            && !about.chars().any(|c| c.is_control() && c != '\n')
            && tags.len() <= MAX_TAGS
            && tags.iter().all(tag_ok)
            && langs.len() <= MAX_LANGS
            && langs.iter().all(lang_ok);
        valid.then_some(()).ok_or(WireError::Malformed)
    }

    pub fn name(&self) -> &str {
        self.parts().0
    }
    pub fn about(&self) -> &str {
        self.parts().1
    }
    pub fn tags(&self) -> &[String] {
        self.parts().2
    }
    pub fn langs(&self) -> &[String] {
        self.parts().3
    }
}

/// What a profile signs for the discovery service.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Request {
    /// Bind this profile to the account the human signs in with.
    Link {
        kind: String,
        service: String,
    },
    /// Drop the binding of `kind`.
    Unlink {
        kind: String,
        service: String,
    },
    Card(Card),
    /// Take the card with this id off the index.
    Withdraw {
        card: String,
    },
}

fn strings(e: &mut Encoder<Vec<u8>>, items: &[String]) {
    let _ = e.array(items.len() as u64);
    for item in items {
        let _ = e.str(item);
    }
}

fn read_strings(d: &mut Decoder<'_>, max: usize) -> Result<Vec<String>, WireError> {
    let count = d
        .array()
        .map_err(|_| WireError::Malformed)?
        .ok_or(WireError::Malformed)?;
    if count > max as u64 {
        return Err(WireError::TooLarge);
    }
    (0..count)
        .map(|_| d.str().map(str::to_owned).map_err(|_| WireError::Malformed))
        .collect()
}

fn text(d: &mut Decoder<'_>) -> Result<String, WireError> {
    d.str().map(str::to_owned).map_err(|_| WireError::Malformed)
}

impl Request {
    pub fn encode(&self) -> Vec<u8> {
        let mut e = Encoder::new(Vec::new());
        match self {
            Self::Link { kind, service } | Self::Unlink { kind, service } => {
                let _ = e.array(3);
                let _ = e.str(if matches!(self, Self::Link { .. }) {
                    "link-v1"
                } else {
                    "unlink-v1"
                });
                let _ = e.str(kind);
                let _ = e.str(service);
            }
            Self::Card(
                card @ (Card::Group {
                    group_id,
                    name,
                    about,
                    tags,
                    langs,
                }
                | Card::Channel {
                    group_id,
                    name,
                    about,
                    tags,
                    langs,
                }),
            ) => {
                let _ = e.array(6);
                let _ = e.str(if matches!(card, Card::Channel { .. }) {
                    "channel-card-v1"
                } else {
                    "group-card-v1"
                });
                let _ = e.bytes(group_id);
                let _ = e.str(name);
                let _ = e.str(about);
                strings(&mut e, tags);
                strings(&mut e, langs);
            }
            Self::Card(Card::Profile {
                name,
                about,
                tags,
                langs,
            }) => {
                let _ = e.array(5);
                let _ = e.str("profile-card-v1");
                let _ = e.str(name);
                let _ = e.str(about);
                strings(&mut e, tags);
                strings(&mut e, langs);
            }
            Self::Withdraw { card } => {
                let _ = e.array(2);
                let _ = e.str("withdraw-v1");
                let _ = e.str(card);
            }
        }
        e.into_writer()
    }

    pub fn decode(bytes: &[u8]) -> Result<Self, WireError> {
        let mut d = Decoder::new(bytes);
        let count = d
            .array()
            .map_err(|_| WireError::Malformed)?
            .ok_or(WireError::Malformed)?;
        let tag = d.str().map_err(|_| WireError::Malformed)?;
        let request = match (tag, count) {
            ("link-v1", 3) | ("unlink-v1", 3) => {
                let kind = text(&mut d)?;
                let service = text(&mut d)?;
                if !KINDS.contains(&kind.as_str()) || service.len() > MAX_SERVICE {
                    return Err(WireError::Malformed);
                }
                if tag == "link-v1" {
                    Self::Link { kind, service }
                } else {
                    Self::Unlink { kind, service }
                }
            }
            ("group-card-v1" | "channel-card-v1", 6) => {
                let group_id = d
                    .bytes()
                    .map_err(|_| WireError::Malformed)?
                    .try_into()
                    .map_err(|_| WireError::Malformed)?;
                let (name, about) = (text(&mut d)?, text(&mut d)?);
                let tags = read_strings(&mut d, MAX_TAGS)?;
                let langs = read_strings(&mut d, MAX_LANGS)?;
                Self::Card(if tag == "channel-card-v1" {
                    Card::Channel {
                        group_id,
                        name,
                        about,
                        tags,
                        langs,
                    }
                } else {
                    Card::Group {
                        group_id,
                        name,
                        about,
                        tags,
                        langs,
                    }
                })
            }
            ("profile-card-v1", 5) => Self::Card(Card::Profile {
                name: text(&mut d)?,
                about: text(&mut d)?,
                tags: read_strings(&mut d, MAX_TAGS)?,
                langs: read_strings(&mut d, MAX_LANGS)?,
            }),
            ("withdraw-v1", 2) => Self::Withdraw {
                card: text(&mut d)?,
            },
            _ => return Err(WireError::Malformed),
        };
        if d.position() != bytes.len() || request.encode() != bytes {
            return Err(WireError::NonCanonical);
        }
        Ok(request)
    }
}

/// A request of the profile whose root key signed it.
pub fn verify_request(
    wire: &[u8],
    domain: [u8; 32],
    now: u64,
) -> Result<([u8; 32], Request, u64), WireError> {
    let document = VerifiedDocument::decode(wire, domain, now)?;
    if document.kind() != DocumentKind::Directory {
        return Err(WireError::Malformed);
    }
    Ok((
        *document.author(),
        Request::decode(document.body())?,
        document.issued_at(),
    ))
}

/// A handle bound to a profile, as the discovery service signs it.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Binding {
    pub kind: String,
    /// SHA-256 of the handle's normal form.
    pub digest: [u8; 32],
    pub network_id: String,
    pub issued_at: u64,
}

impl Binding {
    pub fn encode(&self) -> Vec<u8> {
        let mut e = Encoder::new(Vec::new());
        let _ = e.array(5);
        let _ = e.str("binding-v1");
        let _ = e.str(&self.kind);
        let _ = e.bytes(&self.digest);
        let _ = e.str(&self.network_id);
        let _ = e.u64(self.issued_at);
        e.into_writer()
    }

    pub fn decode(bytes: &[u8]) -> Result<Self, WireError> {
        let mut d = Decoder::new(bytes);
        if d.array().map_err(|_| WireError::Malformed)? != Some(5)
            || d.str().map_err(|_| WireError::Malformed)? != "binding-v1"
        {
            return Err(WireError::Malformed);
        }
        let binding = Self {
            kind: text(&mut d)?,
            digest: d
                .bytes()
                .map_err(|_| WireError::Malformed)?
                .try_into()
                .map_err(|_| WireError::Malformed)?,
            network_id: text(&mut d)?,
            issued_at: d.u64().map_err(|_| WireError::Malformed)?,
        };
        if d.position() != bytes.len() || binding.encode() != bytes {
            return Err(WireError::NonCanonical);
        }
        Ok(binding)
    }
}

/// A binding signed by the service key `key`.
pub fn verify_binding(
    wire: &[u8],
    domain: [u8; 32],
    now: u64,
    key: &[u8; 32],
) -> Result<Binding, WireError> {
    let document = VerifiedDocument::decode(wire, domain, now)?;
    if document.kind() != DocumentKind::Directory || document.author() != key {
        return Err(WireError::InvalidSignature);
    }
    Binding::decode(document.body())
}

/// The form a handle is found by: an email trimmed and lowercased, a Gmail
/// address without dots or a `+suffix` in its local part and at
/// `gmail.com`; a GitHub login lowercased. `None` for something that is
/// not one.
pub fn normalize_handle(kind: &str, handle: &str) -> Option<String> {
    let handle = handle.trim().to_lowercase();
    match kind {
        "google" => {
            let (local, domain) = handle.rsplit_once('@')?;
            let usable_domain = domain.contains('.')
                && domain.split('.').all(|label| {
                    !label.is_empty() && label.chars().all(|c| c.is_alphanumeric() || c == '-')
                });
            if local.is_empty()
                || !usable_domain
                || handle.chars().any(|c| c.is_whitespace() || c.is_control())
                || local.chars().any(|c| "@\"(),:;<>[\\]".contains(c))
            {
                return None;
            }
            if domain == "gmail.com" || domain == "googlemail.com" {
                let local: String = local
                    .split('+')
                    .next()
                    .unwrap_or_default()
                    .chars()
                    .filter(|c| *c != '.')
                    .collect();
                (!local.is_empty()).then(|| format!("{local}@gmail.com"))
            } else {
                Some(handle)
            }
        }
        "github" => {
            let valid = (1..=39).contains(&handle.len())
                && handle
                    .chars()
                    .all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || c == '-')
                && !handle.starts_with('-')
                && !handle.ends_with('-');
            valid.then_some(handle)
        }
        _ => None,
    }
}

/// What a lookup sends for a handle: SHA-256 of its normal form.
pub fn handle_digest(kind: &str, handle: &str) -> Option<[u8; 32]> {
    normalize_handle(kind, handle).map(|normal| Sha256::digest(normal.as_bytes()).into())
}

#[cfg(test)]
#[path = "directory_tests.rs"]
mod tests;
