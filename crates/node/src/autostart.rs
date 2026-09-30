//! Starting Kaiki Chat when the owner logs in (spec/owner-cli-v1.md and
//! spec/desktop-gui-v1.md, "Start at login"): the app's window, or the
//! profile's daemon for the command line, as a job of the system's login
//! manager. Only the job's file is written, so nothing starts before the
//! next login. The owner's choice is kept in the profile's
//! `autostart.json`: once the owner turns a job off, here or in the system,
//! later starts leave it off. When the system blocks a job until the owner
//! allows it (macOS Login Items), a start asks the owner once by opening the
//! system's login items.
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::{
    collections::BTreeMap,
    fs, io,
    os::unix::fs::{DirBuilderExt, OpenOptionsExt},
    path::{Path, PathBuf},
    sync::Arc,
};

pub type Result<T> = crate::Result<T>;

/// The launchd labels: the app's window, and the default profile's daemon
/// (another profile's has its hash after it).
const WINDOW_LABEL: &str = "com.kaikichat.app";
const DAEMON_LABEL: &str = "com.kaikichat.kaiki";
/// The owner's choices, in the profile's directory.
const CHOICES: &str = "autostart.json";

/// What starts at login.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Launch {
    /// The app's window: its binary.
    Window { app: PathBuf },
    /// The profile's daemon: `kaiki [--data-dir DIR] daemon start`, with
    /// `env` (the secrets settings; never a password). No `data_dir` is the
    /// profile kaiki opens by itself.
    Daemon {
        cli: PathBuf,
        data_dir: Option<PathBuf>,
        env: Vec<(String, String)>,
    },
}

impl Launch {
    /// Its name in the owner's choices.
    fn key(&self) -> &'static str {
        match self {
            Self::Window { .. } => "window",
            Self::Daemon { .. } => "daemon",
        }
    }

    /// The command line for `action` (`start` or `stop` of the daemon).
    fn command(&self, action: &str) -> Vec<String> {
        match self {
            Self::Window { app } => vec![text(app)],
            Self::Daemon { cli, data_dir, .. } => {
                let mut words = vec![text(cli)];
                if let Some(dir) = data_dir {
                    words.extend(["--data-dir".to_owned(), text(dir)]);
                }
                words.extend(["daemon".to_owned(), action.to_owned()]);
                words
            }
        }
    }

    fn env(&self) -> &[(String, String)] {
        match self {
            Self::Window { .. } => &[],
            Self::Daemon { env, .. } => env,
        }
    }

    /// What tells another profile's job from the default profile's.
    fn suffix(&self) -> Option<String> {
        match self {
            Self::Daemon {
                data_dir: Some(dir),
                ..
            } => Some(hex::encode(
                &Sha256::digest(dir.as_os_str().as_encoded_bytes())[..4],
            )),
            _ => None,
        }
    }
}

fn text(path: &Path) -> String {
    path.to_string_lossy().into_owned()
}

/// The app's window at login, when `exe` is the installed app (a bundle in
/// the Applications folder on macOS, the package's binary on Linux) and the
/// window opens the profile it opens by itself. A build, a copy run from a
/// disk image or a download, and a profile named by `AGENTIC_DATA_DIR`
/// (`named_profile`) do not open at login.
pub fn window_launch(exe: &Path, home: &Path, named_profile: bool) -> Option<Launch> {
    if named_profile {
        return None;
    }
    let installed = if cfg!(target_os = "macos") {
        crate::update::app_bundle(exe).is_some_and(|bundle| {
            bundle.starts_with("/Applications") || bundle.starts_with(home.join("Applications"))
        })
    } else {
        exe.starts_with("/usr") || exe.starts_with("/opt")
    };
    installed.then(|| Launch::Window {
        app: exe.to_owned(),
    })
}

/// How this system starts things at login.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Manager {
    /// macOS: launch agents in `~/Library/LaunchAgents`.
    Launchd,
    /// Linux with systemd: user units enabled for the login.
    Systemd,
    /// Other Linux: XDG autostart entries of the graphical session.
    Xdg,
}

impl Manager {
    pub fn here() -> Self {
        if cfg!(target_os = "macos") {
            Self::Launchd
        } else if Path::new("/run/systemd/system").is_dir() {
            Self::Systemd
        } else {
            Self::Xdg
        }
    }
}

