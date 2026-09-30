//! Starting Kaiki Chat at login (autostart.rs): the jobs each system's
//! login manager runs, and the owner's choice, which a later start never
//! overrides.
#![allow(clippy::unwrap_used, clippy::expect_used)]
use super::*;
use std::{
    fs,
    path::{Path, PathBuf},
    sync::{
        Arc,
        atomic::{AtomicBool, AtomicUsize, Ordering},
    },
};
use tempfile::TempDir;

/// The system's login-items settings: whether it blocks a job, and how
/// often the owner was asked to allow one.
#[derive(Default)]
struct Settings {
    blocked: AtomicBool,
    asked: AtomicUsize,
}

impl Settings {
    fn block(&self, blocked: bool) {
        self.blocked.store(blocked, Ordering::SeqCst);
    }
    fn asked(&self) -> usize {
        self.asked.load(Ordering::SeqCst)
    }
}

impl Approval for Settings {
    fn blocked(&self, _job: &Path) -> bool {
        self.blocked.load(Ordering::SeqCst)
    }
    fn ask(&self) -> std::io::Result<()> {
        self.asked.fetch_add(1, Ordering::SeqCst);
        Ok(())
    }
}

/// A home of its own, with `manager` starting things at login in it.
fn place(root: &TempDir, manager: Manager) -> Place {
    let home = root.path().join("my home");
    Place {
        manager,
        config: home.join(".config"),
        home,
    }
}

/// The owner CLI as install.sh puts it, under a home with a space.
fn cli(root: &TempDir) -> PathBuf {
    root.path().join("my home/.local/share/kaiki/bin/kaiki")
}

/// A profile kept with the file secrets: the job names the password's
/// file, never the password.
fn file_secrets(root: &TempDir) -> Vec<(String, String)> {
    vec![
        ("AGENTIC_SECRETS".into(), "file".into()),
        (
            "AGENTIC_PASSWORD_FILE".into(),
            root.path()
                .join("my home/.kaiki-password")
                .to_string_lossy()
                .into_owned(),
        ),
    ]
}

fn daemon(root: &TempDir, data_dir: Option<PathBuf>, env: Vec<(String, String)>) -> Launch {
    Launch::Daemon {
        cli: cli(root),
        data_dir,
        env,
    }
}

fn autostart(
    root: &TempDir,
    launch: Launch,
    manager: Manager,
    settings: &Arc<Settings>,
) -> Autostart {
    Autostart::new(
        launch,
        place(root, manager),
        &root.path().join("profile"),
        settings.clone(),
    )
}

/// The value of `key=` in a unit or desktop entry, each line once.
fn values(text: &str, key: &str) -> Vec<String> {
    text.lines()
        .filter_map(|line| line.strip_prefix(&format!("{key}=")))
        .map(str::to_owned)
        .collect()
}

/// A launchd job as launchd reads it.
#[cfg(target_os = "macos")]
fn launchd_job(path: &Path) -> serde_json::Value {
    let output = std::process::Command::new("/usr/bin/plutil")
        .args(["-convert", "json", "-o", "-"])
        .arg(path)
        .output()
        .unwrap();
    assert!(
        output.status.success(),
        "{}: not a property list",
        path.display()
    );
    serde_json::from_slice(&output.stdout).unwrap()
}

