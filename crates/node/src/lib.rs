//! Independent daemon runtime. The application core remains reusable without network or UI.
#[cfg(unix)]
pub mod autostart;
#[cfg(unix)]
pub mod discover;
#[cfg(unix)]
pub mod host;
pub mod ipc;
#[cfg(unix)]
pub mod mcp;
#[cfg(unix)]
pub mod messaging_cli;
#[cfg(unix)]
pub mod network_preset;
#[cfg(unix)]
mod runtime;
#[cfg(unix)]
mod runtime_client;
#[cfg(unix)]
pub mod secrets_file;
#[cfg(unix)]
pub mod update;
#[cfg(unix)]
pub use runtime::{NodeConfig, run};
use serde::Deserialize;
use zeroize::{Zeroize, Zeroizing};

pub type NodeError = Box<dyn std::error::Error + Send + Sync>;
pub type Result<T> = std::result::Result<T, NodeError>;
pub const NETWORK_DOMAIN: [u8; 32] = [
    0xae, 0x2e, 0x31, 0x82, 0xad, 0xe8, 0x17, 0xa3, 0xe7, 0x26, 0xc2, 0x9e, 0xf3, 0x08, 0xee, 0xd1,
    0x5c, 0x6e, 0xc2, 0x67, 0xff, 0xc0, 0x1a, 0x68, 0xda, 0x99, 0x0e, 0x57, 0x6f, 0xa0, 0xdf, 0x18,
];
#[derive(Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct BootstrapInput {
    master_key: String,
    owner_token: String,
}
impl Drop for BootstrapInput {
    fn drop(&mut self) {
        self.master_key.zeroize();
        self.owner_token.zeroize();
    }
}
pub struct Bootstrap {
    pub master_key: Zeroizing<[u8; 32]>,
    pub owner_token: Zeroizing<[u8; 32]>,
}
impl Bootstrap {
    pub fn decode(bytes: &[u8]) -> Result<Self> {
        if bytes.len() > 4096 {
            return Err("bootstrap exceeds size limit".into());
        }
        let input: BootstrapInput =
            serde_json::from_slice(bytes).map_err(|_| "invalid bootstrap input")?;
        fn secret(value: &str) -> Result<Zeroizing<[u8; 32]>> {
            let mut bytes = Zeroizing::new([0; 32]);
            hex::decode_to_slice(value, &mut *bytes)
                .map_err(|_| "bootstrap secret must be 32 bytes in hex")?;
            Ok(bytes)
        }
        Ok(Self {
            master_key: secret(&input.master_key)?,
            owner_token: secret(&input.owner_token)?,
        })
    }
}