/// Where this user's login items are.
#[derive(Clone, Debug)]
pub struct Place {
    pub manager: Manager,
    pub home: PathBuf,
    /// `XDG_CONFIG_HOME`, else `~/.config`.
    pub config: PathBuf,
}

impl Place {
    pub fn here() -> Option<Self> {
        let home = dirs::home_dir()?;
        let config = std::env::var_os("XDG_CONFIG_HOME")
            .map(PathBuf::from)
            .filter(|dir| dir.is_absolute())
            .unwrap_or_else(|| home.join(".config"));
        Some(Self {
            manager: Manager::here(),
            home,
            config,
        })
    }
}

/// The system's say over a job: whether it keeps the job from running until
/// the owner allows it, and how the owner is asked to.
pub trait Approval: Send + Sync {
    fn blocked(&self, job: &Path) -> bool;
    /// Opens the system's login items for the owner.
    fn ask(&self) -> io::Result<()>;
}

/// This system's login items: macOS blocks a launch agent the owner turned
/// off in System Settings; Linux blocks none.
pub struct SystemApproval;

impl Approval for SystemApproval {
    fn blocked(&self, job: &Path) -> bool {
        system::blocked(job)
    }
    fn ask(&self) -> io::Result<()> {
        system::open_login_items()
    }
}

/// macOS's login items through ServiceManagement's SMAppService (macOS 13
/// and later; before it launch agents were never blocked), asked by
/// `osascript`, whose JavaScript reaches the framework: this crate has no
/// unsafe code to call it itself.
#[cfg(target_os = "macos")]
mod system {
    use std::{
        io,
        path::Path,
        process::{Command, Stdio},
    };

    /// SMAppServiceStatusRequiresApproval: the owner turned the job off in
    /// System Settings.
    const REQUIRES_APPROVAL: &str = "2";
    const STATUS: &str = r#"function run(argv) { ObjC.import("ServiceManagement"); return $.SMAppService.statusForLegacyURL($.NSURL.fileURLWithPath(argv[0])); }"#;
    const OPEN: &str =
        r#"ObjC.import("ServiceManagement"); $.SMAppService.openSystemSettingsLoginItems();"#;
    /// Where System Settings shows login items, for a macOS whose
    /// SMAppService does not open them.
    const LOGIN_ITEMS: &str = "x-apple.systempreferences:com.apple.LoginItems-Settings.extension";

    fn script(source: &str, args: &[&std::ffi::OsStr]) -> Option<String> {
        let output = Command::new("/usr/bin/osascript")
            .args(["-l", "JavaScript", "-e", source])
            .args(args)
            .stdin(Stdio::null())
            .stderr(Stdio::null())
            .output()
            .ok()?;
        output
            .status
            .success()
            .then(|| String::from_utf8_lossy(&output.stdout).trim().to_owned())
    }

    pub(super) fn blocked(job: &Path) -> bool {
        script(STATUS, &[job.as_os_str()]).as_deref() == Some(REQUIRES_APPROVAL)
    }

    pub(super) fn open_login_items() -> io::Result<()> {
        if script(OPEN, &[]).is_some() {
            return Ok(());
        }
        let opened = Command::new("/usr/bin/open")
            .arg(LOGIN_ITEMS)
            .stdin(Stdio::null())
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .status()?;
        if opened.success() {
            Ok(())
        } else {
            Err(io::Error::other("System Settings did not open"))
        }
    }
}

#[cfg(not(target_os = "macos"))]
mod system {
    use std::{io, path::Path};

    pub(super) fn blocked(_job: &Path) -> bool {
        false
    }

    pub(super) fn open_login_items() -> io::Result<()> {
        Ok(())
    }
}

/// Whether a job starts at login.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum State {
    On,
    /// No job: the owner took it out, or none was put in.
    Off,
    /// The job is in, and the system keeps it from running until the owner
    /// allows it.
    Blocked,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
pub struct Status {
    pub state: State,
    /// The job's file.
    pub path: PathBuf,
}

/// The owner's choice about one job.
#[derive(Default, Serialize, Deserialize)]
struct Choice {
    /// The owner turned it off, here or in the system.
    #[serde(default)]
    off: bool,
    /// The owner was asked to allow it in the system.
    #[serde(default)]
    asked: bool,
    /// The job last written.
    #[serde(default)]
    job: Option<PathBuf>,
}

