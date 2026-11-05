use serde::{Deserialize, Serialize};
use serde_json::Value;
use sha2::{Digest, Sha256};
use std::{
    fs,
    io::{BufRead, BufReader, Read},
    path::{Component, Path, PathBuf},
    process::{Child, Command, Stdio},
    sync::{Arc, Mutex},
    time::Duration,
};

#[derive(Deserialize)]
struct FileEntry {
    path: String,
    sha256: String,
}
#[derive(Deserialize)]
struct Manifest {
    version: String,
    minecraft: String,
    files: Vec<FileEntry>,
}
#[derive(Default)]
pub struct Progress {
    action: Option<String>,
    lines: Vec<String>,
    error: Option<String>,
}
pub struct Shared {
    backend: Arc<Backend>,
    progress: Arc<Mutex<Progress>>,
    child: Arc<Mutex<Option<Child>>>,
}
pub struct Backend {
    payload: PathBuf,
    root: PathBuf,
    launcher_home: PathBuf,
    manifest: Manifest,
}
#[derive(Serialize)]
pub struct Snapshot {
    version: String,
    minecraft: String,
    installed: bool,
    prepared: bool,
    accounts: Value,
    action: Option<String>,
    lines: Vec<String>,
    error: Option<String>,
    data_home: String,
}

fn err(e: impl std::fmt::Display) -> String {
    e.to_string()
}
fn hash(path: &Path) -> Result<String, String> {
    let mut file = fs::File::open(path).map_err(err)?;
    let mut sha = Sha256::new();
    let mut buffer = [0; 65536];
    loop {
        let n = file.read(&mut buffer).map_err(err)?;
        if n == 0 {
            break;
        }
        sha.update(&buffer[..n]);
    }
    Ok(format!("{:x}", sha.finalize()))
}
fn relative(s: &str) -> Result<&Path, String> {
    let path = Path::new(s);
    if path
        .components()
        .any(|p| !matches!(p, Component::Normal(_)))
        || s.contains('\\')
        || s.is_empty()
    {
        return Err("Invalid payload path".into());
    }
    Ok(path)
}
fn hidden(c: &mut Command) {
    #[cfg(windows)]
    {
        use std::os::windows::process::CommandExt;
        c.creation_flags(0x08000000);
    }
    #[cfg(not(windows))]
    let _ = c;
}

