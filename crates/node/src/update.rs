//! Replacing this app with a build of its latest release (the release a
//! signed network preset names, network_preset.rs): the command line as
//! install.sh left it, or the macOS app bundle. Only the archive whose
//! SHA-256 the preset announces is taken; it replaces the install whole,
//! beside it, so a failure at any step leaves the install as it was.
use crate::network_preset::Build;
use serde::Deserialize;
use sha2::{Digest, Sha256};
use std::{
    fmt, fs,
    io::Write,
    os::unix::fs::{DirBuilderExt, OpenOptionsExt},
    path::{Path, PathBuf},
    process::{Command, Stdio},
    time::Duration,
};

pub type Result<T> = crate::Result<T>;

/// Beside the binaries of an install from the archive: the build it is.
pub const MARKER: &str = "install.json";
/// The files every command-line build has.
const CLI_FILES: [&str; 3] = ["kaiki", "kaiki-agentic-node", MARKER];
/// No build is larger.
const DOWNLOAD_LIMIT: u64 = 1 << 30;
const CONNECT_TIMEOUT: Duration = Duration::from_secs(10);
const DOWNLOAD_TIMEOUT: Duration = Duration::from_secs(15 * 60);

/// What `install.json` says.
#[derive(Debug, Deserialize)]
pub struct Install {
    /// The build's name in a release, such as `cli-macos-arm64`.
    pub build: String,
}

/// Why an update did not happen: `not_updatable`, `download_failed`,
/// `hash_mismatch` or `bad_archive`.
#[derive(Debug)]
pub struct UpdateError {
    pub code: &'static str,
    pub message: String,
}

impl fmt::Display for UpdateError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.message)
    }
}

impl std::error::Error for UpdateError {}

fn refuse(code: &'static str, message: impl Into<String>) -> crate::NodeError {
    Box::new(UpdateError {
        code,
        message: message.into(),
    })
}

fn bad_archive(why: &str) -> crate::NodeError {
    refuse(
        "bad_archive",
        format!("the download is not this app's build: {why}"),
    )
}

/// This computer's platform, as builds are named: `macos-arm64` or
/// `linux-x86_64`; none where there are no builds.
pub fn platform() -> Option<&'static str> {
    if cfg!(all(target_os = "macos", target_arch = "aarch64")) {
        Some("macos-arm64")
    } else if cfg!(all(target_os = "linux", target_arch = "x86_64")) {
        Some("linux-x86_64")
    } else {
        None
    }
}

/// The install from the archive that `exe` runs from, and its build; a
/// binary installed any other way (a cargo build, the one in the app
/// bundle) does not update itself.
pub fn installed_cli(exe: &Path) -> Result<(PathBuf, Install)> {
    let not_ours = || {
        refuse(
            "not_updatable",
            "this kaiki was not installed from kaikichat.com/install.sh; update it the way it was installed",
        )
    };
    let dir = exe.parent().ok_or_else(not_ours)?;
    let install = fs::read(dir.join(MARKER))
        .ok()
        .and_then(|bytes| serde_json::from_slice::<Install>(&bytes).ok())
        .filter(|install| !install.build.is_empty())
        .ok_or_else(not_ours)?;
    Ok((dir.to_owned(), install))
}

/// Replaces the install in `dir` with `build`: an archive of a `kaiki`
/// directory with the binaries and a marker of the same build.
pub async fn install_cli(dir: &Path, build: &Build) -> Result<()> {
    let (_, current) = installed_cli(&dir.join("kaiki"))?;
    replace(dir, "kaiki", build, |new| {
        for name in CLI_FILES {
            if !new.join(name).is_file() {
                return Err(bad_archive(&format!("no {name}")));
            }
        }
        match installed_cli(&new.join("kaiki")) {
            Ok((_, install)) if install.build == current.build => Ok(()),
            _ => Err(bad_archive(&format!("not the build {}", current.build))),
        }
    })
    .await
}

/// The macOS app bundle a binary at `exe` belongs to.
pub fn app_bundle(exe: &Path) -> Option<PathBuf> {
    let macos = exe.parent()?;
    let contents = macos.parent()?;
    let bundle = contents.parent()?;
    (macos.file_name()? == "MacOS"
        && contents.file_name()? == "Contents"
        && bundle.extension()? == "app")
        .then(|| bundle.to_owned())
}