/// One job at login and the owner's choice about it.
pub struct Autostart {
    launch: Launch,
    place: Place,
    data_dir: PathBuf,
    approval: Arc<dyn Approval>,
}

impl Autostart {
    /// `launch` in `place`, with the owner's choice kept in `data_dir`.
    pub fn new(launch: Launch, place: Place, data_dir: &Path, approval: Arc<dyn Approval>) -> Self {
        Self {
            launch,
            place,
            data_dir: data_dir.to_owned(),
            approval,
        }
    }

    pub fn status(&self) -> Status {
        let job = self.job();
        let state = if !job.present() {
            State::Off
        } else if self.approval.blocked(&job.file) {
            State::Blocked
        } else {
            State::On
        };
        Status {
            state,
            path: job.file,
        }
    }

    /// At a start: puts the job in (again, if it names another place),
    /// unless the owner turned it off here or took it out in the system;
    /// asks the owner once when the system blocks it.
    pub fn ensure(&self) -> Result<Status> {
        let job = self.job();
        let mut state = None;
        self.decide(|choice| {
            if choice.off {
                return Ok(false);
            }
            if choice.job.as_deref() == Some(job.file.as_path()) && !job.present() {
                choice.off = true;
                return Ok(true);
            }
            job.write()?;
            let mut changed = choice.job.as_deref() != Some(job.file.as_path());
            choice.job = Some(job.file.clone());
            // Asking the system takes a process on macOS: once per start.
            let blocked = self.approval.blocked(&job.file);
            if blocked && !choice.asked {
                self.approval.ask()?;
                choice.asked = true;
                changed = true;
            }
            state = Some(if blocked { State::Blocked } else { State::On });
            Ok(changed)
        })?;
        Ok(match state {
            Some(state) => Status {
                state,
                path: job.file,
            },
            None => self.status(),
        })
    }

    /// The owner turns it on: the job is put in, and the owner is asked to
    /// allow it when the system blocks it.
    pub fn enable(&self) -> Result<Status> {
        let job = self.job();
        self.decide(|choice| {
            choice.off = false;
            job.write()?;
            choice.job = Some(job.file.clone());
            if self.approval.blocked(&job.file) {
                self.approval.ask()?;
                choice.asked = true;
            }
            Ok(true)
        })?;
        Ok(self.status())
    }

    /// The owner turns it off: the job is taken out, and stays out.
    pub fn disable(&self) -> Result<Status> {
        let job = self.job();
        self.decide(|choice| {
            if let Some(old) = choice.job.take().filter(|old| *old != job.file) {
                remove(&old)?;
            }
            job.remove()?;
            choice.off = true;
            Ok(true)
        })?;
        Ok(self.status())
    }

    /// Opens the system's login items for the owner.
    pub fn ask(&self) -> Result<()> {
        Ok(self.approval.ask()?)
    }

    /// Changes the owner's choice about this job with `change`, which says
    /// whether it changed; the other jobs' choices stay as they are.
    fn decide(&self, change: impl FnOnce(&mut Choice) -> io::Result<bool>) -> Result<()> {
        let path = self.data_dir.join(CHOICES);
        let mut choices: BTreeMap<String, Choice> = fs::read(&path)
            .ok()
            .and_then(|bytes| serde_json::from_slice(&bytes).ok())
            .unwrap_or_default();
        let key = self.launch.key();
        let mut choice = choices.remove(key).unwrap_or_default();
        let changed = change(&mut choice)?;
        choices.insert(key.to_owned(), choice);
        if changed {
            fs::DirBuilder::new()
                .recursive(true)
                .mode(0o700)
                .create(&self.data_dir)?;
            write_file(&path, &serde_json::to_vec(&choices)?, 0o600)?;
        }
        Ok(())
    }

