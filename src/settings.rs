use anyhow::{Context, Result};
use serde_derive::{Deserialize, Serialize};

// This struct is parsed from either registry settings or headset-battery-indicator-settings.toml if in portable mode.
#[derive(Debug, Clone, Copy, Deserialize, Serialize)]
pub struct Settings {
    #[serde(default, rename = "NotificationsEnabled")]
    pub notifications_enabled: bool,
    #[serde(default, rename = "UseNumberIcon")]
    pub use_number_icon: bool,
}

#[cfg(feature = "portable")]
impl Settings {
    fn settings_file_path() -> Result<std::path::PathBuf> {
        let exe_path = std::env::current_exe().context("getting current executable path")?;
        let settings_path = exe_path
            .parent()
            .context("getting parent directory of executable")?
            .join("headset-battery-indicator-settings.toml");
        Ok(settings_path)
    }

    pub fn load() -> Result<Self> {
        let settings_path = Self::settings_file_path()?;

        if settings_path.exists() {
            let settings: Settings = toml::from_str(&std::fs::read_to_string(&settings_path)?)
                .with_context(|| format!("parsing settings file at {:?}", settings_path))?;
            log::debug!("Loaded settings from file: {:?}", settings);
            Ok(settings)
        } else {
            log::debug!(
                "Settings file not found at {:?}, using default settings",
                settings_path
            );
            Ok(Settings {
                notifications_enabled: true,
                use_number_icon: false,
            })
        }
    }

    pub fn save(&self) -> Result<()> {
        let settings_path = Self::settings_file_path()?;

        let toml_string = toml::to_string_pretty(self).context("serializing settings to TOML")?;
        std::fs::write(&settings_path, toml_string)
            .with_context(|| format!("writing settings to file at {:?}", settings_path))?;

        Ok(())
    }
}

#[cfg(not(feature = "portable"))]
impl Settings {
    pub fn load() -> Result<Self> {
        let hkcu = winreg::RegKey::predef(winreg::enums::HKEY_CURRENT_USER);
        let (key, _) = hkcu
            .create_subkey("Software\\HeadsetBatteryIndicator")
            .context("accessing registry key")?;

        let settings: Settings = key.decode().context("decoding registry values")?;

        log::debug!("Loaded settings: {:?}", settings);
        Ok(settings)
    }

    pub fn save(&self) -> Result<()> {
        let hkcu = winreg::RegKey::predef(winreg::enums::HKEY_CURRENT_USER);
        let (key, _) = hkcu
            .create_subkey("Software\\HeadsetBatteryIndicator")
            .context("accessing registry key")?;

        key.encode(self).context("encoding settings to registry")?;

        Ok(())
    }
}
