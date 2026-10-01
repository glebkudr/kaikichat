//! Replacing an install with a published build (update.rs) against a local
//! server: only the announced archive replaces it, whole, and nothing else
//! does.
#![allow(clippy::unwrap_used, clippy::expect_used)]
use super::*;
use crate::network_preset::Build;
use sha2::{Digest, Sha256};
use std::{
    collections::{BTreeMap, HashMap},
    fs,
    io::{Read, Write},
    net::TcpListener,
    os::unix::fs::PermissionsExt,
    path::{Path, PathBuf},
    process::Command,
    sync::{Arc, Mutex},
    thread,
};
use tempfile::TempDir;

/// Files by path over HTTP; any other path is 404.
struct Files {
    base: String,
    files: Arc<Mutex<HashMap<String, Vec<u8>>>>,
}

impl Files {
    fn start() -> Self {
        let listener = TcpListener::bind("127.0.0.1:0").unwrap();
        let base = format!("http://{}", listener.local_addr().unwrap());
        let files = Arc::new(Mutex::new(HashMap::<String, Vec<u8>>::new()));
        let served = files.clone();
        thread::spawn(move || {
            for stream in listener.incoming() {
                let Ok(mut stream) = stream else { continue };
                let mut request = Vec::new();
                let mut byte = [0u8; 1];
                while !request.ends_with(b"\r\n\r\n") && stream.read(&mut byte).unwrap_or(0) == 1 {
                    request.push(byte[0]);
                }
                let path = String::from_utf8_lossy(&request)
                    .split_whitespace()
                    .nth(1)
                    .unwrap_or_default()
                    .to_owned();
                let body = served.lock().unwrap().get(&path).cloned();
                let (status, body) = match body {
                    Some(body) => ("200 OK", body),
                    None => ("404 Not Found", Vec::new()),
                };
                let _ = write!(
                    stream,
                    "HTTP/1.1 {status}\r\nContent-Length: {}\r\nConnection: close\r\n\r\n",
                    body.len()
                );
                let _ = stream.write_all(&body);
            }
        });
        Self { base, files }
    }
    /// Serves `bytes` at `path`: its URL.
    fn put(&self, path: &str, bytes: &[u8]) -> String {
        self.files
            .lock()
            .unwrap()
            .insert(path.to_owned(), bytes.to_vec());
        format!("{}{path}", self.base)
    }
}

/// A gzipped tar of the directory `top` holding `files` (path, text);
/// executable when the text starts with `#!`.
fn archive(top: &str, files: &[(&str, &str)]) -> Vec<u8> {
    let staging = TempDir::new().unwrap();
    for (path, text) in files {
        let file = staging.path().join(top).join(path);
        fs::create_dir_all(file.parent().unwrap()).unwrap();
        // `-> target` is a link, as scripts/pack-cli.sh keeps a former name.
        if let Some(target) = text.strip_prefix("-> ") {
            std::os::unix::fs::symlink(target, &file).unwrap();
            continue;
        }
        fs::write(&file, text).unwrap();
        let mode = if text.starts_with("#!") { 0o755 } else { 0o644 };
        fs::set_permissions(&file, fs::Permissions::from_mode(mode)).unwrap();
    }
    let out = staging.path().join("out.tar.gz");
    pack(staging.path(), top, &out);
    fs::read(out).unwrap()
}

/// `tar -czf out -C dir top` as scripts/publish-cli.sh packs it: without
/// macOS metadata entries.
fn pack(dir: &Path, top: &str, out: &Path) {
    let version = Command::new("tar").arg("--version").output().unwrap();
    let mut command = Command::new("tar");
    command.env("COPYFILE_DISABLE", "1");
    if version.stdout.starts_with(b"bsdtar") {
        command.args(["--no-xattrs", "--no-mac-metadata"]);
    }
    let status = command
        .arg("-czf")
        .arg(out)
        .arg("-C")
        .arg(dir)
        .arg(top)
        .status()
        .unwrap();
    assert!(status.success());
}