    /// The job's file and text in this place.
    fn job(&self) -> Job {
        let suffix = self.launch.suffix();
        let (manager, window) = (
            self.place.manager,
            matches!(self.launch, Launch::Window { .. }),
        );
        match (manager, window) {
            (Manager::Launchd, _) => {
                let label = match (&suffix, window) {
                    (_, true) => WINDOW_LABEL.to_owned(),
                    (None, false) => DAEMON_LABEL.to_owned(),
                    (Some(suffix), false) => format!("{DAEMON_LABEL}.{suffix}"),
                };
                Job {
                    file: self
                        .place
                        .home
                        .join("Library/LaunchAgents")
                        .join(format!("{label}.plist")),
                    text: launch_agent(&label, &self.launch),
                    link: None,
                }
            }
            (_, true) => Job {
                file: self.place.config.join("autostart/kaiki-chat.desktop"),
                text: desktop_entry("Kaiki Chat", &self.launch),
                link: None,
            },
            (Manager::Systemd, false) => {
                let name = match &suffix {
                    None => "kaiki.service".to_owned(),
                    Some(suffix) => format!("kaiki-{suffix}.service"),
                };
                let units = self.place.config.join("systemd/user");
                Job {
                    file: units.join(&name),
                    text: user_unit(&self.launch),
                    link: Some(units.join("default.target.wants").join(&name)),
                }
            }
            (Manager::Xdg, false) => {
                let name = match &suffix {
                    None => "kaiki.desktop".to_owned(),
                    Some(suffix) => format!("kaiki-{suffix}.desktop"),
                };
                Job {
                    file: self.place.config.join("autostart").join(name),
                    text: desktop_entry("Kaiki Chat daemon", &self.launch),
                    link: None,
                }
            }
        }
    }
}

/// A job's file, its text, and for a systemd unit the link that enables it.
struct Job {
    file: PathBuf,
    text: String,
    link: Option<PathBuf>,
}

impl Job {
    /// Whether the job is in: its file, enabled, and not hidden by the
    /// desktop's startup settings.
    fn present(&self) -> bool {
        if let Some(link) = &self.link
            && fs::symlink_metadata(link).is_err()
        {
            return false;
        }
        match fs::read_to_string(&self.file) {
            Ok(text) => !text
                .lines()
                .any(|line| line == "Hidden=true" || line == "X-GNOME-Autostart-enabled=false"),
            Err(_) => false,
        }
    }

    /// Writes the job unless it is there as it is, and enables it.
    fn write(&self) -> io::Result<()> {
        if fs::read_to_string(&self.file).ok().as_deref() != Some(self.text.as_str()) {
            if let Some(dir) = self.file.parent() {
                fs::create_dir_all(dir)?;
            }
            write_file(&self.file, self.text.as_bytes(), 0o644)?;
        }
        if let Some(link) = &self.link
            && fs::read_link(link).ok().as_deref() != Some(self.file.as_path())
        {
            if let Some(dir) = link.parent() {
                fs::create_dir_all(dir)?;
            }
            remove(link)?;
            std::os::unix::fs::symlink(&self.file, link)?;
        }
        Ok(())
    }

    fn remove(&self) -> io::Result<()> {
        if let Some(link) = &self.link {
            remove(link)?;
        }
        remove(&self.file)
    }
}

fn remove(path: &Path) -> io::Result<()> {
    match fs::remove_file(path) {
        Err(error) if error.kind() != io::ErrorKind::NotFound => Err(error),
        _ => Ok(()),
    }
}

/// Writes `bytes` beside `path` and renames it over: a login never finds
/// half a job.
fn write_file(path: &Path, bytes: &[u8], mode: u32) -> io::Result<()> {
    use std::io::Write;
    let name = path.file_name().unwrap_or_default().to_string_lossy();
    let new = path.with_file_name(format!(".{name}.new"));
    let mut file = fs::OpenOptions::new()
        .write(true)
        .create(true)
        .truncate(true)
        .mode(mode)
        .open(&new)?;
    file.write_all(bytes)?;
    file.sync_all()?;
    fs::rename(&new, path)
}

