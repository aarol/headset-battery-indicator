use anyhow::Context;
use winit::window::Theme;

use crate::headset_control::BatteryStatus;

pub use crate::number_icon::generate_number_icon;

pub fn load_from_resource(
    theme: Theme,
    battery_percent: isize,
    state: BatteryStatus,
) -> anyhow::Result<tray_icon::Icon> {
    let res_id = battery_res_id_for(theme, battery_percent, state);

    tray_icon::Icon::from_resource(res_id, None)
        .with_context(|| format!("loading icon from resource {res_id}"))
}

fn battery_res_id_for(theme: Theme, battery_percent: isize, state: BatteryStatus) -> u16 {
    let level = match battery_percent {
        -1 => 1,
        0..=12 => 1,  // 0%
        13..=37 => 2, // 25%
        38..=62 => 3, // 50%
        63..=87 => 4, // 75%
        _ => 5,       // 100%
    };

    // light mode icons are (10,20,...,50)
    // dark mode icons are (15,25,...,55)
    let theme_offset: u16 = if theme == Theme::Light { 5 } else { 0 };
    // Charging icons are at icon id + 1
    let charging_offset = (state == BatteryStatus::Charging) as u16;

    if state == BatteryStatus::Unavailable {
        10 + theme_offset
    } else {
        level * 10 + theme_offset + charging_offset
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn load_all_icons() {
        for i in 0..=100 {
            let _ = load_from_resource(Theme::Dark, i, BatteryStatus::Available);
        }
        for i in 0..=100 {
            let _ = load_from_resource(Theme::Light, i, BatteryStatus::Available);
        }
    }
}