/// `bytes` served at `path`, announced with their hash.
fn announced(files: &Files, path: &str, bytes: &[u8]) -> Build {
    Build {
        url: files.put(path, bytes),
        sha256: hex::encode(Sha256::digest(bytes)),
    }
}

/// Every file under `dir`: its relative path and text.
fn listing(dir: &Path) -> BTreeMap<String, String> {
    fn walk(root: &Path, dir: &Path, into: &mut BTreeMap<String, String>) {
        for entry in fs::read_dir(dir).unwrap() {
            let path = entry.unwrap().path();
            if path.is_dir() {
                walk(root, &path, into);
            } else {
                let name = path
                    .strip_prefix(root)
                    .unwrap()
                    .to_string_lossy()
                    .into_owned();
                into.insert(name, fs::read_to_string(&path).unwrap_or_default());
            }
        }
    }
    let mut files = BTreeMap::new();
    walk(dir, dir, &mut files);
    files
}

fn names(dir: &Path) -> Vec<String> {
    let mut names: Vec<_> = fs::read_dir(dir)
        .unwrap()
        .map(|entry| entry.unwrap().file_name().into_string().unwrap())
        .collect();
    names.sort();
    names
}

const MARKER_TEXT: &str = r#"{"build":"cli-test"}"#;

/// The command line as install.sh leaves it: `share/kaiki/bin` with the
/// binaries and the marker naming its build.
fn install(root: &TempDir) -> PathBuf {
    let bin = root.path().join("share/kaiki/bin");
    fs::create_dir_all(&bin).unwrap();
    for (name, text) in [
        ("kaiki", "#!/bin/sh\necho old kaiki\n"),
        ("kaiki-agentic-node", "#!/bin/sh\necho old node\n"),
        (MARKER, MARKER_TEXT),
    ] {
        fs::write(bin.join(name), text).unwrap();
    }
    bin
}

fn new_cli() -> Vec<u8> {
    archive(
        "kaiki",
        &[
            ("kaiki", "#!/bin/sh\necho new kaiki\n"),
            ("kaiki-agentic-node", "#!/bin/sh\necho new node\n"),
            ("agentic-node", "-> kaiki-agentic-node"),
            ("agentic-mcp", "#!/bin/sh\necho new mcp\n"),
            (MARKER, MARKER_TEXT),
        ],
    )
}

fn code(error: &crate::NodeError) -> &'static str {
    error
        .downcast_ref::<UpdateError>()
        .unwrap_or_else(|| panic!("not an update refusal: {error}"))
        .code
}