/// Replaces the app bundle that `exe` runs from with `build`: an archive of
/// a bundle of the same name with the same binary. Answers the bundle.
pub async fn install_app(exe: &Path, build: &Build) -> Result<PathBuf> {
    let bundle = app_bundle(exe).ok_or_else(|| {
        refuse(
            "not_updatable",
            "this app does not run from an app bundle; download the new version",
        )
    })?;
    let name = bundle
        .file_name()
        .and_then(|name| name.to_str())
        .ok_or_else(|| refuse("not_updatable", "the app bundle's name is not text"))?;
    let binary = exe.file_name().unwrap_or_default().to_owned();
    replace(&bundle, name, build, |new| {
        if new.join("Contents/MacOS").join(&binary).is_file() {
            Ok(())
        } else {
            Err(bad_archive("no app binary"))
        }
    })
    .await?;
    Ok(bundle)
}

/// Replaces `target` with the directory `entry` of the archive `build`,
/// when `fits` takes it: fetched and unpacked in a directory beside it (the
/// same volume, so the swap is two renames), which is then removed.
async fn replace(
    target: &Path,
    entry: &str,
    build: &Build,
    fits: impl FnOnce(&Path) -> Result<()>,
) -> Result<()> {
    let parent = target
        .parent()
        .ok_or_else(|| refuse("not_updatable", "the install has no parent directory"))?;
    let mut nonce = [0u8; 8];
    getrandom::fill(&mut nonce).map_err(|_| "OS randomness unavailable")?;
    let work = parent.join(format!(".kaiki-update-{}", hex::encode(nonce)));
    fs::DirBuilder::new()
        .mode(0o700)
        .create(&work)
        .map_err(|error| {
            refuse(
                "not_updatable",
                format!("cannot write beside {}: {error}", target.display()),
            )
        })?;
    let result = async {
        let archive = work.join("build.tar.gz");
        download(build, &archive).await?;
        let unpacked = work.join("unpacked");
        fs::create_dir(&unpacked)?;
        let unpacked_ok = Command::new("tar")
            .arg("-xzf")
            .arg(&archive)
            .arg("-C")
            .arg(&unpacked)
            .stdin(Stdio::null())
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .status()?
            .success();
        if !unpacked_ok {
            return Err(bad_archive("it does not unpack"));
        }
        let top: Vec<_> = fs::read_dir(&unpacked)?
            .map(|item| item.map(|item| item.file_name()))
            .collect::<std::io::Result<_>>()?;
        let new = unpacked.join(entry);
        if top.len() != 1 || top[0] != entry || !new.is_dir() {
            return Err(bad_archive(&format!("it does not hold {entry} alone")));
        }
        fits(&new)?;
        let old = work.join("old");
        fs::rename(target, &old)?;
        if let Err(error) = fs::rename(&new, target) {
            let _ = fs::rename(&old, target);
            return Err(error.into());
        }
        Ok(())
    }
    .await;
    if let Err(error) = fs::remove_dir_all(&work) {
        tracing::warn!("{} was not removed: {error}", work.display());
    }
    result
}

/// `build`'s archive into `to`, if it is the one announced.
async fn download(build: &Build, to: &Path) -> Result<()> {
    let failed = |error: &dyn fmt::Display| {
        refuse(
            "download_failed",
            format!("{} could not be downloaded: {error}", build.url),
        )
    };
    let client = reqwest::Client::builder()
        .redirect(reqwest::redirect::Policy::none())
        .connect_timeout(CONNECT_TIMEOUT)
        .timeout(DOWNLOAD_TIMEOUT)
        .build()?;
    let mut response = client
        .get(&build.url)
        .send()
        .await
        .map_err(|error| failed(&error))?;
    if response.status() != reqwest::StatusCode::OK {
        return Err(failed(&response.status()));
    }
    let mut file = fs::OpenOptions::new()
        .write(true)
        .create_new(true)
        .mode(0o600)
        .open(to)?;
    let mut hash = Sha256::new();
    let mut size = 0u64;
    while let Some(chunk) = response.chunk().await.map_err(|error| failed(&error))? {
        size += chunk.len() as u64;
        if size > DOWNLOAD_LIMIT {
            return Err(failed(&"it is larger than any build"));
        }
        hash.update(&chunk);
        file.write_all(&chunk)?;
    }
    file.sync_all()?;
    if hex::encode(hash.finalize()) != build.sha256.to_ascii_lowercase() {
        return Err(refuse(
            "hash_mismatch",
            "the download is not the build the release announced; nothing was changed",
        ));
    }
    Ok(())
}

#[cfg(test)]
#[path = "update_tests.rs"]
mod tests;