/// On macOS launchd runs `kaiki daemon start` at login with the profile's
/// secrets settings, and opens the app's window; each profile has a job of
/// its own. Both jobs leave the daemon running when their process ends:
/// kaiki exits once it started it, and the owner closes the window.
#[cfg(target_os = "macos")]
#[test]
fn launchd_starts_the_daemon_and_opens_the_window_at_login() {
    let root = TempDir::new().unwrap();
    let settings = Arc::new(Settings::default());
    let agents = root.path().join("my home/Library/LaunchAgents");

    let status = autostart(
        &root,
        daemon(&root, None, file_secrets(&root)),
        Manager::Launchd,
        &settings,
    )
    .ensure()
    .unwrap();
    assert_eq!(
        status,
        Status {
            state: State::On,
            path: agents.join("com.kaikichat.kaiki.plist")
        }
    );
    let job = launchd_job(&status.path);
    assert_eq!(
        job["Label"], "com.kaikichat.kaiki",
        "launchd knows it by its file's name"
    );
    assert_eq!(
        job["ProgramArguments"],
        serde_json::json!([cli(&root), "daemon", "start"])
    );
    assert_eq!(job["RunAtLoad"], true);
    assert_eq!(job["AbandonProcessGroup"], true);
    assert_eq!(
        job["EnvironmentVariables"],
        serde_json::json!({
            "AGENTIC_SECRETS": "file",
            "AGENTIC_PASSWORD_FILE": root.path().join("my home/.kaiki-password"),
        })
    );

    let other_dir = root.path().join("other profile");
    let other = Autostart::new(
        daemon(&root, Some(other_dir.clone()), vec![]),
        place(&root, Manager::Launchd),
        &other_dir,
        settings.clone(),
    )
    .ensure()
    .unwrap();
    assert_ne!(other.path, status.path, "each profile has a job of its own");
    assert!(status.path.is_file(), "the default profile's job stays");
    let job = launchd_job(&other.path);
    assert_eq!(
        job["ProgramArguments"],
        serde_json::json!([cli(&root), "--data-dir", other_dir, "daemon", "start"])
    );
    assert_ne!(
        job["Label"], "com.kaikichat.kaiki",
        "launchd loads one job per label"
    );
    assert_eq!(
        job["Label"].as_str().map(|label| format!("{label}.plist")),
        other
            .path
            .file_name()
            .map(|name| name.to_string_lossy().into_owned())
    );

    let app = "/Applications/Kaiki Chat.app/Contents/MacOS/agentic-desktop";
    let window = autostart(
        &root,
        Launch::Window { app: app.into() },
        Manager::Launchd,
        &settings,
    )
    .ensure()
    .unwrap();
    assert_eq!(window.state, State::On);
    assert_eq!(window.path, agents.join("com.kaikichat.app.plist"));
    let job = launchd_job(&window.path);
    assert_eq!(job["Label"], "com.kaikichat.app");
    assert_eq!(job["ProgramArguments"], serde_json::json!([app]));
    assert_eq!(job["RunAtLoad"], true);
    assert_eq!(job["AbandonProcessGroup"], true);
    assert_eq!(settings.asked(), 0, "nothing blocked, nobody asked");
}

/// On Linux a systemd user unit, enabled for the owner's login, starts the
/// daemon and stops it at logout; without systemd an XDG autostart entry
/// starts it. The app's window opens from an XDG autostart entry, in the
/// graphical session.
#[test]
fn systemd_and_xdg_autostart_start_kaiki_on_linux() {
    let root = TempDir::new().unwrap();
    let settings = Arc::new(Settings::default());
    let config = root.path().join("my home/.config");
    let kaiki = cli(&root).to_string_lossy().into_owned();
    let password = root.path().join("my home/.kaiki-password");

    let unit = autostart(
        &root,
        daemon(&root, None, file_secrets(&root)),
        Manager::Systemd,
        &settings,
    )
    .ensure()
    .unwrap();
    assert_eq!(
        unit,
        Status {
            state: State::On,
            path: config.join("systemd/user/kaiki.service")
        }
    );
    let text = fs::read_to_string(&unit.path).unwrap();
    assert_eq!(
        values(&text, "ExecStart"),
        [format!("\"{kaiki}\" daemon start")]
    );
    // kaiki exits once the daemon runs: the unit stays active, so systemd
    // neither stops it nor kills the daemon until the owner logs out.
    assert_eq!(values(&text, "Type"), ["oneshot"]);
    assert_eq!(values(&text, "RemainAfterExit"), ["yes"]);
    assert_eq!(
        values(&text, "ExecStop"),
        [format!("\"{kaiki}\" daemon stop")]
    );
    let env = values(&text, "Environment");
    assert_eq!(env.len(), 2, "{env:?}");
    assert!(
        env.iter()
            .any(|value| value.trim_matches('"') == "AGENTIC_SECRETS=file"),
        "{env:?}"
    );
    assert!(
        env.contains(&format!("\"AGENTIC_PASSWORD_FILE={}\"", password.display())),
        "{env:?}"
    );
    assert_eq!(values(&text, "WantedBy"), ["default.target"]);
    let enabled = config.join("systemd/user/default.target.wants/kaiki.service");
    assert_eq!(
        fs::canonicalize(&enabled).unwrap(),
        fs::canonicalize(&unit.path).unwrap()
    );

    let entry = autostart(
        &root,
        daemon(&root, None, file_secrets(&root)),
        Manager::Xdg,
        &settings,
    )
    .ensure()
    .unwrap();
    assert_eq!(entry.path, config.join("autostart/kaiki.desktop"));
    let text = fs::read_to_string(&entry.path).unwrap();
    assert_eq!(values(&text, "Type"), ["Application"]);
    assert_eq!(
        values(&text, "Exec"),
        [format!(
            "env AGENTIC_SECRETS=file \"AGENTIC_PASSWORD_FILE={}\" \"{kaiki}\" daemon start",
            password.display()
        )]
    );

    let window = autostart(
        &root,
        Launch::Window {
            app: "/usr/bin/agentic-desktop".into(),
        },
        Manager::Systemd,
        &settings,
    )
    .ensure()
    .unwrap();
    assert_eq!(window.path, config.join("autostart/kaiki-chat.desktop"));
    let text = fs::read_to_string(&window.path).unwrap();
    assert_eq!(values(&text, "Exec"), ["/usr/bin/agentic-desktop"]);
    assert_eq!(values(&text, "Name"), ["Kaiki Chat"]);
}

