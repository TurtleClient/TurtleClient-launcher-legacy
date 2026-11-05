use super::*;
use std::io::Write;
use std::net::TcpStream;

#[derive(Clone, Serialize, Deserialize)]
pub struct Profile {
    pub id: String,
    pub name: String,
    pub home: PathBuf,
    pub version: Option<String>,
    #[serde(default)]
    pub played_secs: u64,
    pub last_played_ms: Option<u64>,
}

#[derive(Clone, Serialize, Deserialize)]
pub struct ModuleSettings {
    pub zoom_enabled: bool,
    pub emotes_enabled: bool,
    pub capes: bool,
    pub zoom_factor: f64,
    pub smooth_zoom: bool,
    pub zoom_key: String,
    pub emote_key: String,
}
impl Default for ModuleSettings {
    fn default() -> Self {
        Self {
            zoom_enabled: true,
            emotes_enabled: true,
            capes: true,
            zoom_factor: 4.0,
            smooth_zoom: true,
            zoom_key: "c".into(),
            emote_key: "b".into(),
        }
    }
}

fn binding(key: &str, enabled: bool) -> Result<String, String> {
    if key.is_empty()
        || key.len() > 48
        || !key
            .bytes()
            .all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || c == b'.' || c == b'_')
        || key.starts_with("disabled.")
    {
        return Err("Invalid keyboard binding".into());
    }
    // The game compares bindings to concrete keyboard events. An unbound name never matches
    // an event; keeping the original suffix makes disabling a shortcut fully reversible.
    Ok(format!(
        "key.keyboard.{}{key}",
        if enabled { "" } else { "disabled." }
    ))
}
fn decode_binding(value: Option<&str>, fallback: &str) -> (String, bool) {
    let key = value
        .unwrap_or(fallback)
        .strip_prefix("key.keyboard.")
        .unwrap_or(fallback);
    match key.strip_prefix("disabled.") {
        Some(key) => (key.into(), false),
        None => (key.into(), true),
    }
}
fn profile_name(name: &str) -> Result<&str, String> {
    if name.chars().any(char::is_control) {
        return Err("Profile names cannot contain control characters".into());
    }
    let name = name.trim();
    if name.is_empty()
        || name.chars().count() > 48
        || name.starts_with('-')
        || name
            .chars()
            .any(|c| c.is_control() || matches!(c, '/' | '\\'))
    {
        return Err("Profile name must contain 1–48 characters without path separators".into());
    }
    Ok(name)
}

