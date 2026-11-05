use super::*;
use base64::{Engine, engine::general_purpose::STANDARD};

const LIMIT: u64 = 1024 * 1024;

#[derive(Clone, Serialize, Deserialize)]
pub struct PlayerSkin {
    uuid: String,
    skin: Option<String>,
    cape: Option<String>,
    model: String,
    cached: bool,
}

fn texture_url(url: &str) -> Result<String, String> {
    let hash = url
        .strip_prefix("https://textures.minecraft.net/texture/")
        .or_else(|| url.strip_prefix("http://textures.minecraft.net/texture/"))
        .ok_or("Invalid Minecraft texture host")?;
    if hash.len() != 64 || !hash.bytes().all(|c| c.is_ascii_hexdigit()) {
        return Err("Invalid Minecraft texture hash".into());
    }
    Ok(format!("https://textures.minecraft.net/texture/{hash}"))
}

fn fetch(agent: &ureq::Agent, url: &str) -> Result<Vec<u8>, String> {
    let response = agent.get(url).call().map_err(err)?;
    let mut bytes = Vec::new();
    response
        .into_reader()
        .take(LIMIT + 1)
        .read_to_end(&mut bytes)
        .map_err(err)?;
    if bytes.len() as u64 > LIMIT {
        return Err("Minecraft texture response too large".into());
    }
    Ok(bytes)
}

fn png_data(bytes: Vec<u8>, skin: bool) -> Result<String, String> {
    if bytes.len() < 33 || &bytes[..8] != b"\x89PNG\r\n\x1a\n" || &bytes[12..16] != b"IHDR" {
        return Err("Invalid Minecraft skin PNG".into());
    }
    let width = u32::from_be_bytes(bytes[16..20].try_into().map_err(err)?);
    let height = u32::from_be_bytes(bytes[20..24].try_into().map_err(err)?);
    if width != 64 || (height != 32 && (!skin || height != 64)) {
        return Err("Unsupported Minecraft texture dimensions".into());
    }
    Ok(format!("data:image/png;base64,{}", STANDARD.encode(bytes)))
}

fn public_skin(uuid: &str) -> Result<PlayerSkin, String> {
    if uuid.len() != 32 || !uuid.bytes().all(|c| c.is_ascii_hexdigit()) {
        return Err("Invalid Minecraft profile UUID".into());
    }
    let agent = ureq::AgentBuilder::new()
        .timeout(Duration::from_secs(10))
        .redirects(0)
        .build();
    let profile: Value = serde_json::from_slice(&fetch(
        &agent,
        &format!("https://sessionserver.mojang.com/session/minecraft/profile/{uuid}"),
    )?)
    .map_err(err)?;
    let encoded = profile["properties"]
        .as_array()
        .and_then(|p| p.iter().find(|p| p["name"] == "textures"))
        .and_then(|p| p["value"].as_str())
        .ok_or("Minecraft profile has no textures")?;
    let textures: Value =
        serde_json::from_slice(&STANDARD.decode(encoded).map_err(err)?).map_err(err)?;
    let skin = textures["textures"]["SKIN"]["url"]
        .as_str()
        .map(|url| png_data(fetch(&agent, &texture_url(url)?)?, true))
        .transpose()?;
    let cape = textures["textures"]["CAPE"]["url"]
        .as_str()
        .and_then(|url| texture_url(url).ok())
        .and_then(|url| fetch(&agent, &url).ok())
        .and_then(|bytes| png_data(bytes, false).ok());
    Ok(PlayerSkin {
        uuid: uuid.into(),
        skin,
        cape,
        model: if textures["textures"]["SKIN"]["metadata"]["model"] == "slim" {
            "slim"
        } else {
            "default"
        }
        .into(),
        cached: false,
    })
}