/// Turned off by the owner, here or in the system's own settings, the job
/// stays off: a later start (the next `kaiki` run, the window opening
/// again) does not put it back; only turning it on does.
#[test]
fn the_owners_off_holds_until_the_owner_turns_it_on() {
    let root = TempDir::new().unwrap();
    let settings = Arc::new(Settings::default());
    let started = || {
        autostart(
            &root,
            daemon(&root, None, vec![]),
            Manager::Systemd,
            &settings,
        )
        .ensure()
        .unwrap()
    };
    let unit = started();
    let enabled = root
        .path()
        .join("my home/.config/systemd/user/default.target.wants/kaiki.service");
    assert!(enabled.exists());

    let off = autostart(
        &root,
        daemon(&root, None, vec![]),
        Manager::Systemd,
        &settings,
    )
    .disable()
    .unwrap();
    assert_eq!(off.state, State::Off);
    assert!(!unit.path.exists() && !enabled.exists());
    assert_eq!(started().state, State::Off);
    assert!(!unit.path.exists(), "a start does not put it back");

    let on = autostart(
        &root,
        daemon(&root, None, vec![]),
        Manager::Systemd,
        &settings,
    )
    .enable()
    .unwrap();
    assert_eq!(on.state, State::On);
    assert!(unit.path.is_file() && enabled.exists());

    // `systemctl --user disable kaiki`: the owner's choice too.
    fs::remove_file(&enabled).unwrap();
    assert_eq!(started().state, State::Off);
    assert!(!enabled.exists());
    let on = autostart(
        &root,
        daemon(&root, None, vec![]),
        Manager::Systemd,
        &settings,
    )
    .enable()
    .unwrap();
    assert_eq!(on.state, State::On);
    assert!(enabled.exists());
}

/// A job that names another place of the app or of kaiki (the app moved
/// to another folder, kaiki installed elsewhere) is written anew at the
/// next start, so the next login runs the one in use; the same job is
/// left as it is (macOS tells the owner of every new one).
#[test]
fn a_job_naming_an_old_place_is_written_anew() {
    let root = TempDir::new().unwrap();
    let settings = Arc::new(Settings::default());
    let window = |app: &str| {
        autostart(
            &root,
            Launch::Window { app: app.into() },
            Manager::Launchd,
            &settings,
        )
        .ensure()
        .unwrap()
    };
    let old = "/Users/owner/Applications/Kaiki Chat.app/Contents/MacOS/agentic-desktop";
    let new = "/Applications/Kaiki Chat.app/Contents/MacOS/agentic-desktop";
    let first = window(old);
    let written = fs::metadata(&first.path).unwrap().modified().unwrap();
    window(old);
    assert_eq!(
        fs::metadata(&first.path).unwrap().modified().unwrap(),
        written
    );
    let moved = window(new);
    assert_eq!(moved.state, State::On);
    let text = fs::read_to_string(&moved.path).unwrap();
    assert!(text.contains(new) && !text.contains(old), "{text}");

    let kaiki = |cli: &Path| {
        autostart(
            &root,
            Launch::Daemon {
                cli: cli.into(),
                data_dir: None,
                env: vec![],
            },
            Manager::Systemd,
            &settings,
        )
        .ensure()
        .unwrap()
    };
    let before = root.path().join("old/kaiki");
    kaiki(&before);
    let unit = kaiki(&cli(&root));
    assert_eq!(unit.state, State::On);
    let text = fs::read_to_string(&unit.path).unwrap();
    assert_eq!(
        values(&text, "ExecStart"),
        [format!("\"{}\" daemon start", cli(&root).display())]
    );
}

