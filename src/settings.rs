use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use serde_derive::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum IconType {
    Image,
    TextGdi,
    TextBitmap,
}

impl Serialize for IconType {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        let s = match self {
            IconType::Image => "image",
            IconType::TextGdi => "text_gdi",
            IconType::TextBitmap => "text_bitmap",
        };
        serializer.serialize_str(s)
    }
}

impl<'de> Deserialize<'de> for IconType {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        let s = String::deserialize(deserializer)?;
        match s.as_str() {
            "image" => Ok(IconType::Image),
            "text_gdi" => Ok(IconType::TextGdi),
            "text_bitmap" => Ok(IconType::TextBitmap),
            _ => Err(serde::de::Error::custom(format!(
                "Invalid icon type: {}",
                s
            ))),
        }
    }
}

// This struct is parsed from either registry settings or headset-battery-indicator-settings.toml if in portable mode.
#[derive(Debug, Clone, Copy, Deserialize, Serialize, PartialEq)]
#[serde(default)]
pub struct Settings {
    #[serde(rename = "NotificationsEnabled")]
    pub notifications_enabled: bool,
    #[serde(rename = "UseNumberIcon")]
    pub use_number_icon: bool,
    #[serde(rename = "IconType")]
    pub icon_type: IconType,
}

impl Default for Settings {
    fn default() -> Self {
        Settings {
            notifications_enabled: true,
            use_number_icon: false,
            icon_type: IconType::Image,
        }
    }
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
            let mut settings: Settings = toml::from_str(&std::fs::read_to_string(&settings_path)?)
                .with_context(|| format!("parsing settings file at {:?}", settings_path))?;
            log::debug!("Loaded settings from file: {:?}", settings);

            if settings.use_number_icon {
                // Legacy setting, override icon_type
                settings.icon_type = IconType::TextGdi;
                settings.use_number_icon = false;
            }

            Ok(settings)
        } else {
            log::debug!(
                "Settings file not found at {:?}, using default settings",
                settings_path
            );
            Ok(Settings::default())
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

        let mut settings: Settings = key.decode().context("decoding registry values")?;

        if settings.use_number_icon {
            // Legacy setting, override icon_type
            settings.icon_type = IconType::TextGdi;
            settings.use_number_icon = false;
        }
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
