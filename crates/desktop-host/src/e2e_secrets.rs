//! Isolated automation profiles only. Normal builds use the system Keychain.
use crate::{Result, SecretStore};
use sha2::{Digest, Sha256};
use std::{
    fs::{self, File, OpenOptions},
    io::{Read, Write},
    os::unix::fs::{DirBuilderExt, MetadataExt, OpenOptionsExt, PermissionsExt},
    path::{Path, PathBuf},
};
use zeroize::Zeroizing;

pub struct E2eFileStore {
    directory: PathBuf,
    account: String,
}

fn private_directory(path: &Path) -> Result<()> {
    let metadata = fs::symlink_metadata(path)?;
    if !metadata.is_dir() || metadata.permissions().mode() & 0o077 != 0 {
        return Err("E2E profile directory must be private".into());
    }
    Ok(())
}

impl E2eFileStore {
    pub fn new(directory: &Path) -> Result<Self> {
        fs::DirBuilder::new()
            .recursive(true)
            .mode(0o700)
            .create(directory)?;
        private_directory(directory)?;
        let directory = directory.canonicalize()?;
        let profile = directory.join("profile.db");
        let account = hex::encode(Sha256::digest(profile.as_os_str().as_encoded_bytes()));
        Ok(Self { directory, account })
    }

    fn path(&self, account: &str) -> Result<PathBuf> {
        if account != self.account {
            return Err("E2E secret belongs to another profile".into());
        }
        private_directory(&self.directory)?;
        Ok(self.directory.join(".e2e-secrets"))
    }
}

impl SecretStore for E2eFileStore {
    fn get(&self, account: &str) -> Result<Option<Zeroizing<Vec<u8>>>> {
        let path = self.path(account)?;
        let before = match fs::symlink_metadata(&path) {
            Ok(value) => value,
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => return Ok(None),
            Err(error) => return Err(error.into()),
        };
        if !before.is_file() || before.len() != 64 || before.permissions().mode() & 0o077 != 0 {
            return Err("invalid or nonprivate E2E secret file".into());
        }
        let file = File::open(&path)?;
        let opened = file.metadata()?;
        if (before.dev(), before.ino()) != (opened.dev(), opened.ino()) {
            return Err("E2E secret file changed while opening".into());
        }
        let mut bytes = Zeroizing::new(Vec::new());
        file.take(65).read_to_end(&mut bytes)?;
        if bytes.len() != 64 {
            return Err("invalid E2E secret length".into());
        }
        Ok(Some(bytes))
    }

    fn set(&self, account: &str, secret: &[u8]) -> Result<()> {
        let path = self.path(account)?;
        if secret.len() != 64 {
            return Err("invalid E2E secret length".into());
        }
        let mut file = OpenOptions::new()
            .write(true)
            .create_new(true)
            .mode(0o600)
            .open(path)?;
        file.write_all(secret)?;
        file.sync_all()?;
        File::open(&self.directory)?.sync_all()?;
        Ok(())
    }
}