#[tokio::test]
async fn an_install_is_replaced_by_its_announced_build() {
    let files = Files::start();
    let root = TempDir::new().unwrap();
    let bin = install(&root);
    let (dir, marker) = installed_cli(&bin.join("kaiki")).unwrap();
    assert_eq!(
        (fs::canonicalize(dir).unwrap(), marker.build.as_str()),
        (fs::canonicalize(&bin).unwrap(), "cli-test")
    );
    // A link to the old binary: the install is replaced beside it, never
    // written over (a running binary keeps its file).
    fs::hard_link(bin.join("kaiki"), root.path().join("witness")).unwrap();

    let bytes = new_cli();
    let build = announced(&files, "/kaiki-cli-test.tar.gz", &bytes);
    install_cli(&bin, &build).await.unwrap();
    assert_eq!(
        listing(&bin),
        BTreeMap::from([
            (
                "agentic-mcp".to_owned(),
                "#!/bin/sh\necho new mcp\n".to_owned()
            ),
            (
                "agentic-node".to_owned(),
                "#!/bin/sh\necho new node\n".to_owned()
            ),
            (MARKER.to_owned(), MARKER_TEXT.to_owned()),
            ("kaiki".to_owned(), "#!/bin/sh\necho new kaiki\n".to_owned()),
            (
                "kaiki-agentic-node".to_owned(),
                "#!/bin/sh\necho new node\n".to_owned()
            ),
        ])
    );
    assert_eq!(
        fs::metadata(bin.join("kaiki"))
            .unwrap()
            .permissions()
            .mode()
            & 0o111,
        0o111
    );
    assert_eq!(
        fs::read_to_string(root.path().join("witness")).unwrap(),
        "#!/bin/sh\necho old kaiki\n"
    );
    // Nothing is left beside it, and the new install updates again.
    assert_eq!(names(bin.parent().unwrap()), ["bin"]);
    assert_eq!(
        installed_cli(&bin.join("kaiki")).unwrap().1.build,
        "cli-test"
    );
    // A later build without the daemon's former name updates it too: the
    // link is only for kaiki 0.2.4 and older.
    let later = archive(
        "kaiki",
        &[
            ("kaiki", "#!/bin/sh\necho later kaiki\n"),
            ("kaiki-agentic-node", "#!/bin/sh\necho later node\n"),
            (MARKER, MARKER_TEXT),
        ],
    );
    let build = announced(&files, "/kaiki-cli-later.tar.gz", &later);
    install_cli(&bin, &build).await.unwrap();
    assert_eq!(
        fs::read_to_string(bin.join("kaiki-agentic-node")).unwrap(),
        "#!/bin/sh\necho later node\n"
    );
}

