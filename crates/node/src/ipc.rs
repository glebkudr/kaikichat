//! Bounded local IPC with disjoint owner-token and signed agent-proof boundaries.
use crate::Result;
use serde::Deserialize;
use serde_json::{Value, json};
use std::{
    fs,
    os::unix::fs::{FileTypeExt, PermissionsExt},
    path::{Path, PathBuf},
    sync::Arc,
    time::Duration,
};
use subtle::ConstantTimeEq;
use tokio::{
    io::{AsyncReadExt, AsyncWriteExt},
    net::{UnixListener, UnixStream},
    sync::{Semaphore, mpsc, oneshot},
};
use zeroize::{Zeroize, Zeroizing};
pub const MAX_REQUEST: usize = 1024 * 1024;
pub const MAX_RESPONSE: usize = 16 * 1024 * 1024;
const DEADLINE: Duration = Duration::from_secs(5);
#[cfg(test)]
#[path = "ipc_tests.rs"]
mod tests;
pub(crate) struct Command {
    pub kind: CommandKind,
    pub reply: oneshot::Sender<Value>,
}
impl Command {
    /// A disconnected caller cannot leave an unstarted mutation in the actor queue.
    /// Once dispatch starts, its atomic commit owns the outcome; retries use operation IDs.
    pub fn execute(self, dispatch: impl FnOnce(CommandKind) -> Value) {
        if !self.reply.is_closed() {
            let result = dispatch(self.kind);
            let _ = self.reply.send(result);
        }
    }
}
pub(crate) enum CommandKind {
    Owner { method: String, request: Value },
    Agent { proof: Vec<u8> },
}
#[derive(Deserialize)]
#[serde(untagged)]
enum Request {
    Owner(OwnerRequest),
    Agent(AgentRequest),
}
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct OwnerRequest {
    token: String,
    method: String,
    #[serde(default)]
    request: Value,
}
impl Drop for OwnerRequest {
    fn drop(&mut self) {
        self.token.zeroize();
    }
}
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct AgentRequest {
    proof: String,
}
pub fn error(code: &str, message: &str) -> Value {
    json!({"error":{"code":code,"message":message}})
}
pub(crate) fn agent_failure(failure: agentic_core::AgentFailure) -> Value {
    json!({"error":failure})
}