impl Backend {
    pub(super) fn profiles(&self) -> Result<Vec<Profile>, String> {
        let values = self.query(&["instance", "list", "--json"])?;
        values
            .as_array()
            .ok_or("Invalid instance list")?
            .iter()
            .filter(|v| v["version"] == self.manifest.version && v["java"].is_null())
            .cloned()
            .map(|v| serde_json::from_value(v).map_err(err))
            .collect()
    }
    pub(super) fn selected_profile<'a>(&self, profiles: &'a [Profile]) -> Option<&'a Profile> {
        let id = fs::read_to_string(self.root.join("selected-profile.txt")).ok();
        profiles
            .iter()
            .find(|p| Some(p.id.as_str()) == id.as_deref())
            .or_else(|| profiles.iter().find(|p| p.name == "TurtleClient"))
            .or_else(|| profiles.first())
    }
    pub(super) fn select_profile(&self, id: &str) -> Result<(), String> {
        let profiles = self.profiles()?;
        let profile = profiles
            .iter()
            .find(|p| p.id == id)
            .ok_or("Unknown Turtle profile")?;
        fs::write(self.root.join("selected-profile.txt"), &profile.id).map_err(err)
    }
    pub(super) fn create_profile(&self, name: &str) -> Result<(), String> {
        let name = profile_name(name)?;
        self.install()?;
        let value = self.query(&["instance", "create", name, "--json"])?;
        let id = value["id"].as_str().ok_or("Missing new profile ID")?;
        self.query(&["instance", "version", id, &self.manifest.version, "--json"])?;
        self.select_profile(id)
    }
    pub(super) fn rename_profile(&self, name: &str) -> Result<(), String> {
        let name = profile_name(name)?;
        let profiles = self.profiles()?;
        let profile = self
            .selected_profile(&profiles)
            .ok_or("No profile selected")?;
        self.query(&["instance", "rename", &profile.id, name, "--json"])?;
        Ok(())
    }
    fn config(&self, profile: &Profile) -> Result<Value, String> {
        let path = profile.home.join("config/turtleclient.json");
        if !path.exists() {
            return Ok(serde_json::json!({}));
        }
        let value: Value = serde_json::from_slice(&fs::read(&path).map_err(err)?)
            .map_err(|e| format!("{}: {e}", path.display()))?;
        if !value.is_object() {
            return Err("Invalid TurtleClient configuration; existing file preserved".into());
        }
        Ok(value)
    }
    pub(super) fn modules(&self, profile: &Profile) -> Result<ModuleSettings, String> {
        let value = self.config(profile)?;
        let (zoom_key, zoom_enabled) = decode_binding(value["zoom_key"].as_str(), "c");
        let (emote_key, emotes_enabled) = decode_binding(value["emote_key"].as_str(), "b");
        Ok(ModuleSettings {
            zoom_key,
            zoom_enabled,
            emote_key,
            emotes_enabled,
            capes: value["capes"].as_bool().unwrap_or(true),
            smooth_zoom: value["smooth_zoom"].as_bool().unwrap_or(true),
            zoom_factor: value["zoom_factor"]
                .as_f64()
                .unwrap_or(4.0)
                .clamp(2.0, 10.0),
        })
    }
    fn write_modules(&self, settings: ModuleSettings) -> Result<ModuleSettings, String> {
        if !settings.zoom_factor.is_finite() || !(2.0..=10.0).contains(&settings.zoom_factor) {
            return Err("Zoom must be between 2 and 10".into());
        }
        if settings.zoom_key == settings.emote_key {
            return Err("Zoom and emotes need different keys".into());
        }
        let zoom = binding(&settings.zoom_key, settings.zoom_enabled)?;
        let emote = binding(&settings.emote_key, settings.emotes_enabled)?;
        let profiles = self.profiles()?;
        let profile = self
            .selected_profile(&profiles)
            .ok_or("No profile selected")?;
        let mut value = self.config(profile)?;
        value["zoom_key"] = zoom.into();
        value["emote_key"] = emote.into();
        value["zoom_factor"] = settings.zoom_factor.into();
        value["smooth_zoom"] = settings.smooth_zoom.into();
        value["capes"] = settings.capes.into();
        let path = profile.home.join("config/turtleclient.json");
        fs::create_dir_all(path.parent().ok_or("Missing config directory")?).map_err(err)?;
        let temp = path.with_extension("json.tmp");
        fs::write(&temp, serde_json::to_vec_pretty(&value).map_err(err)?).map_err(err)?;
        fs::rename(temp, &path).map_err(err)?;
        Ok(settings)
    }
    fn cape_home(&self) -> PathBuf {
        std::env::var_os("TURTLECLIENT_DATA_HOME")
            .map(PathBuf::from)
            .unwrap_or_else(|| self.root.join("Ocean-Rest"))
    }
    fn account_uuid(&self) -> Result<String, String> {
        let value = self.query(&["account", "list", "--json"])?;
        let uuid = value["accounts"]
            .as_array()
            .and_then(|a| a.iter().find(|v| v["selected"] == true))
            .and_then(|a| a["uuid"].as_str())
            .ok_or("Sign in to select a cape")?;
        if !(32..=36).contains(&uuid.len())
            || !uuid.bytes().all(|c| c.is_ascii_hexdigit() || c == b'-')
        {
            return Err("Invalid account UUID".into());
        }
        Ok(uuid.into())
    }
    fn cape(&self) -> Result<Option<Vec<u8>>, String> {
        let Ok(uuid) = self.account_uuid() else {
            return Ok(None);
        };
        let path = self.cape_home().join("data").join(format!("{uuid}.json"));
        if !path.exists() {
            return Ok(None);
        }
        let meta: Value = serde_json::from_slice(&fs::read(path).map_err(err)?).map_err(err)?;
        let file = relative(meta["image_path"].as_str().ok_or("Invalid cape metadata")?)?;
        if file.components().count() != 1 {
            return Err("Invalid cape filename".into());
        }
        let path = self.cape_home().join("capes").join(file);
        if fs::metadata(&path).map_err(err)?.len() > 2 * 1024 * 1024 {
            return Err("Cape image too large".into());
        }
        Ok(Some(fs::read(path).map_err(err)?))
    }
    fn put_cape(&self, data: Vec<u8>) -> Result<(), String> {
        validate_cape(&data)?;
        let uuid = self.account_uuid()?;
        self.api()?;
        let boundary = "TurtleLauncherCapeUpload";
        let mut body = format!("--{boundary}\r\nContent-Disposition: form-data; name=\"file\"; filename=\"cape.png\"\r\nContent-Type: image/png\r\n\r\n").into_bytes();
        body.extend_from_slice(&data);
        body.extend_from_slice(format!("\r\n--{boundary}--\r\n").as_bytes());
        let mut socket = TcpStream::connect_timeout(
            &"127.0.0.1:3000".parse().map_err(err)?,
            Duration::from_secs(3),
        )
        .map_err(err)?;
        socket
            .set_read_timeout(Some(Duration::from_secs(10)))
            .map_err(err)?;
        socket
            .set_write_timeout(Some(Duration::from_secs(10)))
            .map_err(err)?;
        write!(socket, "POST /api/admin/cape/{uuid} HTTP/1.1\r\nHost: localhost\r\nConnection: close\r\nContent-Type: multipart/form-data; boundary={boundary}\r\nContent-Length: {}\r\n\r\n", body.len()).map_err(err)?;
        socket.write_all(&body).map_err(err)?;
        let mut response = String::new();
        socket
            .take(65536)
            .read_to_string(&mut response)
            .map_err(err)?;
        if !response.starts_with("HTTP/1.1 200") {
            return Err("Cape service rejected the upload".into());
        }
        // Verify the service committed the account mapping and the exact uploaded bytes.
        if self.cape()?.as_deref() != Some(data.as_slice()) {
            return Err("Cape service did not save the image".into());
        }
        Ok(())
    }
}
fn validate_cape(data: &[u8]) -> Result<(), String> {
    if data.len() > 2 * 1024 * 1024
        || data.len() < 33
        || &data[..8] != b"\x89PNG\r\n\x1a\n"
        || &data[12..16] != b"IHDR"
    {
        return Err("Choose a PNG cape of at most 2 MB".into());
    }
    let width = u32::from_be_bytes(data[16..20].try_into().map_err(err)?);
    let height = u32::from_be_bytes(data[20..24].try_into().map_err(err)?);
    if width != 64 || height != 32 {
        return Err("Cape must be 64 × 32 pixels".into());
    }
    Ok(())
}

