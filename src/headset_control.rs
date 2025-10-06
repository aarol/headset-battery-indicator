use std::os::windows::process::CommandExt;
use std::process;
use std::process::Stdio;

use anyhow::Context;
use serde_derive::Deserialize;
use serde_derive::Serialize;

use crate::lang;
use crate::lang::Key::*;

// const CREATE_NO_WINDOW: u32 = 0x08000000;
const DETACHED_PROCESS: u32 = 0x00000008;

pub fn query_devices(vec: &mut Vec<Device>) -> anyhow::Result<()> {
    let res = process::Command::new("./headsetcontrol.exe")
        .args(&["--battery", "--output", "json"])
        .stdout(Stdio::piped())
        .creation_flags(DETACHED_PROCESS)
        .output()
        .context("Failed to execute headsetcontrol.exe")?;

    let response: Output =
        serde_json::from_slice(&res.stdout).context("Invalid JSON from headsetcontrol")?;

    vec.clear();
    for device in response.devices {
        if device.capabilities_str.iter().any(|cap| cap == "battery") {
            vec.push(device);
        }
    }

    Ok(())
}

#[derive(Default, Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Output {
    // pub name: String,
    // pub version: String,
    // #[serde(rename = "api_version")]
    // pub api_version: String,
    // #[serde(rename = "hidapi_version")]
    // pub hidapi_version: String,
    // pub device_count: i64,
    pub devices: Vec<Device>,
}

#[derive(Default, Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Device {
    pub status: String,
    // pub device: String,
    // pub vendor: String,
    pub product: String,
    #[serde(rename = "id_vendor")]
    pub id_vendor: String,
    #[serde(rename = "id_product")]
    pub id_product: String,
    // pub capabilities: Vec<String>,
    #[serde(rename = "capabilities_str")]
    pub capabilities_str: Vec<String>,
    pub battery: Battery,
    // pub equalizer: Equalizer,
    // #[serde(rename = "equalizer_presets_count")]
    // pub equalizer_presets_count: i64,
    // #[serde(rename = "equalizer_presets")]
    // pub equalizer_presets: EqualizerPresets,
    // pub chatmix: i64,
}

impl Device {
    pub fn status_text(&self) -> Option<&'static str> {
        match self.battery.status {
            BatteryState::BatteryCharging => Some(lang::t(device_charging)),
            BatteryState::BatteryAvailable => None,
            BatteryState::BatteryUnavailable => Some(lang::t(battery_unavailable)),
            _ => Some(lang::t(device_disconnected)),
        }
    }
}

impl std::fmt::Display for Device {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        if self.battery.level > 0 {
            write!(
                f,
                "{name}: {battery}% {remaining}",
                name = self.product,
                battery = self.battery.level,
                remaining = lang::t(battery_remaining)
            )?;
        } else {
            write!(f, "{}", self.product)?;
        }

        if let Some(status) = self.status_text() {
            write!(f, " {status}")?;
        }

        Ok(())
    }
}

#[derive(Default, Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Battery {
    pub status: BatteryState,
    /// percentage in range 0-100
    pub level: isize,
}

// #[derive(Default, Debug, Clone, PartialEq, Serialize, Deserialize)]
// #[serde(rename_all = "camelCase")]
// pub struct Equalizer {
//     pub bands: i64,
//     pub baseline: i64,
//     pub step: f64,
//     pub min: i64,
//     pub max: i64,
// }

// #[derive(Default, Debug, Clone, PartialEq, Serialize, Deserialize)]
// #[serde(rename_all = "camelCase")]
// pub struct EqualizerPresets {
//     pub flat: Vec<f64>,
//     pub bass: Vec<f64>,
//     pub focus: Vec<f64>,
//     pub smiley: Vec<f64>,
// }

#[derive(Default, Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum BatteryState {
    #[default]
    BatteryUnavailable,
    BatteryCharging,
    BatteryAvailable,
    BatteryHiderror,
    BatteryTimeout,
}
