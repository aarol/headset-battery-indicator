use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use winreg::enums::HKEY_CURRENT_USER;

#[derive(Debug, Clone, Copy, Deserialize, Serialize)]
pub struct Settings {
    #[serde(default, rename="NotificationsEnabled")]
    pub notifications_enabled: bool,
    #[serde(default, rename="UseNumberIcon")]
    pub use_number_icon: bool,
}

impl Settings {
    pub fn load() -> Result<Self> {
        let hkcu = winreg::RegKey::predef(HKEY_CURRENT_USER);
        let (key, _) = hkcu
            .create_subkey("Software\\HeadsetBatteryIndicator")
            .context("accessing registry key")?;

        let settings: Settings = key.decode().context("decoding registry values")?;

        log::debug!("Loaded settings: {:?}", settings);
        Ok(settings)
    }

    pub fn save(&self) -> Result<()> {
        let hkcu = winreg::RegKey::predef(HKEY_CURRENT_USER);
        let (key, _) = hkcu
            .create_subkey("Software\\HeadsetBatteryIndicator")
            .context("accessing registry key")?;

        key.encode(self).context("encoding settings to registry")?;

        Ok(())
    }
}
