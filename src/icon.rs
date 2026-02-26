use anyhow::Context;
use winit::window::Theme;

use crate::headset_control::BatteryStatus;

pub fn generate_number_icon(
    theme: Theme,
    battery_percent: isize,
    state: BatteryStatus,
    icon_color: Option<[u8; 3]>,
) -> anyhow::Result<tray_icon::Icon> {
    let size = 16;
    let mut img = vec![0u8; (size * size * 4) as usize];

    let color: [u8; 4] = if let Some([r, g, b]) = icon_color {
        [r, g, b, 255]
    } else {
        match state {
            BatteryStatus::Charging => [0, 255, 0, 255], // Green when charging
            BatteryStatus::Available if battery_percent <= 20 => [255, 0, 0, 255], // Red when low
            // Otherwise, use white for dark theme and black for light theme
            _ if theme == Theme::Dark => [255, 255, 255, 255],
            _ => [0, 0, 0, 255], // Black for light theme
        }
    };

    if state == BatteryStatus::Unavailable {
        // Draw two dashes for unavailable state
        // First dash
        for x in 2..6 {
            for y in 6..8 {
                put_pixel(&mut img, size, x, y, color);
            }
        }
        // Second dash
        for x in 10..14 {
            for y in 6..8 {
                put_pixel(&mut img, size, x, y, color);
            }
        }
    } else {
        // Draw the digits
        let percent = battery_percent.clamp(0, 99);
        if percent < 10 {
            // Single digit - draw centered
            draw_digit(&mut img, size, percent as usize, 5, 2, color);
        } else {
            // Two digits - use full width
            let tens = (percent / 10) as usize;
            let ones = (percent % 10) as usize;
            draw_digit(&mut img, size, tens, 0, 2, color);
            draw_digit(&mut img, size, ones, 8, 2, color);
        }
    }

    tray_icon::Icon::from_rgba(img, size, size)
        .context("creating icon from generated image buffer")
}

pub fn load_from_resource(
    theme: Theme,
    battery_percent: isize,
    state: BatteryStatus,
) -> anyhow::Result<tray_icon::Icon> {
    let res_id = battery_res_id_for(theme, battery_percent, state);

    tray_icon::Icon::from_resource(res_id, None)
        .with_context(|| format!("loading icon from resource {res_id}"))
}

fn put_pixel(img: &mut [u8], size: u32, x: u32, y: u32, color: [u8; 4]) {
    let idx = ((y * size + x) * 4) as usize;
    if idx + 3 < img.len() {
        img[idx] = color[0];
        img[idx + 1] = color[1];
        img[idx + 2] = color[2];
        img[idx + 3] = color[3];
    }
}

fn draw_digit(img: &mut [u8], size: u32, digit: usize, x: u32, y: u32, color: [u8; 4]) {
    let bitmap = DIGIT_BITMAPS[digit];

    for (row, &row_data) in bitmap.iter().enumerate() {
        for col in 0..6 {
            if (row_data >> (5 - col)) & 1 == 1 {
                put_pixel(img, size, x + col, y + row as u32, color);
            }
        }
    }
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

// Hardcoded digit bitmaps (6x12 pixels each)
// Each digit is represented as 12 rows, each row is a u8 where the lower 6 bits represent pixels
// 1 = pixel on, 0 = pixel off
const DIGIT_BITMAPS: [[u8; 12]; 10] = [
    // 0
    [
        0b111111, 0b110011, 0b110011, 0b110011, 0b110011, 0b110011, 0b110011, 0b110011, 0b110011,
        0b110011, 0b111111, 0b000000,
    ],
    // 1
    [
        0b001100, 0b001100, 0b001100, 0b001100, 0b001100, 0b001100, 0b001100, 0b001100, 0b001100,
        0b001100, 0b001100, 0b000000,
    ],
    // 2
    [
        0b111111, 0b000011, 0b000011, 0b000011, 0b000011, 0b111111, 0b110000, 0b110000, 0b110000,
        0b110000, 0b111111, 0b000000,
    ],
    // 3
    [
        0b111111, 0b000011, 0b000011, 0b000011, 0b000011, 0b111111, 0b000011, 0b000011, 0b000011,
        0b000011, 0b111111, 0b000000,
    ],
    // 4
    [
        0b110011, 0b110011, 0b110011, 0b110011, 0b110011, 0b111111, 0b000011, 0b000011, 0b000011,
        0b000011, 0b000011, 0b000000,
    ],
    // 5
    [
        0b111111, 0b110000, 0b110000, 0b110000, 0b110000, 0b111111, 0b000011, 0b000011, 0b000011,
        0b000011, 0b111111, 0b000000,
    ],
    // 6
    [
        0b111111, 0b110000, 0b110000, 0b110000, 0b110000, 0b111111, 0b110011, 0b110011, 0b110011,
        0b110011, 0b111111, 0b000000,
    ],
    // 7
    [
        0b111111, 0b000011, 0b000011, 0b000011, 0b000110, 0b001100, 0b011000, 0b011000, 0b011000,
        0b011000, 0b011000, 0b000000,
    ],
    // 8
    [
        0b111111, 0b110011, 0b110011, 0b110011, 0b110011, 0b111111, 0b110011, 0b110011, 0b110011,
        0b110011, 0b111111, 0b000000,
    ],
    // 9
    [
        0b111111, 0b110011, 0b110011, 0b110011, 0b110011, 0b111111, 0b000011, 0b000011, 0b000011,
        0b000011, 0b111111, 0b000000,
    ],
];

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