impl Backend {
    pub fn new(payload: PathBuf) -> Result<Self, String> {
        let manifest: Manifest =
            serde_json::from_slice(&fs::read(payload.join("manifest.json")).map_err(err)?)
                .map_err(err)?;
        relative(&manifest.version)?;
        relative(&manifest.minecraft)?;
        let root = std::env::var_os("TURTLE_LAUNCHER_HOME")
            .map(PathBuf::from)
            .or_else(|| {
                std::env::var_os("LOCALAPPDATA").map(|p| PathBuf::from(p).join("TurtleClient"))
            })
            .ok_or("LOCALAPPDATA unavailable")?;
        let launcher_home = std::env::var_os("HORIZON_LAUNCHER_HOME")
            .map(PathBuf::from)
            .unwrap_or_else(|| root.join("Launcher"));
        fs::create_dir_all(root.join("logs")).map_err(err)?;
        Ok(Self {
            payload,
            root,
            launcher_home,
            manifest,
        })
    }
    pub fn beside_executable() -> Result<Self, String> {
        Self::new(
            std::env::current_exe()
                .map_err(err)?
                .parent()
                .ok_or("No executable directory")?
                .join("payload"),
        )
    }
    fn version_dir(&self) -> PathBuf {
        self.launcher_home
            .join("versions")
            .join(&self.manifest.version)
    }
    fn prepared(&self) -> bool {
        self.version_dir()
            .join(".local/horizon-client")
            .join(&self.manifest.minecraft)
            .join("horizon-resources.json")
            .is_file()
    }
    fn command(&self, args: &[&str]) -> Command {
        let mut c = Command::new(self.payload.join("horizon-launcher.exe"));
        c.args(args)
            .current_dir(&self.root)
            .env("HORIZON_LAUNCHER_HOME", &self.launcher_home)
            .env("HORIZON_LAUNCHER_NO_SELF_UPDATE", "1");
        for (key, sub) in [
            ("HORIZON_HOME", "Game"),
            ("XDG_DATA_HOME", "xdg-data"),
            ("XDG_CONFIG_HOME", "xdg-config"),
            ("HORIZON_LAUNCHER_TRASH", "trash"),
            ("HORIZON_LAUNCHER_CLIPBOARD", "clipboard"),
            ("HORIZON_LAUNCHER_DESKTOP", "desktop"),
        ] {
            if std::env::var_os(key).is_none() {
                c.env(key, self.root.join(sub));
            }
        }
        hidden(&mut c);
        c
    }
    fn query(&self, args: &[&str]) -> Result<Value, String> {
        let output = self
            .command(args)
            .stdin(Stdio::null())
            .output()
            .map_err(err)?;
        if !output.status.success() {
            return Err(String::from_utf8_lossy(&output.stderr).trim().to_owned());
        }
        serde_json::from_slice(&output.stdout).map_err(err)
    }
    pub fn install(&self) -> Result<(), String> {
        // Only the native program and redistributable tools are shipped; never Mojang resources.
        let dir = self.version_dir();
        let game_hash = self
            .manifest
            .files
            .iter()
            .find(|f| f.path == "version/horizon-client.exe")
            .ok_or("Missing game in manifest")?;
        if !dir.join(".installed").exists()
            || hash(&dir.join("horizon-client.exe")).ok().as_ref() != Some(&game_hash.sha256)
        {
            let staging = self
                .launcher_home
                .join("versions")
                .join(format!(".turtle-install-{}", std::process::id()));
            if dir.exists() {
                return Err("Existing Turtle version differs from this package; its files were preserved. Install an updated package with a new version.".into());
            }
            fs::create_dir_all(&staging).map_err(err)?;
            for entry in &self.manifest.files {
                let path = relative(&entry.path)?;
                let source = self.payload.join(path);
                if hash(&source)? != entry.sha256 {
                    return Err(format!("Payload checksum failed: {}", entry.path));
                }
                if let Ok(part) = path.strip_prefix("version") {
                    let target = staging.join(part);
                    fs::create_dir_all(target.parent().ok_or("No parent")?).map_err(err)?;
                    fs::copy(source, target).map_err(err)?;
                }
            }
            fs::write(staging.join(".installed"), "").map_err(err)?;
            fs::rename(&staging, &dir).map_err(err)?;
        }
        let instances = self.query(&["instance", "list", "--json"])?;
        if !instances
            .as_array()
            .is_some_and(|v| v.iter().any(|i| i["name"] == "TurtleClient"))
        {
            self.query(&["instance", "create", "TurtleClient", "--json"])?;
        }
        self.query(&[
            "instance",
            "version",
            "TurtleClient",
            &self.manifest.version,
            "--json",
        ])?;
        Ok(())
    }
    pub fn snapshot(&self, p: &Progress) -> Result<Snapshot, String> {
        let accounts = self.query(&["account", "list", "--json"])?;
        Ok(Snapshot {
            version: self.manifest.version.clone(),
            minecraft: self.manifest.minecraft.clone(),
            installed: self.version_dir().join(".installed").exists(),
            prepared: self.prepared(),
            accounts,
            action: p.action.clone(),
            lines: p.lines.clone(),
            error: p.error.clone(),
            data_home: self.root.to_string_lossy().into_owned(),
        })
    }
    fn api(&self) -> Result<(), String> {
        let mut c = Command::new(self.payload.join("turtle-launch.exe"));
        c.arg("--start-api")
            .env(
                "TURTLECLIENT_DATA_HOME",
                std::env::var_os("TURTLECLIENT_DATA_HOME")
                    .map(PathBuf::from)
                    .unwrap_or_else(|| self.root.join("Ocean-Rest")),
            )
            .stdin(Stdio::null());
        hidden(&mut c);
        // A persistent Windows grandchild can retain inherited pipe handles. Wait for the
        // short-lived helper itself, with file-backed output, rather than waiting for pipe EOF.
        let log_path = self.root.join("logs/ocean-start.log");
        let log = fs::File::create(&log_path).map_err(err)?;
        c.stdout(log.try_clone().map_err(err)?).stderr(log);
        let status = c.status().map_err(err)?;
        if status.success() {
            Ok(())
        } else {
            Err(fs::read_to_string(log_path).unwrap_or_else(|_| status.to_string()))
        }
    }
}
impl Shared {
    pub fn new(backend: Backend) -> Self {
        Self {
            backend: Arc::new(backend),
            progress: Arc::new(Mutex::new(Progress::default())),
            child: Arc::new(Mutex::new(None)),
        }
    }
    pub fn cancel_non_game(&self) {
        if self.progress.lock().unwrap().action.as_deref() == Some("play") {
            return;
        }
        let mut child = self.child.lock().unwrap();
        let Some(c) = child.as_mut() else {
            return;
        };
        if c.try_wait().ok().flatten().is_some() {
            return;
        }
        #[cfg(windows)]
        {
            let mut stop = Command::new("taskkill.exe");
            stop.args(["/PID", &c.id().to_string(), "/T", "/F"])
                .stdout(Stdio::null())
                .stderr(Stdio::null());
            hidden(&mut stop);
            let _ = stop.status();
        }
        let _ = c.kill();
    }
}
fn line(progress: &Mutex<Progress>, text: String) {
    let mut p = progress.lock().unwrap();
    p.lines.push(text.chars().take(2000).collect());
    if p.lines.len() > 150 {
        p.lines.remove(0);
    }
}
fn run_child(
    mut command: Command,
    progress: &Arc<Mutex<Progress>>,
    child: &Arc<Mutex<Option<Child>>>,
    play_log: Option<&Path>,
) -> Result<(), String> {
    command.stdin(Stdio::null());
    if let Some(path) = play_log {
        let log = fs::File::create(path).map_err(err)?;
        command.stdout(log.try_clone().map_err(err)?).stderr(log);
    } else {
        command.stdout(Stdio::piped()).stderr(Stdio::piped());
    }
    let mut process = command.spawn().map_err(err)?;
    let readers = [
        process
            .stdout
            .take()
            .map(|r| Box::new(r) as Box<dyn Read + Send>),
        process
            .stderr
            .take()
            .map(|r| Box::new(r) as Box<dyn Read + Send>),
    ];
    *child.lock().unwrap() = Some(process);
    let handles: Vec<_> = readers
        .into_iter()
        .flatten()
        .map(|r| {
            let p = progress.clone();
            std::thread::spawn(move || {
                for text in BufReader::new(r).lines().map_while(Result::ok) {
                    line(&p, text);
                }
            })
        })
        .collect();
    let status = loop {
        let done = child
            .lock()
            .unwrap()
            .as_mut()
            .ok_or("Process unavailable")?
            .try_wait()
            .map_err(err)?;
        if let Some(s) = done {
            break s;
        }
        std::thread::sleep(Duration::from_millis(200));
    };
    *child.lock().unwrap() = None;
    for handle in handles {
        let _ = handle.join();
    }
    if !status.success() {
        return Err(format!(
            "Process exited: {status}. See the activity log / logs/game-launch.log."
        ));
    }
    Ok(())
}

