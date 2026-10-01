//! Moving the app where it stays (relocate.rs): which copies are offered the
//! move, and the move itself on real bundles with the system's own tools.
#![allow(clippy::unwrap_used, clippy::expect_used)]
use super::*;

const HOME: &str = "/Users/alice";
const BINARY: &str = "Contents/MacOS/agentic-desktop";

fn offered(exe: &str, mounts: &str, downloaded: bool) -> Option<Unsettled> {
    unsettled(Path::new(exe), Path::new(HOME), mounts, downloaded)
}

/// What `/sbin/mount` lists while macOS runs two downloads from its
/// quarantine folder.
const MOUNTS: &str = "/dev/disk3s1s1 on / (apfs, sealed, local, read-only, journaled)
devfs on /dev (devfs, local, nobrowse)
/dev/disk3s5 on /System/Volumes/Data (apfs, local, journaled, nobrowse, protect, root data)
/Users/alice/Downloads/Other.app on /private/var/folders/sf/1_kf/T/AppTranslocation/0B7D3C11-5E2A-4F6B-9C8D-1A2B3C4D5E6F (nullfs, local, nodev, nosuid, read-only, nobrowse, mounted by alice)
/Users/alice/Downloads/Kaiki Chat-1.app on /private/var/folders/sf/1_kf/T/AppTranslocation/EF5CFAE6-90FC-4984-A545-3E3315966AB2 (nullfs, local, nodev, nosuid, read-only, nobrowse, mounted by alice)
/dev/disk5s1 on /Volumes/Kaiki Chat (apfs, local, nodev, nosuid, read-only, journaled, noowners, quarantine, mounted by alice)
";
const TRANSLOCATED: &str = "/private/var/folders/sf/1_kf/T/AppTranslocation/EF5CFAE6-90FC-4984-A545-3E3315966AB2/d/Kaiki Chat-1.app";

#[test]
fn a_download_macos_runs_from_its_quarantine_folder_is_offered_with_the_owners_copy() {
    let exe = format!("{TRANSLOCATED}/{BINARY}");
    assert_eq!(
        offered(&exe, MOUNTS, true),
        Some(Unsettled {
            bundle: TRANSLOCATED.into(),
            original: Some("/Users/alice/Downloads/Kaiki Chat-1.app".into()),
        })
    );
    // The mounts no longer name it: the move is still offered, nothing goes
    // to the Trash.
    let without: String = MOUNTS
        .lines()
        .filter(|line| !line.contains("Kaiki Chat-1.app"))
        .map(|line| format!("{line}\n"))
        .collect();
    assert_eq!(
        offered(&exe, &without, true),
        Some(Unsettled {
            bundle: TRANSLOCATED.into(),
            original: None,
        })
    );
    // A disk image's app run the same way: the image is not the owner's to
    // empty.
    let from_image = MOUNTS.replace(
        "/Users/alice/Downloads/Kaiki Chat-1.app on",
        "/Volumes/Kaiki Chat/Kaiki Chat.app on",
    );
    assert_eq!(
        offered(&exe, &from_image, true),
        Some(Unsettled {
            bundle: TRANSLOCATED.into(),
            original: None,
        })
    );
}

#[test]
fn downloads_and_disk_images_are_offered_while_installed_apps_and_builds_are_not() {
    let app = |dir: &str| format!("{dir}/Kaiki Chat.app/{BINARY}");
    assert_eq!(
        offered(&app("/Users/alice/Downloads"), MOUNTS, true),
        Some(Unsettled {
            bundle: "/Users/alice/Downloads/Kaiki Chat.app".into(),
            original: Some("/Users/alice/Downloads/Kaiki Chat.app".into()),
        })
    );
    assert_eq!(
        offered(&app("/Volumes/Kaiki Chat"), MOUNTS, false),
        Some(Unsettled {
            bundle: "/Volumes/Kaiki Chat/Kaiki Chat.app".into(),
            original: None,
        })
    );
    for (exe, downloaded) in [
        (app("/Applications"), true),
        (app("/Users/alice/Applications"), true),
        // A copy the owner built or made: no download's quarantine.
        (
            app("/Users/alice/Code/chat/target/release/bundle/macos"),
            false,
        ),
        (
            app("/Volumes/WD4000/Code2/kaikichat/target/release/bundle/macos"),
            false,
        ),
        // Not an app bundle at all.
        (
            "/Users/alice/Code/chat/target/debug/agentic-desktop".to_owned(),
            true,
        ),
    ] {
        assert_eq!(offered(&exe, MOUNTS, downloaded), None, "{exe}");
    }
}

#[cfg(target_os = "macos")]
mod on_macos {
    use super::*;
    use std::{fs, os::unix::fs::PermissionsExt, process::Command};
    use tempfile::TempDir;

