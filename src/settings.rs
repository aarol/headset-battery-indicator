use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Settings {
    #[serde(default = "default_overlay_enabled")]
    pub overlay_enabled: bool,
}

fn default_overlay_enabled() -> bool {
    true
}

impl Default for Settings {
    fn default() -> Self {
        Self {
            overlay_enabled: true,
        }
    }
}

pub fn load() -> Settings {
    let path = match settings_path() {
        Some(p) => p,
        None => return Settings::default(),
    };

    let bytes = match std::fs::read(&path) {
        Ok(b) => b,
        Err(_) => return Settings::default(),
    };

    serde_json::from_slice(&bytes).unwrap_or_default()
}

pub fn save(settings: &Settings) -> std::io::Result<()> {
    let path = settings_path().ok_or_else(|| {
        std::io::Error::new(std::io::ErrorKind::NotFound, "config dir not found")
    })?;

    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent)?;
    }

    let bytes = serde_json::to_vec_pretty(settings)
        .map_err(|e| std::io::Error::new(std::io::ErrorKind::InvalidData, e))?;

    std::fs::write(path, bytes)
}

fn settings_path() -> Option<std::path::PathBuf> {
    let mut dir = dirs::config_dir()?;
    dir.push("headset-battery-indicator");
    dir.push("settings.json");
    Some(dir)
}
