use serde::{Deserialize, Serialize};
use std::fs;
use std::path::PathBuf;

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct Settings {
    pub notifications_enabled: bool,
}

impl Default for Settings {
    fn default() -> Self {
        Self {
            notifications_enabled: true,
        }
    }
}

impl Settings {
    pub fn load() -> Self {
        let config_path = Self::get_config_path();
        if let Some(path) = &config_path {
            if let Ok(contents) = fs::read_to_string(path) {
                if let Ok(settings) = toml::from_str(&contents) {
                    return settings;
                }
            }
        }
        Self::default()
    }

    pub fn save(&self) {
        if let Some(path) = Self::get_config_path() {
            if let Some(parent) = path.parent() {
                let _ = fs::create_dir_all(parent);
            }
            if let Ok(toml) = toml::to_string(self) {
                let _ = fs::write(path, toml);
            }
        }
    }

    fn get_config_path() -> Option<PathBuf> {
        dirs::config_dir().map(|mut path| {
            path.push("headset-battery-indicator");
            path.push("settings.toml");
            path
        })
    }
}