#[tauri::command]
pub async fn get_state(state: tauri::State<'_, Shared>) -> Result<Snapshot, String> {
    let b = state.backend.clone();
    let p = state.progress.clone();
    tauri::async_runtime::spawn_blocking(move || b.snapshot(&p.lock().unwrap()))
        .await
        .map_err(err)?
}
#[tauri::command]
pub fn start_action(
    action: String,
    value: Option<String>,
    state: tauri::State<'_, Shared>,
) -> Result<(), String> {
    if ![
        "install", "prepare", "download", "sign-in", "play", "api", "select", "sign-out",
    ]
    .contains(&action.as_str())
    {
        return Err("Unknown action".into());
    }
    {
        let mut p = state.progress.lock().unwrap();
        if p.action.is_some() {
            return Err("An operation is already running".into());
        }
        p.action = Some(action.clone());
        p.error = None;
        p.lines.push(format!("> {action}"));
    }
    let b = state.backend.clone();
    let p = state.progress.clone();
    let child = state.child.clone();
    std::thread::spawn(move || {
        let result = (|| match action.as_str() {
            "install" => b.install(),
            "api" => b.api(),
            "prepare" | "download" => {
                b.install()?;
                let mut args = vec!["prepare", "--version", &b.manifest.version];
                if action == "download" {
                    args.push("--download");
                } else if let Some(path) = value.as_deref().filter(|s| !s.trim().is_empty()) {
                    args.extend(["--minecraft", path]);
                }
                run_child(b.command(&args), &p, &child, None)
            }
            "sign-in" => run_child(b.command(&["account", "sign-in"]), &p, &child, None),
            "select" | "sign-out" => run_child(
                b.command(&[
                    "account",
                    &action,
                    value.as_deref().ok_or("No account selected")?,
                ]),
                &p,
                &child,
                None,
            ),
            "play" => {
                b.install()?;
                if !b.prepared() {
                    return Err("Prepare Minecraft 26.3 resources in Setup first.".into());
                }
                b.api()?;
                run_child(
                    b.command(&["play", "--instance", "TurtleClient"]),
                    &p,
                    &child,
                    Some(&b.root.join("logs/game-launch.log")),
                )
            }
            _ => unreachable!(),
        })();
        if let Err(e) = result {
            line(&p, e.clone());
            p.lock().unwrap().error = Some(e);
        } else {
            line(&p, "> OK".into());
        }
        p.lock().unwrap().action = None;
    });
    Ok(())
}
#[tauri::command]
pub fn cancel_action(state: tauri::State<'_, Shared>) {
    state.cancel_non_game();
}
#[tauri::command]
pub fn open_location(location: String, state: tauri::State<'_, Shared>) -> Result<(), String> {
    let path = match location.as_str() {
        "logs" => state.backend.root.join("logs"),
        "data" => state.backend.root.clone(),
        _ => return Err("Unknown location".into()),
    };
    Command::new("explorer.exe")
        .arg(path)
        .spawn()
        .map_err(err)?;
    Ok(())
}
#[tauri::command]
pub fn open_sign_in() -> Result<(), String> {
    // Fixed Microsoft URL only: frontend strings never become shell commands.
    Command::new("explorer.exe")
        .arg("https://www.microsoft.com/link")
        .spawn()
        .map_err(err)?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn payload_paths_stay_relative() {
        for bad in [
            "../secret",
            "/absolute",
            "C:/escape",
            "a/../../b",
            "a\\b",
            "",
        ] {
            assert!(relative(bad).is_err(), "{bad}");
        }
        assert!(relative("version/exporters/Exporter.java").is_ok());
    }
}