#[tokio::test]
async fn a_build_other_than_the_announced_one_changes_nothing() {
    let files = Files::start();
    let root = TempDir::new().unwrap();
    let bin = install(&root);
    let before = listing(&bin);
    let unchanged = |bin: &Path| {
        assert_eq!(listing(bin), before);
        assert_eq!(names(bin.parent().unwrap()), ["bin"]);
    };

    // Other bytes than the hash says.
    let bytes = new_cli();
    let mut build = announced(&files, "/kaiki-cli-test.tar.gz", &bytes);
    build.sha256 = hex::encode(Sha256::digest(b"another build"));
    let error = install_cli(&bin, &build).await.unwrap_err();
    assert_eq!(code(&error), "hash_mismatch");
    unchanged(&bin);

    // Nothing at the address.
    let mut gone = announced(&files, "/kaiki-cli-test.tar.gz", &bytes);
    gone.url = format!("{}/missing.tar.gz", files.base);
    let error = install_cli(&bin, &gone).await.unwrap_err();
    assert_eq!(code(&error), "download_failed");
    unchanged(&bin);

    // The announced archive, but not of this command line: no kaiki
    // directory, one without its binaries or its marker, one packed with the
    // daemon under its former name only (its kaiki would find no daemon), or
    // the build of another platform.
    for bytes in [
        archive("other", &[("kaiki", "#!/bin/sh\n"), (MARKER, MARKER_TEXT)]),
        archive("kaiki", &[("kaiki", "#!/bin/sh\n"), (MARKER, MARKER_TEXT)]),
        archive(
            "kaiki",
            &[
                ("kaiki", "#!/bin/sh\n"),
                ("kaiki-agentic-node", "#!/bin/sh\n"),
            ],
        ),
        archive(
            "kaiki",
            &[
                ("kaiki", "#!/bin/sh\n"),
                ("agentic-node", "#!/bin/sh\n"),
                (MARKER, MARKER_TEXT),
            ],
        ),
        archive(
            "kaiki",
            &[
                ("kaiki", "#!/bin/sh\n"),
                ("kaiki-agentic-node", "#!/bin/sh\n"),
                (MARKER, r#"{"build":"cli-other"}"#),
            ],
        ),
        b"not an archive".to_vec(),
    ] {
        let build = announced(&files, "/odd.tar.gz", &bytes);
        let error = install_cli(&bin, &build).await.unwrap_err();
        assert_eq!(code(&error), "bad_archive");
        unchanged(&bin);
    }
}

#[test]
fn only_an_install_from_the_archive_updates_itself() {
    let root = TempDir::new().unwrap();
    // A build, as cargo leaves it: no marker beside it.
    let build = root.path().join("target/debug");
    fs::create_dir_all(&build).unwrap();
    fs::write(build.join("kaiki"), "#!/bin/sh\n").unwrap();
    let error = installed_cli(&build.join("kaiki")).unwrap_err();
    assert_eq!(code(&error), "not_updatable");
    // A marker that does not name a build.
    fs::write(build.join(MARKER), "{}").unwrap();
    assert_eq!(
        code(&installed_cli(&build.join("kaiki")).unwrap_err()),
        "not_updatable"
    );

    // The app's bundle, from the path of a binary in it.
    assert_eq!(
        app_bundle(Path::new(
            "/Applications/Kaiki Chat.app/Contents/MacOS/kaiki-chat"
        )),
        Some(PathBuf::from("/Applications/Kaiki Chat.app"))
    );
    for elsewhere in [
        "/usr/bin/kaiki-chat",
        "/home/owner/chat/target/debug/agentic-desktop",
    ] {
        assert_eq!(app_bundle(Path::new(elsewhere)), None, "{elsewhere}");
    }
    // Builds are named per platform, as install.sh names them.
    let expected = if cfg!(all(target_os = "macos", target_arch = "aarch64")) {
        Some("macos-arm64")
    } else if cfg!(all(target_os = "linux", target_arch = "x86_64")) {
        Some("linux-x86_64")
    } else {
        None
    };
    assert_eq!(platform(), expected);
}

#[tokio::test]
async fn an_app_bundle_is_replaced_whole() {
    let files = Files::start();
    let root = TempDir::new().unwrap();
    let bundle = root.path().join("Applications/Kaiki Chat.app");
    for (path, text) in [
        ("Contents/Info.plist", "old plist"),
        ("Contents/MacOS/kaiki-chat", "#!/bin/sh\necho old app\n"),
        (
            "Contents/MacOS/kaiki-agentic-node",
            "#!/bin/sh\necho old node\n",
        ),
        ("Contents/Resources/gone.txt", "only in the old app"),
    ] {
        let file = bundle.join(path);
        fs::create_dir_all(file.parent().unwrap()).unwrap();
        fs::write(file, text).unwrap();
    }
    let exe = bundle.join("Contents/MacOS/kaiki-chat");
    let new = [
        ("Contents/Info.plist", "new plist"),
        ("Contents/MacOS/kaiki-chat", "#!/bin/sh\necho new app\n"),
        (
            "Contents/MacOS/kaiki-agentic-node",
            "#!/bin/sh\necho new node\n",
        ),
    ];

    // An archive of another app, or one without this binary, changes nothing.
    let before = listing(&bundle);
    for bytes in [
        archive("Other.app", &new),
        archive("Kaiki Chat.app", &[("Contents/Info.plist", "new plist")]),
    ] {
        let build = announced(&files, "/odd.tar.gz", &bytes);
        let error = install_app(&exe, &build).await.unwrap_err();
        assert_eq!(code(&error), "bad_archive");
        assert_eq!(listing(&bundle), before);
    }

    let build = announced(
        &files,
        "/kaiki-chat-macos.tar.gz",
        &archive("Kaiki Chat.app", &new),
    );
    assert_eq!(
        fs::canonicalize(install_app(&exe, &build).await.unwrap()).unwrap(),
        fs::canonicalize(&bundle).unwrap()
    );
    assert_eq!(
        listing(&bundle),
        new.iter()
            .map(|(path, text)| ((*path).to_owned(), (*text).to_owned()))
            .collect::<BTreeMap<_, _>>()
    );
    assert_eq!(names(bundle.parent().unwrap()), ["Kaiki Chat.app"]);
    let error = install_app(Path::new("/usr/bin/kaiki-chat"), &build)
        .await
        .unwrap_err();
    assert_eq!(code(&error), "not_updatable");
}
