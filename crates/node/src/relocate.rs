//! Moving the macOS app where it stays (spec/desktop-gui-v1.md, "Start at
//! login"): a copy run from a download, a disk image or the quarantine
//! folder macOS runs downloads from cannot open at login, and its command
//! line is gone once it quits. One press puts it into the Applications
//! folder, without the download's quarantine, and the owner's download goes
//! to the Trash.
use std::{
    ffi::OsStr,
    fs, io,
    os::unix::{fs::DirBuilderExt, process::CommandExt},
    path::{Path, PathBuf},
    process::{Command, Stdio},
};

pub type Result<T> = crate::Result<T>;

/// The folder macOS runs a quarantined download from: a read-only mount of
/// the download at `AppTranslocation/<id>`, the app inside it.
const TRANSLOCATION: &str = "AppTranslocation";

/// An app bundle that runs from a place it does not stay in.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Unsettled {
    /// The bundle this process runs from.
    pub bundle: PathBuf,
    /// The owner's own copy, which goes to the Trash once the app moved: a
    /// download in the home folder; none on a disk image.
    pub original: Option<PathBuf>,
}

/// The app at `exe`, when it runs from where it does not stay. `mounts` is
/// what `/sbin/mount` lists; `downloaded`, whether the bundle carries the
/// quarantine of a download. A copy in the home folder without it is one the
/// owner built or made, and stays where it is.
pub fn unsettled(exe: &Path, home: &Path, mounts: &str, downloaded: bool) -> Option<Unsettled> {
    let bundle = crate::update::app_bundle(exe)?;
    if bundle.starts_with("/Applications") || bundle.starts_with(home.join("Applications")) {
        return None;
    }
    if let Some(mount) = bundle
        .ancestors()
        .find(|dir| dir.parent().and_then(Path::file_name) == Some(OsStr::new(TRANSLOCATION)))
    {
        let original =
            translocated_from(mounts, mount).filter(|original| original.starts_with(home));
        return Some(Unsettled { bundle, original });
    }
    if bundle.parent().and_then(Path::parent) == Some(Path::new("/Volumes")) {
        return Some(Unsettled {
            bundle,
            original: None,
        });
    }
    (downloaded && bundle.starts_with(home)).then(|| Unsettled {
        original: Some(bundle.clone()),
        bundle,
    })
}

/// What macOS mounted at `mount`: `<source> on <mount> (<options>)`.
fn translocated_from(mounts: &str, mount: &Path) -> Option<PathBuf> {
    let at = format!(" on {} (", mount.display());
    mounts.lines().find_map(|line| {
        line.split_once(&at)
            .map(|(source, _)| PathBuf::from(source))
    })
}

/// `unsettled` for this system: its mounts and the bundle's quarantine.
pub fn here(exe: &Path, home: &Path) -> Option<Unsettled> {
    let bundle = crate::update::app_bundle(exe)?;
    let mounts = Command::new("/sbin/mount")
        .stdin(Stdio::null())
        .stderr(Stdio::null())
        .output()
        .map(|output| String::from_utf8_lossy(&output.stdout).into_owned())
        .unwrap_or_default();
    let downloaded = Command::new("/usr/bin/xattr")
        .args(["-p", "com.apple.quarantine"])
        .arg(&bundle)
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .status()
        .is_ok_and(|status| status.success());
    unsettled(exe, home, &mounts, downloaded)
}

/// Where a moved item goes: the owner can still take it back.
pub trait Trash: Send + Sync {
    fn trash(&self, path: &Path) -> io::Result<()>;
}

/// The owner's Trash, through Foundation's own call (no Finder, so no
/// automation prompt).
pub struct SystemTrash;

impl Trash for SystemTrash {
    fn trash(&self, path: &Path) -> io::Result<()> {
        const SCRIPT: &str = "ObjC.import('Foundation');\
            function run(argv){return $.NSFileManager.defaultManager\
            .trashItemAtURLResultingItemURLError($.NSURL.fileURLWithPath(argv[0]),null,null)?'trashed':'kept';}";
        let output = Command::new("/usr/bin/osascript")
            .args(["-l", "JavaScript", "-e", SCRIPT])
            .arg(path)
            .stdin(Stdio::null())
            .stderr(Stdio::null())
            .output()?;
        if output.status.success() && output.stdout.trim_ascii() == b"trashed" {
            Ok(())
        } else {
            Err(io::Error::other(format!(
                "{} did not go to the Trash",
                path.display()
            )))
        }
    }
}

