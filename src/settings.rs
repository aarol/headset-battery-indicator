use anyhow::{Context, Result};
use winreg::enums::HKEY_CURRENT_USER;

#[derive(Debug, Clone, Copy)]
pub struct Settings {
    pub notifications_enabled: bool,
    pub use_number_icon: bool,
    pub icon_color: Option<[u8; 3]>,
}

impl Settings {
    pub fn load() -> Result<Self> {
        let hkcu = winreg::RegKey::predef(HKEY_CURRENT_USER);
        let (key, _) = hkcu
            .create_subkey("Software\\HeadsetBatteryIndicator")
            .context("accessing registry key")?;

        let notifications_enabled: u32 = key.get_value("NotificationsEnabled").unwrap_or_default();
        let use_number_icon: u32 = key.get_value("UseNumberIcon").unwrap_or_default();
        let icon_color: Option<[u8; 3]> = match key.get_value::<u32, _>("IconColor") {
            Ok(v) => Some([
                (v & 0xFF) as u8,
                ((v >> 8) & 0xFF) as u8,
                ((v >> 16) & 0xFF) as u8,
            ]),
            Err(_) => None,
        };

        let settings = Self {
            notifications_enabled: notifications_enabled != 0,
            use_number_icon: use_number_icon != 0,
            icon_color,
        };
        log::debug!("Loaded settings: {:?}", settings);
        Ok(settings)
    }

    pub fn save(&self) -> Result<()> {
        let hkcu = winreg::RegKey::predef(HKEY_CURRENT_USER);
        let (key, _) = hkcu
            .create_subkey("Software\\HeadsetBatteryIndicator")
            .context("accessing registry key")?;

        key.set_value("NotificationsEnabled", &(self.notifications_enabled as u32))
            .context("setting NotificationsEnabled value")?;

        key.set_value("UseNumberIcon", &(self.use_number_icon as u32))
            .context("setting UseNumberIcon value")?;

        match self.icon_color {
            Some([r, g, b]) => {
                let colorref = r as u32 | ((g as u32) << 8) | ((b as u32) << 16);
                key.set_value("IconColor", &colorref)
                    .context("setting IconColor value")?;
            }
            None => {
                let _ = key.delete_value("IconColor");
            }
        }

        Ok(())
    }
}
