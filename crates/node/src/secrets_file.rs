//! The profile secret sealed in a file with a password (spec/owner-cli-v1.md),
//! for machines without a keychain: Argon2id of the password with a random
//! salt keys XChaCha20-Poly1305. A wrong password and a changed file look
//! the same: the secret does not open.
use crate::host::SecretStore;
use chacha20poly1305::aead::{Aead, KeyInit};
use chacha20poly1305::{XChaCha20Poly1305, XNonce};
use serde::{Deserialize, Serialize};
use std::fs::{self, OpenOptions};
use std::io::Write;
use std::os::unix::fs::OpenOptionsExt;
use std::path::{Path, PathBuf};
use zeroize::Zeroizing;

/// The secrets file does not open with this password (or was changed).
#[derive(Debug)]
pub struct SecretsLocked;

impl std::fmt::Display for SecretsLocked {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("the secrets file does not open with this password")
    }
}

impl std::error::Error for SecretsLocked {}

#[derive(Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct Sealed {
    version: u8,
    kdf: String,
    salt: String,
    nonce: String,
    ciphertext: String,
}

pub struct PasswordFileStore {
    path: PathBuf,
    password: Zeroizing<String>,
}

impl PasswordFileStore {
    /// `secrets.json` in `data_dir`, opened with `password`.
    pub fn new(data_dir: &Path, password: Zeroizing<String>) -> Self {
        Self {
            path: data_dir.join("secrets.json"),
            password,
        }
    }

    fn cipher(&self, salt: &[u8]) -> crate::Result<XChaCha20Poly1305> {
        let mut key = Zeroizing::new([0; 32]);
        argon2::Argon2::default()
            .hash_password_into(self.password.as_bytes(), salt, key.as_mut())
            .map_err(|error| error.to_string())?;
        Ok(XChaCha20Poly1305::new(key.as_ref().into()))
    }
}

impl SecretStore for PasswordFileStore {
    fn get(&self, _account: &str) -> crate::Result<Option<Zeroizing<Vec<u8>>>> {
        let bytes = match fs::read(&self.path) {
            Ok(bytes) => bytes,
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => return Ok(None),
            Err(error) => return Err(error.into()),
        };
        let sealed: Sealed = serde_json::from_slice(&bytes).map_err(|_| SecretsLocked)?;
        if sealed.version != 1 || sealed.kdf != "argon2id" {
            return Err(SecretsLocked.into());
        }
        let decode = |text: &str| hex::decode(text).map_err(|_| SecretsLocked);
        let (salt, nonce, ciphertext) = (
            decode(&sealed.salt)?,
            decode(&sealed.nonce)?,
            decode(&sealed.ciphertext)?,
        );
        if nonce.len() != 24 {
            return Err(SecretsLocked.into());
        }
        let secret = self
            .cipher(&salt)?
            .decrypt(XNonce::from_slice(&nonce), ciphertext.as_slice())
            .map_err(|_| SecretsLocked)?;
        Ok(Some(Zeroizing::new(secret)))
    }

    fn set(&self, _account: &str, secret: &[u8]) -> crate::Result<()> {
        let mut salt = [0; 16];
        let mut nonce = [0; 24];
        getrandom::fill(&mut salt).map_err(|_| "OS randomness unavailable")?;
        getrandom::fill(&mut nonce).map_err(|_| "OS randomness unavailable")?;
        let ciphertext = self
            .cipher(&salt)?
            .encrypt(XNonce::from_slice(&nonce), secret)
            .map_err(|_| "sealing the secret failed")?;
        let sealed = serde_json::to_vec(&Sealed {
            version: 1,
            kdf: "argon2id".into(),
            salt: hex::encode(salt),
            nonce: hex::encode(nonce),
            ciphertext: hex::encode(ciphertext),
        })?;
        let temporary = self.path.with_extension("json.tmp");
        let mut file = OpenOptions::new()
            .write(true)
            .create(true)
            .truncate(true)
            .mode(0o600)
            .open(&temporary)?;
        file.write_all(&sealed)?;
        file.sync_all()?;
        fs::rename(&temporary, &self.path)?;
        Ok(())
    }
}