/// Puts `app` into the first of `folders` that takes it, as `name`, and
/// answers the bundle to open: a copy without the quarantine, swapped in
/// whole; a newer app already there stays and is the one to open. The
/// owner's original then goes to the Trash; when it does not, the app is
/// moved all the same.
pub fn move_app(
    app: &Unsettled,
    folders: &[PathBuf],
    name: &str,
    trash: &dyn Trash,
) -> Result<PathBuf> {
    let mut refusals = Vec::new();
    let mut moved = None;
    for folder in folders {
        match put(&app.bundle, folder, name) {
            Ok(target) => {
                moved = Some(target);
                break;
            }
            Err(Put::Closed(error)) => refusals.push(format!("{}: {error}", folder.display())),
            Err(Put::Failed(error)) => return Err(error),
        }
    }
    let moved = moved.ok_or_else(|| {
        format!(
            "no Applications folder takes the app: {}",
            refusals.join("; ")
        )
    })?;
    if let Some(original) = &app.original
        && let Err(error) = trash.trash(original)
    {
        tracing::warn!("{} stays where it is: {error}", original.display());
    }
    Ok(moved)
}

enum Put {
    /// The folder does not take it: the next one may.
    Closed(io::Error),
    /// The copy itself failed: nothing changed.
    Failed(crate::NodeError),
}

/// `bundle` into `folder` as `name`, through a work directory beside it (the
/// same volume, so the swap is two renames), which is then removed.
fn put(bundle: &Path, folder: &Path, name: &str) -> std::result::Result<PathBuf, Put> {
    let target = folder.join(name);
    if fs::symlink_metadata(&target).is_ok()
        && let (Some(there), Some(ours)) = (version(&target), version(bundle))
        && there > ours
    {
        return Ok(target);
    }
    fs::create_dir_all(folder).map_err(Put::Closed)?;
    let mut nonce = [0u8; 8];
    getrandom::fill(&mut nonce).map_err(|_| Put::Failed("OS randomness unavailable".into()))?;
    let work = folder.join(format!(".kaiki-move-{}", hex::encode(nonce)));
    fs::DirBuilder::new()
        .mode(0o700)
        .create(&work)
        .map_err(Put::Closed)?;
    let result = swap_in(bundle, &work, &target);
    if let Err(error) = fs::remove_dir_all(&work) {
        tracing::warn!("{} was not removed: {error}", work.display());
    }
    result.map(|()| target).map_err(Put::Failed)
}

fn swap_in(bundle: &Path, work: &Path, target: &Path) -> Result<()> {
    let new = work.join("new.app");
    // ditto keeps the signature and the stapled ticket; the quarantine it
    // keeps too (`--noqtn` notwithstanding) is taken off every item.
    let mut ditto = Command::new("/usr/bin/ditto");
    ditto.arg(bundle).arg(&new);
    let mut xattr = Command::new("/usr/bin/xattr");
    xattr.args(["-r", "-d", "com.apple.quarantine"]).arg(&new);
    for mut step in [ditto, xattr] {
        let done = step
            .stdin(Stdio::null())
            .stdout(Stdio::null())
            .stderr(Stdio::piped())
            .output()?;
        if !done.status.success() {
            return Err(format!(
                "{} could not be copied: {}",
                bundle.display(),
                String::from_utf8_lossy(&done.stderr).trim()
            )
            .into());
        }
    }
    if fs::symlink_metadata(target).is_ok() {
        let old = work.join("old.app");
        fs::rename(target, &old)?;
        if let Err(error) = fs::rename(&new, target) {
            let _ = fs::rename(&old, target);
            return Err(error.into());
        }
    } else {
        fs::rename(&new, target)?;
    }
    Ok(())
}

/// The bundle's version, `X.Y.Z` from its `Info.plist`.
fn version(bundle: &Path) -> Option<(u64, u64, u64)> {
    let output = Command::new("/usr/bin/plutil")
        .args(["-extract", "CFBundleShortVersionString", "raw", "-o", "-"])
        .arg(bundle.join("Contents/Info.plist"))
        .stdin(Stdio::null())
        .stderr(Stdio::null())
        .output()
        .ok()?;
    if !output.status.success() {
        return None;
    }
    crate::network_preset::version_of(String::from_utf8_lossy(&output.stdout).trim()).ok()
}

/// Opens `bundle` once this process has quit: the moved app starts from its
/// new place, with its own daemon and its place at login.
pub fn open_after_exit(bundle: &Path) -> io::Result<()> {
    Command::new("/bin/sh")
        .arg("-c")
        .arg("while /bin/kill -0 \"$1\" 2>/dev/null; do /bin/sleep 0.2; done; exec /usr/bin/open \"$2\"")
        .arg("sh")
        .arg(std::process::id().to_string())
        .arg(bundle)
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .process_group(0)
        .spawn()
        .map(drop)
}

#[cfg(test)]
#[path = "relocate_tests.rs"]
mod tests;