pub(crate) struct SocketGuard(PathBuf);
impl Drop for SocketGuard {
    fn drop(&mut self) {
        let _ = fs::remove_file(&self.0);
    }
}
pub(crate) fn bind(path: &Path) -> Result<(UnixListener, SocketGuard)> {
    let parent = path
        .parent()
        .ok_or("IPC path requires a private directory")?;
    let metadata = fs::metadata(parent)?;
    if !metadata.is_dir() || metadata.permissions().mode() & 0o077 != 0 {
        return Err("IPC directory must be private (0700)".into());
    }
    match fs::symlink_metadata(path) {
        Ok(existing) => {
            if !existing.file_type().is_socket() {
                return Err("IPC path is not a socket".into());
            }
            match std::os::unix::net::UnixStream::connect(path) {
                Err(e) if e.kind() == std::io::ErrorKind::ConnectionRefused => {
                    fs::remove_file(path)?
                }
                _ => return Err("IPC socket is already in use or inaccessible".into()),
            }
        }
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => {}
        Err(e) => return Err(e.into()),
    }
    let listener = UnixListener::bind(path)?;
    let guard = SocketGuard(path.into());
    fs::set_permissions(path, fs::Permissions::from_mode(0o600))?;
    Ok((listener, guard))
}
pub(crate) async fn serve(
    listener: UnixListener,
    token: Arc<Zeroizing<[u8; 32]>>,
    commands: mpsc::Sender<Command>,
) {
    let slots = Arc::new(Semaphore::new(32));
    loop {
        let Ok((stream, _)) = listener.accept().await else {
            break;
        };
        let Ok(permit) = slots.clone().try_acquire_owned() else {
            drop(stream);
            continue;
        };
        let token = token.clone();
        let commands = commands.clone();
        tokio::spawn(async move {
            let _permit = permit;
            let _ = tokio::time::timeout(DEADLINE, handle(stream, token, commands)).await;
        });
    }
}
async fn handle(
    mut stream: UnixStream,
    token: Arc<Zeroizing<[u8; 32]>>,
    commands: mpsc::Sender<Command>,
) -> Result<()> {
    let size = stream.read_u32().await? as usize;
    if size == 0 || size > MAX_REQUEST {
        return Err("IPC frame exceeds limit".into());
    }
    let mut bytes = Zeroizing::new(vec![0; size]);
    stream.read_exact(&mut bytes).await?;
    let request: Request = serde_json::from_slice(&bytes).map_err(|_| "invalid IPC request")?;
    let kind = match request {
        Request::Owner(mut request) => {
            let mut supplied = Zeroizing::new([0; 32]);
            let authenticated = hex::decode_to_slice(&request.token, &mut *supplied).is_ok()
                && bool::from(supplied.ct_eq(&**token));
            if !authenticated {
                return write_response(
                    &mut stream,
                    &error("unauthorized", "Owner authentication required"),
                )
                .await;
            }
            CommandKind::Owner {
                method: std::mem::take(&mut request.method),
                request: std::mem::take(&mut request.request),
            }
        }
        Request::Agent(request) => {
            if request.proof.is_empty()
                || request.proof.len() > agentic_protocol::MAX_DOCUMENT_BYTES * 2
            {
                return Err("agent proof exceeds limit".into());
            }
            let proof = hex::decode(request.proof).map_err(|_| "invalid agent proof encoding")?;
            CommandKind::Agent { proof }
        }
    };
    let (reply, received) = oneshot::channel();
    // One request per socket. EOF or trailing input abandons the queued call.
    let mut trailing = [0];
    tokio::select! {
        biased;
        _ = stream.read(&mut trailing) => return Err("IPC caller disconnected".into()),
        sent = commands.send(Command { kind, reply }) => sent.map_err(|_| "core unavailable")?,
    }
    let response = tokio::select! {
        biased;
        _ = stream.read(&mut trailing) => return Err("IPC caller disconnected".into()),
        response = received => response.map_err(|_| "core unavailable")?,
    };
    write_response(&mut stream, &response).await
}
async fn write_response(stream: &mut UnixStream, response: &Value) -> Result<()> {
    let mut bytes = serde_json::to_vec(response)?;
    if bytes.len() > MAX_RESPONSE {
        bytes = serde_json::to_vec(&error("response_too_large", "Use paginated history"))?;
    }
    stream.write_u32(bytes.len() as u32).await?;
    stream.write_all(&bytes).await?;
    stream.shutdown().await?;
    Ok(())
}
/// Local owner client shared by native shell/CLI. Never pass this owner token to agent MCP clients.
pub async fn call(path: &Path, token: &[u8; 32], method: &str, request: Value) -> Result<Value> {
    exchange(
        path,
        json!({"token":hex::encode(token),"method":method,"request":request}),
    )
    .await
}
/// Agent client requires only its signed proof; no owner credentials are accepted here.
pub async fn call_agent(path: &Path, proof: &[u8]) -> Result<Value> {
    if proof.is_empty() || proof.len() > agentic_protocol::MAX_DOCUMENT_BYTES {
        return Err("agent proof exceeds limit".into());
    }
    exchange(path, json!({"proof":hex::encode(proof)})).await
}
async fn exchange(path: &Path, mut payload: Value) -> Result<Value> {
    let bytes = Zeroizing::new(serde_json::to_vec(&payload)?);
    if let Some(Value::String(token)) = payload.get_mut("token") {
        token.zeroize();
    }
    drop(payload);
    if bytes.len() > MAX_REQUEST {
        return Err("IPC request exceeds limit".into());
    }
    tokio::time::timeout(DEADLINE, async {
        let mut stream = UnixStream::connect(path).await?;
        stream.write_u32(bytes.len() as u32).await?;
        stream.write_all(&bytes).await?;
        let size = stream.read_u32().await? as usize;
        if size > MAX_RESPONSE {
            return Err("IPC response exceeds limit".into());
        }
        let mut bytes = vec![0; size];
        stream.read_exact(&mut bytes).await?;
        Ok(serde_json::from_slice(&bytes)?)
    })
    .await
    .map_err(|_| "IPC request timed out")?
}