#[tauri::command]
pub async fn save_modules(
    settings: ModuleSettings,
    state: tauri::State<'_, Shared>,
) -> Result<ModuleSettings, String> {
    // Acquire the same operation guard as start_action; do not race with a running game,
    // profile switch or another config write.
    {
        let mut progress = state.progress.lock().unwrap();
        if progress.action.is_some() {
            return Err("Wait until the current operation has finished".into());
        }
        progress.action = Some("save-modules".into());
    }
    let b = state.backend.clone();
    let p = state.progress.clone();
    let result = tauri::async_runtime::spawn_blocking(move || b.write_modules(settings))
        .await
        .map_err(err);
    p.lock().unwrap().action = None;
    result?
}
#[tauri::command]
pub async fn read_cape(state: tauri::State<'_, Shared>) -> Result<Option<Vec<u8>>, String> {
    let b = state.backend.clone();
    tauri::async_runtime::spawn_blocking(move || b.cape())
        .await
        .map_err(err)?
}
#[tauri::command]
pub async fn upload_cape(data: Vec<u8>, state: tauri::State<'_, Shared>) -> Result<(), String> {
    {
        let mut progress = state.progress.lock().unwrap();
        if progress.action.is_some() {
            return Err("Wait until the current operation has finished".into());
        }
        progress.action = Some("upload-cape".into());
    }
    let b = state.backend.clone();
    let p = state.progress.clone();
    let result = tauri::async_runtime::spawn_blocking(move || b.put_cape(data))
        .await
        .map_err(err);
    p.lock().unwrap().action = None;
    result?
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn disabled_shortcuts_round_trip_and_cannot_match_physical_keys() {
        let disabled = binding("left.shift", false).unwrap();
        assert_ne!(disabled, "key.keyboard.left.shift");
        assert_eq!(
            decode_binding(Some(&disabled), "c"),
            ("left.shift".into(), false)
        );
        assert_eq!(
            binding("left.shift", true).unwrap(),
            "key.keyboard.left.shift"
        );
        assert!(binding("c\n", true).is_err());
        assert!(binding("disabled.c", true).is_err());
    }
    #[test]
    fn profile_names_cannot_be_paths_or_cli_flags() {
        for invalid in ["", "  ", "../world", "a\\b", "--json", "x\n"] {
            assert!(profile_name(invalid).is_err(), "{invalid}");
        }
        assert_eq!(profile_name("  Meine Welt  ").unwrap(), "Meine Welt");
    }
    #[test]
    fn rejects_wrong_cape_format_and_dimensions() {
        assert!(validate_cape(b"not a png").is_err());
        let mut header = vec![0; 33];
        header[..8].copy_from_slice(b"\x89PNG\r\n\x1a\n");
        header[12..16].copy_from_slice(b"IHDR");
        header[16..20].copy_from_slice(&64u32.to_be_bytes());
        header[20..24].copy_from_slice(&32u32.to_be_bytes());
        assert!(validate_cape(&header).is_ok());
        header[20..24].copy_from_slice(&64u32.to_be_bytes());
        assert!(validate_cape(&header).is_err());
    }
}
