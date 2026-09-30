//! Small machine-readable messaging interface over the shared signed runtime client.
use crate::{ipc, runtime_client::RuntimeClient};
use agentic_core::{AgentFailure, CoreError};
use clap::{Parser, Subcommand, error::ErrorKind};
use serde_json::{Value, json};
use std::{
    io::{Read, Write},
    path::PathBuf,
};

#[derive(Parser)]
#[command(version, about = "Scoped Agentic Internet messaging CLI")]
struct Cli {
    #[arg(long)]
    credentials: PathBuf,
    #[command(subcommand)]
    command: Command,
}
#[derive(Subcommand)]
enum Command {
    /// Show your public identity, granted contacts and current permissions.
    Context,
    Messages {
        #[command(subcommand)]
        command: Messages,
    },
    Delivery {
        #[command(subcommand)]
        command: Delivery,
    },
    Inbox {
        #[command(subcommand)]
        command: Inbox,
    },
}
#[derive(Subcommand)]
enum Messages {
    /// Send text to an allowed contact; retain the operation ID for retries.
    Send {
        #[arg(long)]
        to: String,
        #[arg(long)]
        operation_id: String,
        /// Read up to 12000 bytes of UTF-8 text from stdin.
        #[arg(long, required = true)]
        text_stdin: bool,
    },
}
#[derive(Subcommand)]
enum Delivery {
    /// Inspect your own send operation. Recipient delivery is independent of storage.
    Get {
        #[arg(long)]
        to: String,
        #[arg(long)]
        operation_id: String,
    },
}
#[derive(Subcommand)]
enum Inbox {
    /// Lease a bounded page; acknowledge after processing, including empty scan pages.
    Poll {
        #[arg(long)]
        from: String,
        #[arg(long)]
        operation_id: String,
        #[arg(long, default_value_t = 10)]
        limit: u32,
        #[arg(long, default_value_t = 4096)]
        max_bytes: u64,
        #[arg(long, default_value_t = 30)]
        lease_seconds: u64,
    },
    /// Acknowledge a processed page; repeating the same lease ID is safe.
    Ack {
        #[arg(long)]
        from: String,
        #[arg(long)]
        lease_id: String,
    },
}

struct Response {
    value: Value,
    code: u8,
}
impl Response {
    fn local(code: &str, message: &'static str) -> Self {
        Self {
            value: json!({"error":{"code":code,"message":message,"retryable":false}}),
            code: 2,
        }
    }
    fn invalid() -> Self {
        Self::local(
            "invalid_request",
            "Invalid command or input; use --help for the command contract",
        )
    }
    fn unavailable() -> Self {
        Self {
            value: ipc::agent_failure(AgentFailure::unavailable()),
            code: 4,
        }
    }
    fn domain(value: Value) -> Self {
        if value.as_object().is_some_and(|object| object.len() == 1) {
            if value.get("result").is_some() {
                return Self { value, code: 0 };
            }
            let error = &value["error"];
            if error["code"].as_str().is_some_and(|code| !code.is_empty())
                && error["message"]
                    .as_str()
                    .is_some_and(|message| !message.is_empty())
                && let Some(retryable) = error["retryable"].as_bool()
            {
                return Self {
                    value,
                    code: if retryable { 4 } else { 3 },
                };
            }
        }
        Self::unavailable()
    }
    fn write(self) -> u8 {
        // No credential, parser or transport details are written to diagnostics.
        if writeln!(std::io::stdout().lock(), "{}", self.value).is_err() {
            4
        } else {
            self.code
        }
    }
}

async fn invoke(client: &RuntimeClient, method: &str, request: Value) -> Response {
    match client.invoke(method, request).await {
        Ok(value) => Response::domain(value),
        Err(_) => Response::unavailable(),
    }
}

/// A message read from stdin: a here-document ends with a line break that
/// is not the message's, so one final line break goes.
pub fn message_text(mut text: String) -> String {
    if text.ends_with('\n') {
        text.pop();
        if text.ends_with('\r') {
            text.pop();
        }
    }
    text
}

fn read_text() -> Result<String, Response> {
    let mut bytes = Vec::new();
    std::io::stdin()
        .lock()
        .take(12001)
        .read_to_end(&mut bytes)
        .map_err(|_| Response::invalid())?;
    if bytes.len() > 12000 {
        return Err(Response::invalid());
    }
    let text = message_text(String::from_utf8(bytes).map_err(|_| Response::invalid())?);
    if text.trim().is_empty() {
        return Err(Response::invalid());
    }
    Ok(text)
}

async fn execute(cli: Cli) -> Response {
    let client = match RuntimeClient::load(&cli.credentials) {
        Ok(client) => client,
        Err(_) => {
            return Response::local(
                "invalid_credentials",
                "A valid private runtime credential file is required",
            );
        }
    };
    let (peer, method, mut request) = match cli.command {
        Command::Context => return invoke(&client, "runtime_context", json!({})).await,
        Command::Messages {
            command:
                Messages::Send {
                    to,
                    operation_id,
                    text_stdin: _,
                },
        } => {
            let text = match read_text() {
                Ok(text) => text,
                Err(error) => return error,
            };
            (
                to,
                "send_message",
                json!({"operationId":operation_id,"text":text}),
            )
        }
        Command::Delivery {
            command: Delivery::Get { to, operation_id },
        } => (to, "delivery_get", json!({"operationId":operation_id})),
        Command::Inbox {
            command:
                Inbox::Poll {
                    from,
                    operation_id,
                    limit,
                    max_bytes,
                    lease_seconds,
                },
        } => (
            from,
            "inbox_poll",
            json!({"operationId":operation_id,"limit":limit,"maxBytes":max_bytes,"leaseSeconds":lease_seconds}),
        ),
        Command::Inbox {
            command: Inbox::Ack { from, lease_id },
        } => (from, "inbox_ack", json!({"leaseId":lease_id})),
    };
    if !peer.strip_prefix("ain1").is_some_and(|key| {
        key.len() == 64
            && key
                .bytes()
                .all(|c| c.is_ascii_digit() || (b'a'..=b'f').contains(&c))
    }) {
        return Response::invalid();
    }
    let context = invoke(&client, "runtime_context", json!({})).await;
    if context.code != 0 {
        return context;
    }
    let Some(contacts) = context.value["result"]["conversations"].as_array() else {
        return Response::unavailable();
    };
    let mut matching = contacts
        .iter()
        .filter(|contact| contact["networkId"].as_str() == Some(&peer));
    let conversation = matching.next().and_then(|contact| contact["id"].as_str());
    // A public ID must name exactly one currently allowed conversation.
    let Some(conversation) = conversation.filter(|_| matching.next().is_none()) else {
        return Response::domain(ipc::agent_failure(CoreError::Unauthorized.agent_failure()));
    };
    request["conversationId"] = json!(conversation);
    invoke(&client, method, request).await
}

pub async fn run() -> u8 {
    match Cli::try_parse() {
        Ok(cli) => execute(cli).await.write(),
        Err(error)
            if matches!(
                error.kind(),
                ErrorKind::DisplayHelp | ErrorKind::DisplayVersion
            ) =>
        {
            if write!(std::io::stdout().lock(), "{error}").is_err() {
                4
            } else {
                0
            }
        }
        Err(_) => Response::invalid().write(),
    }
}
