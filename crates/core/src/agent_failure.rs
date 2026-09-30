//! Safe public agent failures shared by IPC and MCP; private diagnostics stay internal.
use crate::CoreError;
use agentic_capabilities::CapabilityError;
use agentic_store::StoreError;
use serde::Serialize;

#[derive(Serialize)]
#[serde(rename_all = "snake_case")]
enum Code {
    Unauthorized,
    InboxBusy,
    InboxLeaseExpired,
    InboxItemTooLarge,
    IdempotencyConflict,
    Unavailable,
    Cancelled,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct SizeDetails {
    required_bytes: u64,
}

#[derive(Serialize)]
pub struct AgentFailure {
    code: Code,
    message: &'static str,
    retryable: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    details: Option<SizeDetails>,
}
impl AgentFailure {
    fn new(code: Code, message: &'static str, retryable: bool) -> Self {
        Self {
            code,
            message,
            retryable,
            details: None,
        }
    }
    pub fn unavailable() -> Self {
        Self::new(
            Code::Unavailable,
            "Daemon request unavailable; retry with the same operation ID after recovery",
            true,
        )
    }
    pub fn cancelled() -> Self {
        Self::new(Code::Cancelled, "Request cancelled", false)
    }
}

impl CoreError {
    /// Exposes actionable domain outcomes without revealing private storage or authorization data.
    pub fn agent_failure(&self) -> AgentFailure {
        match self {
            Self::InboxBusy => AgentFailure::new(
                Code::InboxBusy,
                "Inbox has an active lease; retry after acknowledgment or expiry",
                true,
            ),
            Self::InboxLeaseExpired => AgentFailure::new(
                Code::InboxLeaseExpired,
                "Lease expired or was replaced; poll with a new operation ID",
                false,
            ),
            Self::InboxItemTooLarge { required_bytes } => {
                let mut failure = AgentFailure::new(
                    Code::InboxItemTooLarge,
                    "Next item exceeds this page's byte limit; adjust maxBytes within your grant",
                    false,
                );
                failure.details = Some(SizeDetails {
                    required_bytes: *required_bytes,
                });
                failure
            }
            Self::Store(StoreError::IdempotencyConflict)
            | Self::Capability(CapabilityError::IdempotencyConflict) => AgentFailure::new(
                Code::IdempotencyConflict,
                "Operation ID already binds another request; retry the original request or use a new ID",
                false,
            ),
            Self::Store(error) | Self::Capability(CapabilityError::Store(error))
                if transient_store_error(error) =>
            {
                AgentFailure::unavailable()
            }
            Self::Randomness => AgentFailure::unavailable(),
            _ => AgentFailure::new(
                Code::Unauthorized,
                "Agent authorization or request rejected",
                false,
            ),
        }
    }
}
fn transient_store_error(error: &StoreError) -> bool {
    matches!(
        error,
        StoreError::Database(_)
            | StoreError::Io(_)
            | StoreError::StateConflict
            | StoreError::Randomness
    )
}