/// A launch agent: run at login, in the owner's session. The process it
/// starts (kaiki, the window) leaves the daemon it started running when it
/// ends, so launchd must not end the daemon with it.
fn launch_agent(label: &str, launch: &Launch) -> String {
    let mut plist = String::from(concat!(
        "<?xml version=\"1.0\" encoding=\"UTF-8\"?>\n",
        "<!DOCTYPE plist PUBLIC \"-//Apple//DTD PLIST 1.0//EN\" ",
        "\"http://www.apple.com/DTDs/PropertyList-1.0.dtd\">\n",
        "<plist version=\"1.0\">\n<dict>\n",
    ));
    let line = |plist: &mut String, text: String| {
        plist.push('\t');
        plist.push_str(&text);
        plist.push('\n');
    };
    line(&mut plist, "<key>Label</key>".into());
    line(&mut plist, format!("<string>{}</string>", xml(label)));
    line(&mut plist, "<key>ProgramArguments</key>".into());
    line(&mut plist, "<array>".into());
    for word in launch.command("start") {
        line(&mut plist, format!("\t<string>{}</string>", xml(&word)));
    }
    line(&mut plist, "</array>".into());
    if !launch.env().is_empty() {
        line(&mut plist, "<key>EnvironmentVariables</key>".into());
        line(&mut plist, "<dict>".into());
        for (name, value) in launch.env() {
            line(&mut plist, format!("\t<key>{}</key>", xml(name)));
            line(&mut plist, format!("\t<string>{}</string>", xml(value)));
        }
        line(&mut plist, "</dict>".into());
    }
    for key in ["RunAtLoad", "AbandonProcessGroup"] {
        line(&mut plist, format!("<key>{key}</key>"));
        line(&mut plist, "<true/>".into());
    }
    // As the owner's own start: neither the window nor the daemon throttled.
    line(&mut plist, "<key>ProcessType</key>".into());
    line(&mut plist, "<string>Interactive</string>".into());
    plist.push_str("</dict>\n</plist>\n");
    plist
}

fn xml(text: &str) -> String {
    text.replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('"', "&quot;")
        .replace('\'', "&apos;")
}

/// A systemd user unit, enabled for the login: kaiki exits once the daemon
/// runs, and the unit stays active until the owner logs out, when it stops
/// the daemon.
fn user_unit(launch: &Launch) -> String {
    let mut unit = String::from(
        "[Unit]\nDescription=Kaiki Chat daemon\nDocumentation=https://kaikichat.com\n\n\
         [Service]\nType=oneshot\nRemainAfterExit=yes\n",
    );
    for (name, value) in launch.env() {
        unit.push_str(&format!(
            "Environment={}\n",
            systemd_word(&format!("{name}={value}"))
        ));
    }
    for (key, action) in [("ExecStart", "start"), ("ExecStop", "stop")] {
        let words: Vec<String> = launch
            .command(action)
            .iter()
            .map(|word| systemd_word(&word.replace('$', "$$")))
            .collect();
        unit.push_str(&format!("{key}={}\n", words.join(" ")));
    }
    unit.push_str("\n[Install]\nWantedBy=default.target\n");
    unit
}

/// One word of a unit's line: as it is when plain, else in double quotes.
/// `%` starts a specifier in either.
fn systemd_word(word: &str) -> String {
    let word = word.replace('%', "%%");
    if !word.is_empty() && word.chars().all(plain) {
        word
    } else {
        format!("\"{}\"", word.replace('\\', "\\\\").replace('"', "\\\""))
    }
}

fn plain(c: char) -> bool {
    c.is_ascii_alphanumeric() || "_./:=@+,-".contains(c)
}

/// An XDG autostart entry, run when the owner's graphical session starts.
fn desktop_entry(name: &str, launch: &Launch) -> String {
    let mut words: Vec<String> = Vec::new();
    if !launch.env().is_empty() {
        words.push("env".into());
        for (key, value) in launch.env() {
            words.push(exec_word(&format!("{key}={value}")));
        }
    }
    words.extend(launch.command("start").iter().map(|word| exec_word(word)));
    // The value's own escape of a backslash, after the argument's.
    let exec = words.join(" ").replace('\\', "\\\\");
    let hidden = if matches!(launch, Launch::Daemon { .. }) {
        "NoDisplay=true\n"
    } else {
        ""
    };
    format!(
        "[Desktop Entry]\nType=Application\nName={name}\nExec={exec}\nTerminal=false\n{hidden}X-GNOME-Autostart-enabled=true\n"
    )
}

/// One argument of `Exec`: as it is when plain, else in double quotes with
/// the characters the specification reserves escaped. `%` starts a field
/// code.
fn exec_word(word: &str) -> String {
    let word = word.replace('%', "%%");
    if !word.is_empty() && word.chars().all(plain) {
        return word;
    }
    let mut quoted = String::from("\"");
    for c in word.chars() {
        if matches!(c, '"' | '`' | '$' | '\\') {
            quoted.push('\\');
        }
        quoted.push(c);
    }
    quoted.push('"');
    quoted
}

#[cfg(test)]
#[path = "autostart_tests.rs"]
mod tests;
