//! Shared scoped credentials and signed IPC for MCP and the standalone CLI.
use crate::{Result, ipc};
use agentic_core::AgentCall;
use agentic_protocol::SignedDocument;
use ed25519_dalek::SigningKey;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::{
    fs::{self, File},
    io::Read,
    os::unix::fs::{MetadataExt, PermissionsExt},
    path::{Path, PathBuf},
    time::{SystemTime, UNIX_EPOCH},
};
use zeroize::{Zeroize, Zeroizing};

#[derive(Deserialize, Serialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub(crate) struct Credentials {
    pub(crate) version: u8,
    pub(crate) ipc: PathBuf,
    pub(crate) domain: [u8; 32],
    pub(crate) grant_id: [u8; 32],
    pub(crate) ownership_epoch: u64,
    pub(crate) signing_seed: String,
}
impl Drop for Credentials {
    fn drop(&mut self) {
        self.signing_seed.zeroize();
    }
}
pub(crate) struct RuntimeClient {
    credentials: Credentials,
    key: SigningKey,
}
impl RuntimeClient {
    pub(crate) fn load(path: &Path) -> Result<Self> {
        let initial = fs::symlink_metadata(path)?;
        if !initial.is_file() || initial.permissions().mode() & 0o077 != 0 {
            return Err("credentials must be a private regular file".into());
        }
        let file = File::open(path)?;
        let actual = file.metadata()?;
        if actual.dev() != initial.dev()
            || actual.ino() != initial.ino()
            || actual.permissions().mode() & 0o077 != 0
            || actual.len() > 4096
        {
            return Err("credential file changed or exceeds limit".into());
        }
        let mut bytes = Zeroizing::new(Vec::new());
        file.take(4097).read_to_end(&mut bytes)?;
        if bytes.len() > 4096 {
            return Err("credential file exceeds limit".into());
        }
        let mut credentials: Credentials =
            serde_json::from_slice(&bytes).map_err(|_| "invalid credentials")?;
        if credentials.version != 1 || !credentials.ipc.is_absolute() {
            return Err("unsupported credentials".into());
        }
        let mut seed = Zeroizing::new([0; 32]);
        hex::decode_to_slice(&credentials.signing_seed, &mut *seed)
            .map_err(|_| "invalid signing seed")?;
        credentials.signing_seed.zeroize();
        Ok(Self {
            credentials,
            key: SigningKey::from_bytes(&seed),
        })
    }
    pub(crate) async fn invoke(&self, method: &str, request: Value) -> Result<Value> {
        let mut nonce = [0; 32];
        getrandom::fill(&mut nonce)?;
        let call = AgentCall {
            grant_id: self.credentials.grant_id,
            method: method.into(),
            request,
            nonce,
        };
        let draft = call.draft(
            self.credentials.domain,
            self.credentials.ownership_epoch,
            SystemTime::now().duration_since(UNIX_EPOCH)?.as_secs(),
        )?;
        let proof = SignedDocument::sign(draft, &self.key)?;
        ipc::call_agent(&self.credentials.ipc, proof.as_wire()).await
    }
}
