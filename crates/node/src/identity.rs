//! The identity server's claim API as a node uses it (`coins claim`, phase 1b
//! of Docs/V1_MAILBOX_SWARM_IMPLEMENTATION.md; server:
//! services/identity-server): post a book-key-signed request, then read the
//! claim until the human signed in and it was granted or denied. A holder
//! also reports grants spent twice and reads the grants the server revoked
//! (Docs/V1_IDENTITY_PENALTIES_2026_09_30.md).
use agentic_grant_book::{ClaimRequest, GrantBook, GrantRevocation};
use agentic_mailbox_swarm::stamp::Stamp;

/// The server's answer to a claim request.
#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct ClaimOpened {
    pub(crate) claim_id: String,
    pub(crate) login_url: String,
    pub(crate) expires_at: u64,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) enum ClaimStatus {
    Pending,
    Granted(GrantBook),
    Denied(String),
}

/// What the server made of a reported double spend.
#[derive(Clone, Copy, Debug, PartialEq, Eq, serde::Deserialize)]
pub(crate) struct Reported {
    pub(crate) banned: bool,
    pub(crate) revoked: u64,
}

/// Revocations after a cursor, and the number of the last one listed.
#[derive(Clone, Debug, PartialEq, Eq, serde::Deserialize)]
pub(crate) struct RevocationPage {
    pub(crate) revocations: Vec<GrantRevocation>,
    pub(crate) last: u64,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) enum IdentityError {
    Transport(String),
    /// The server refused with its error code (`stale_request`, …).
    Refused(String),
    Malformed(&'static str),
}

#[async_trait::async_trait]
pub(crate) trait IdentityServer: Send + Sync {
    async fn create(&self, request: &ClaimRequest) -> Result<ClaimOpened, IdentityError>;
    async fn status(&self, claim_id: &str) -> Result<ClaimStatus, IdentityError>;
    /// Two stamps of one slot of `grant`'s book for different operations.
    async fn report(
        &self,
        grant: &GrantBook,
        first: &Stamp,
        second: &Stamp,
    ) -> Result<Reported, IdentityError>;
    async fn revocations(&self, after: u64) -> Result<RevocationPage, IdentityError>;
}

/// A stamp in the form of the directory's HTTP API.
fn stamp_json(stamp: &Stamp) -> serde_json::Value {
    serde_json::json!({
        "book": hex::encode(stamp.book),
        "index": stamp.index,
        "operation": hex::encode(stamp.operation),
        "signature": hex::encode(stamp.signature),
    })
}

/// The claim API over HTTP.
pub(crate) struct IdentityClient {
    base: String,
    http: reqwest::Client,
}

#[derive(serde::Deserialize)]
#[serde(rename_all = "camelCase")]
struct Opened {
    claim_id: String,
    login_url: String,
    expires_at: u64,
}

#[derive(serde::Deserialize)]
struct Decision {
    status: String,
    reason: Option<String>,
    grant: Option<GrantBook>,
}

#[derive(serde::Deserialize)]
struct Refusal {
    error: String,
}

impl IdentityClient {
    pub(crate) fn new(base: &str) -> Result<Self, IdentityError> {
        let http = reqwest::Client::builder()
            .timeout(std::time::Duration::from_secs(10))
            .build()
            .map_err(|error| IdentityError::Transport(error.to_string()))?;
        Ok(Self {
            base: base.trim_end_matches('/').to_owned(),
            http,
        })
    }

    /// The body of a success; a 4xx with the server's error code is its
    /// refusal; anything else (5xx, a proxy page) is a transport failure.
    async fn answer<T: serde::de::DeserializeOwned>(
        response: reqwest::Response,
    ) -> Result<T, IdentityError> {
        let status = response.status();
        let bytes = response
            .bytes()
            .await
            .map_err(|error| IdentityError::Transport(error.to_string()))?;
        if status.is_success() {
            return serde_json::from_slice(&bytes).map_err(|_| IdentityError::Malformed("answer"));
        }
        if status.is_client_error()
            && let Ok(refusal) = serde_json::from_slice::<Refusal>(&bytes)
        {
            return Err(IdentityError::Refused(refusal.error));
        }
        Err(IdentityError::Transport(format!("HTTP {status}")))
    }
}

#[async_trait::async_trait]
impl IdentityServer for IdentityClient {
    async fn create(&self, request: &ClaimRequest) -> Result<ClaimOpened, IdentityError> {
        let response = self
            .http
            .post(format!("{}/v1/claims", self.base))
            .json(&serde_json::json!({ "request": request }))
            .send()
            .await
            .map_err(|error| IdentityError::Transport(error.to_string()))?;
        let opened: Opened = Self::answer(response).await?;
        Ok(ClaimOpened {
            claim_id: opened.claim_id,
            login_url: opened.login_url,
            expires_at: opened.expires_at,
        })
    }

    async fn status(&self, claim_id: &str) -> Result<ClaimStatus, IdentityError> {
        let response = self
            .http
            .get(format!("{}/v1/claims/{claim_id}", self.base))
            .send()
            .await
            .map_err(|error| IdentityError::Transport(error.to_string()))?;
        let decision: Decision = Self::answer(response).await?;
        match (decision.status.as_str(), decision.grant, decision.reason) {
            ("pending", _, _) => Ok(ClaimStatus::Pending),
            ("granted", Some(grant), _) => Ok(ClaimStatus::Granted(grant)),
            ("denied", _, Some(reason)) => Ok(ClaimStatus::Denied(reason)),
            _ => Err(IdentityError::Malformed("claim status")),
        }
    }

    async fn report(
        &self,
        grant: &GrantBook,
        first: &Stamp,
        second: &Stamp,
    ) -> Result<Reported, IdentityError> {
        let body = serde_json::json!({
            "grant": grant,
            "first": stamp_json(first),
            "second": stamp_json(second),
        });
        let response = self
            .http
            .post(format!("{}/v1/reports", self.base))
            .json(&body)
            .send()
            .await
            .map_err(|error| IdentityError::Transport(error.to_string()))?;
        Self::answer(response).await
    }

    async fn revocations(&self, after: u64) -> Result<RevocationPage, IdentityError> {
        let response = self
            .http
            .get(format!("{}/v1/revocations?after={after}", self.base))
            .send()
            .await
            .map_err(|error| IdentityError::Transport(error.to_string()))?;
        Self::answer(response).await
    }
}

#[cfg(test)]
#[path = "identity_tests.rs"]
mod tests;