/// The window and the daemon of one profile are two jobs with two
/// choices: the app's setting turned off leaves the daemon's job, and the
/// owner asked about one job is still asked about the other.
#[test]
fn the_windows_and_the_daemons_choices_are_kept_apart() {
    let root = TempDir::new().unwrap();
    let settings = Arc::new(Settings::default());
    let window = || {
        autostart(
            &root,
            Launch::Window {
                app: "/Applications/Kaiki Chat.app/Contents/MacOS/agentic-desktop".into(),
            },
            Manager::Launchd,
            &settings,
        )
    };
    let kaiki = || {
        autostart(
            &root,
            daemon(&root, None, vec![]),
            Manager::Launchd,
            &settings,
        )
    };

    window().ensure().unwrap();
    let job = kaiki().ensure().unwrap();
    assert_eq!(window().disable().unwrap().state, State::Off);
    assert_eq!(kaiki().ensure().unwrap().state, State::On);
    assert!(job.path.is_file());
    assert_eq!(window().ensure().unwrap().state, State::Off);

    settings.block(true);
    window().enable().unwrap();
    assert_eq!(settings.asked(), 1);
    assert_eq!(kaiki().ensure().unwrap().state, State::Blocked);
    assert_eq!(settings.asked(), 2, "the daemon's job is asked for too");
}

/// A job the system blocks until the owner allows it (turned off in macOS
/// Login Items): a start asks the owner once, by opening the system's
/// login items; turning it on asks again. Once allowed, it is on.
#[test]
fn a_blocked_job_is_asked_for_once_and_again_when_turned_on() {
    let root = TempDir::new().unwrap();
    let settings = Arc::new(Settings::default());
    settings.block(true);
    let job = || {
        autostart(
            &root,
            Launch::Window {
                app: "/Applications/Kaiki Chat.app/Contents/MacOS/agentic-desktop".into(),
            },
            Manager::Launchd,
            &settings,
        )
    };

    assert_eq!(job().ensure().unwrap().state, State::Blocked);
    assert_eq!(settings.asked(), 1);
    assert_eq!(job().ensure().unwrap().state, State::Blocked);
    assert_eq!(job().status().state, State::Blocked);
    assert_eq!(
        settings.asked(),
        1,
        "asked once, not at every start or look"
    );

    assert_eq!(job().enable().unwrap().state, State::Blocked);
    assert_eq!(settings.asked(), 2, "turning it on asks again");

    settings.block(false);
    assert_eq!(job().status().state, State::On);
}

/// Only the installed app opens at login, on the profile it opens by
/// itself: never a build, a copy run from a disk image or a download
/// (macOS runs those from a place that goes away), nor a window on a
/// profile named by `AGENTIC_DATA_DIR`.
#[test]
fn only_the_installed_app_opens_at_login() {
    let home = Path::new("/Users/owner");
    let at_login = |exe: &str, named_profile: bool| {
        window_launch(Path::new(exe), home, named_profile).map(|launch| match launch {
            Launch::Window { app } => app,
            Launch::Daemon { .. } => panic!("the window's launch is the window"),
        })
    };
    let installed: &[&str] = if cfg!(target_os = "macos") {
        &[
            "/Applications/Kaiki Chat.app/Contents/MacOS/agentic-desktop",
            "/Users/owner/Applications/Kaiki Chat.app/Contents/MacOS/agentic-desktop",
        ]
    } else {
        &["/usr/bin/agentic-desktop"]
    };
    for exe in installed {
        assert_eq!(at_login(exe, false), Some(PathBuf::from(exe)), "{exe}");
        assert_eq!(at_login(exe, true), None, "{exe} on a named profile");
    }
    for exe in [
        "/Users/owner/Code/chat/target/debug/agentic-desktop",
        "/Users/owner/Code/chat/target/release/bundle/macos/Kaiki Chat.app/Contents/MacOS/agentic-desktop",
        "/Volumes/Kaiki Chat/Kaiki Chat.app/Contents/MacOS/agentic-desktop",
        "/private/var/folders/x1/T/AppTranslocation/0A1B/d/Kaiki Chat.app/Contents/MacOS/agentic-desktop",
        "/Users/owner/Downloads/Kaiki Chat.app/Contents/MacOS/agentic-desktop",
        "/home/owner/kaiki/agentic-desktop",
    ] {
        assert_eq!(at_login(exe, false), None, "{exe}");
    }
}

/// The real login items of macOS: a job the system never saw is not
/// blocked, so a start never asks the owner about it.
#[cfg(target_os = "macos")]
#[test]
fn macos_does_not_block_a_job_it_never_saw() {
    let root = TempDir::new().unwrap();
    let job = root
        .path()
        .join("Library/LaunchAgents/com.kaikichat.kaiki.plist");
    fs::create_dir_all(job.parent().unwrap()).unwrap();
    fs::write(
        &job,
        "<?xml version=\"1.0\"?><plist version=\"1.0\"><dict/></plist>",
    )
    .unwrap();
    assert!(!SystemApproval.blocked(&job));
}