    /// The items the move sent to the Trash; with `refuse`, the Trash says
    /// no (the owner declined access to Downloads).
    #[derive(Default)]
    struct Bin {
        taken: std::sync::Mutex<Vec<PathBuf>>,
        refuse: bool,
    }

    impl Trash for Bin {
        fn trash(&self, path: &Path) -> io::Result<()> {
            if self.refuse {
                return Err(io::Error::from(io::ErrorKind::PermissionDenied));
            }
            self.taken.lock().unwrap().push(path.to_owned());
            Ok(())
        }
    }

    /// An app bundle `name` of `version` in `dir`, quarantined as a browser
    /// leaves a download.
    fn download(dir: &Path, name: &str, version: &str) -> PathBuf {
        let bundle = dir.join(name);
        fs::create_dir_all(bundle.join("Contents/MacOS")).unwrap();
        fs::create_dir_all(bundle.join("Contents/Resources")).unwrap();
        fs::write(
            bundle.join("Contents/Info.plist"),
            format!(
                r#"<?xml version="1.0" encoding="UTF-8"?>
<!DOCTYPE plist PUBLIC "-//Apple//DTD PLIST 1.0//EN" "http://www.apple.com/DTDs/PropertyList-1.0.dtd">
<plist version="1.0"><dict>
<key>CFBundleExecutable</key><string>agentic-desktop</string>
<key>CFBundleIdentifier</key><string>net.agenticinternet.desktop</string>
<key>CFBundleShortVersionString</key><string>{version}</string>
</dict></plist>
"#
            ),
        )
        .unwrap();
        fs::write(bundle.join(BINARY), format!("app {version}")).unwrap();
        fs::set_permissions(bundle.join(BINARY), fs::Permissions::from_mode(0o755)).unwrap();
        for item in [&bundle, &bundle.join(BINARY)] {
            let marked = Command::new("/usr/bin/xattr")
                .args(["-w", "com.apple.quarantine", "0081;6abd7065;Chrome;"])
                .arg(item)
                .status()
                .unwrap();
            assert!(marked.success());
        }
        bundle
    }

    fn quarantined(item: &Path) -> bool {
        Command::new("/usr/bin/xattr")
            .args(["-p", "com.apple.quarantine"])
            .arg(item)
            .output()
            .unwrap()
            .status
            .success()
    }

    fn listing(dir: &Path) -> Vec<String> {
        let mut names: Vec<String> = fs::read_dir(dir)
            .unwrap()
            .map(|item| item.unwrap().file_name().to_string_lossy().into_owned())
            .collect();
        names.sort();
        names
    }

    fn binary(bundle: &Path) -> String {
        fs::read_to_string(bundle.join(BINARY)).unwrap()
    }

    fn read_only(dir: &Path, on: bool) {
        fs::set_permissions(
            dir,
            fs::Permissions::from_mode(if on { 0o555 } else { 0o755 }),
        )
        .unwrap();
    }

    /// A home with a Downloads folder, and the system's Applications folder.
    fn system() -> (TempDir, PathBuf, PathBuf) {
        let dir = tempfile::tempdir().unwrap();
        let downloads = dir.path().join("home/Downloads");
        let applications = dir.path().join("Applications");
        fs::create_dir_all(&downloads).unwrap();
        fs::create_dir_all(&applications).unwrap();
        (dir, downloads, applications)
    }

    fn folders(dir: &TempDir) -> Vec<PathBuf> {
        vec![
            dir.path().join("Applications"),
            dir.path().join("home/Applications"),
        ]
    }

    /// A download run where it lies: it is its own original.
    fn in_place(bundle: &Path) -> Unsettled {
        Unsettled {
            bundle: bundle.to_owned(),
            original: Some(bundle.to_owned()),
        }
    }

    #[test]
    fn here_reads_the_bundles_quarantine() {
        let (dir, downloads, _) = system();
        let bundle = download(&downloads, "Kaiki Chat.app", "0.2.3");
        let home = dir.path().join("home");
        assert_eq!(here(&bundle.join(BINARY), &home), Some(in_place(&bundle)));
        let cleared = Command::new("/usr/bin/xattr")
            .args(["-r", "-d", "com.apple.quarantine"])
            .arg(&bundle)
            .status()
            .unwrap();
        assert!(cleared.success());
        assert_eq!(here(&bundle.join(BINARY), &home), None);
    }