impl Backend {
    fn player_skin(&self, refresh: bool) -> Result<Option<PlayerSkin>, String> {
        let accounts = self.query(&["account", "list", "--json"])?;
        let Some(uuid) = accounts["accounts"]
            .as_array()
            .and_then(|list| list.iter().find(|a| a["selected"] == true))
            .and_then(|a| a["uuid"].as_str())
        else {
            return Ok(None);
        };
        let uuid = uuid.replace('-', "");
        if uuid.len() != 32 || !uuid.bytes().all(|c| c.is_ascii_hexdigit()) {
            return Err("Invalid account UUID".into());
        }
        let path = self.root.join("cache/skins").join(format!("{uuid}.json"));
        let cached = fs::metadata(&path)
            .ok()
            .filter(|m| m.len() < 3 * LIMIT)
            .and_then(|_| fs::read(&path).ok())
            .and_then(|bytes| serde_json::from_slice::<PlayerSkin>(&bytes).ok())
            .filter(|skin| skin.uuid == uuid);
        let fresh = fs::metadata(&path)
            .and_then(|m| m.modified())
            .ok()
            .and_then(|time| time.elapsed().ok())
            .is_some_and(|age| age < Duration::from_secs(600));
        if !refresh && fresh {
            if let Some(skin) = cached.as_ref() {
                return Ok(Some(skin.clone()));
            }
        }
        let mut skin = match public_skin(&uuid) {
            Ok(skin) => skin,
            Err(error) => match cached {
                Some(mut skin) => {
                    skin.cached = true;
                    return Ok(Some(skin));
                }
                None => return Err(error),
            },
        };
        // Profiles without a custom skin use their UUID's default from the player's own resources.
        if skin.skin.is_none() {
            let skins = [
                "slim/alex",
                "slim/ari",
                "slim/efe",
                "slim/kai",
                "slim/makena",
                "slim/noor",
                "slim/steve",
                "slim/sunny",
                "slim/zuri",
                "wide/alex",
                "wide/ari",
                "wide/efe",
                "wide/kai",
                "wide/makena",
                "wide/noor",
                "wide/steve",
                "wide/sunny",
                "wide/zuri",
            ];
            let id = u128::from_str_radix(&uuid, 16).map_err(err)?;
            let hilo = ((id >> 64) as u64) ^ id as u64;
            let hash = ((hilo >> 32) as i32) ^ hilo as i32;
            let name = skins[hash.rem_euclid(18) as usize];
            let path = self
                .version_dir()
                .join(".local/horizon-client")
                .join(&self.manifest.minecraft)
                .join("assets/minecraft/textures/entity/player")
                .join(format!("{name}.png"));
            skin.skin = fs::read(path).ok().and_then(|png| png_data(png, true).ok());
            skin.model = if name.starts_with("slim/") {
                "slim"
            } else {
                "default"
            }
            .into();
        }
        if skin.skin.is_some() {
            if let Some(parent) = path.parent() {
                let _ = fs::create_dir_all(parent);
            }
            if let Ok(bytes) = serde_json::to_vec(&skin) {
                let _ = fs::write(path, bytes);
            }
        }
        Ok(Some(skin))
    }
}

#[tauri::command]
pub async fn read_player_skin(
    refresh: bool,
    state: tauri::State<'_, Shared>,
) -> Result<Option<PlayerSkin>, String> {
    let backend = state.backend.clone();
    tauri::async_runtime::spawn_blocking(move || backend.player_skin(refresh))
        .await
        .map_err(err)?
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn only_official_texture_hashes_are_allowed() {
        let hash = "a".repeat(64);
        assert_eq!(
            texture_url(&format!("http://textures.minecraft.net/texture/{hash}")).unwrap(),
            format!("https://textures.minecraft.net/texture/{hash}")
        );
        for url in [
            "https://example.com/texture/abc",
            "https://textures.minecraft.net.evil/texture/abc",
            "https://textures.minecraft.net/texture/../secret",
        ] {
            assert!(texture_url(url).is_err());
        }
        assert!(public_skin("../account").is_err());
    }
    #[test]
    #[ignore = "Public Mojang service integration; requires network"]
    fn official_public_profile_loads_a_skin() {
        let skin = public_skin("069a79f444e94726a5befca90e38aaf5").unwrap();
        assert!(skin.skin.unwrap().starts_with("data:image/png;base64,"));
        assert_eq!(skin.model, "default");
    }
}