    #[test]
    fn a_download_run_from_the_quarantine_folder_moves_into_applications_and_its_original_goes_to_the_trash()
     {
        let (dir, downloads, applications) = system();
        // macOS runs the download from a read-only copy elsewhere.
        let mount = dir
            .path()
            .join("AppTranslocation/EF5CFAE6-90FC-4984-A545-3E3315966AB2/d");
        let running = download(&mount, "Kaiki Chat-1.app", "0.2.3");
        let original = download(&downloads, "Kaiki Chat-1.app", "0.2.3");
        read_only(&mount, true);
        let bin = Bin::default();
        let moved = move_app(
            &Unsettled {
                bundle: running.clone(),
                original: Some(original.clone()),
            },
            &folders(&dir),
            "Kaiki Chat.app",
            &bin,
        );
        read_only(&mount, false);
        let moved = moved.unwrap();
        assert_eq!(moved, applications.join("Kaiki Chat.app"));
        assert_eq!(binary(&moved), "app 0.2.3");
        assert_eq!(
            fs::metadata(moved.join(BINARY))
                .unwrap()
                .permissions()
                .mode()
                & 0o777,
            0o755
        );
        // Opened from there, macOS runs it in place, not from its
        // quarantine folder again.
        assert!(!quarantined(&moved));
        assert!(!quarantined(&moved.join(BINARY)));
        assert_eq!(listing(&applications), ["Kaiki Chat.app"]);
        assert!(!dir.path().join("home/Applications").exists());
        assert_eq!(*bin.taken.lock().unwrap(), [original]);
        // The running copy is left as macOS made it.
        assert_eq!(binary(&running), "app 0.2.3");
        assert!(quarantined(&running));
    }

    #[test]
    fn without_a_say_in_the_system_folder_an_app_from_a_disk_image_goes_to_the_home_one() {
        let (dir, _, applications) = system();
        let image = dir.path().join("Volumes/Kaiki Chat");
        let bundle = download(&image, "Kaiki Chat.app", "0.2.3");
        read_only(&applications, true);
        let bin = Bin::default();
        let moved = move_app(
            &Unsettled {
                bundle,
                original: None,
            },
            &folders(&dir),
            "Kaiki Chat.app",
            &bin,
        );
        read_only(&applications, false);
        let moved = moved.unwrap();
        let home_applications = dir.path().join("home/Applications");
        assert_eq!(moved, home_applications.join("Kaiki Chat.app"));
        assert_eq!(binary(&moved), "app 0.2.3");
        assert_eq!(listing(&home_applications), ["Kaiki Chat.app"]);
        assert!(listing(&applications).is_empty());
        // Nothing on the image goes to the Trash.
        assert!(bin.taken.lock().unwrap().is_empty());
    }

    #[test]
    fn an_older_copy_there_is_replaced_whole_and_a_newer_one_is_kept() {
        let (dir, downloads, applications) = system();
        let installed = download(&applications, "Kaiki Chat.app", "0.2.9");
        fs::write(installed.join("Contents/Resources/only-in-0.2.9"), "old").unwrap();
        let newer = download(&downloads, "Kaiki Chat.app", "0.2.10");
        // The owner declines access to Downloads: the app is moved all the
        // same, the download stays.
        let refusing = Bin {
            refuse: true,
            ..Bin::default()
        };
        let moved = move_app(
            &in_place(&newer),
            &folders(&dir),
            "Kaiki Chat.app",
            &refusing,
        )
        .unwrap();
        assert_eq!(moved, installed);
        assert_eq!(binary(&installed), "app 0.2.10");
        assert!(!installed.join("Contents/Resources/only-in-0.2.9").exists());
        assert_eq!(listing(&applications), ["Kaiki Chat.app"]);
        assert_eq!(binary(&newer), "app 0.2.10");

        // An old download opened later: the newer app there is the one to
        // open, and the old copy goes.
        let older = download(&downloads, "Kaiki Chat-2.app", "0.2.9");
        let bin = Bin::default();
        let moved = move_app(&in_place(&older), &folders(&dir), "Kaiki Chat.app", &bin).unwrap();
        assert_eq!(moved, installed);
        assert_eq!(binary(&installed), "app 0.2.10");
        assert_eq!(listing(&applications), ["Kaiki Chat.app"]);
        assert_eq!(*bin.taken.lock().unwrap(), [older]);
    }

    #[test]
    fn a_copy_that_fails_leaves_the_installed_app_and_the_download_as_they_were() {
        let (dir, downloads, applications) = system();
        let installed = download(&applications, "Kaiki Chat.app", "0.2.1");
        let bundle = download(&downloads, "Kaiki Chat.app", "0.2.3");
        // A part of the download cannot be read.
        let unreadable = bundle.join("Contents/Resources/icon.icns");
        fs::write(&unreadable, "icon").unwrap();
        fs::set_permissions(&unreadable, fs::Permissions::from_mode(0o000)).unwrap();
        let bin = Bin::default();
        let moved = move_app(&in_place(&bundle), &folders(&dir), "Kaiki Chat.app", &bin);
        fs::set_permissions(&unreadable, fs::Permissions::from_mode(0o644)).unwrap();
        assert!(moved.is_err());
        assert_eq!(binary(&installed), "app 0.2.1");
        assert_eq!(listing(&applications), ["Kaiki Chat.app"]);
        assert!(bin.taken.lock().unwrap().is_empty());
        assert_eq!(binary(&bundle), "app 0.2.3");
    }
}
