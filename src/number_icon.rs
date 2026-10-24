use crate::headset_control::BatteryStatus;
use anyhow::{Context, ensure};
use std::ffi::c_void;
use winit::window::Theme;
type Handle = *mut c_void;
#[repr(C)]
struct BitmapInfo {
    size: u32,
    width: i32,
    height: i32,
    planes: u16,
    bits: u16,
    compression: u32,
    image_size: u32,
    xppm: i32,
    yppm: i32,
    used: u32,
    important: u32,
    colors: [u32; 1],
}
#[repr(C)]
struct TextSize {
    width: i32,
    height: i32,
}
#[link(name = "gdi32")]
unsafe extern "system" {
    fn CreateCompatibleDC(dc: Handle) -> Handle;
    fn DeleteDC(dc: Handle) -> i32;
    fn CreateDIBSection(
        dc: Handle,
        info: *const BitmapInfo,
        usage: u32,
        bits: *mut *mut c_void,
        section: Handle,
        offset: u32,
    ) -> Handle;
    fn SelectObject(dc: Handle, object: Handle) -> Handle;
    fn DeleteObject(object: Handle) -> i32;
    fn CreateFontW(
        height: i32,
        width: i32,
        angle: i32,
        orientation: i32,
        weight: i32,
        italic: u32,
        underline: u32,
        strike: u32,
        charset: u32,
        out: u32,
        clip: u32,
        quality: u32,
        pitch: u32,
        face: *const u16,
    ) -> Handle;
    fn SetTextColor(dc: Handle, color: u32) -> u32;
    fn SetBkMode(dc: Handle, mode: i32) -> i32;
    fn GetTextExtentPoint32W(dc: Handle, text: *const u16, count: i32, size: *mut TextSize) -> i32;
    fn TextOutW(dc: Handle, x: i32, y: i32, text: *const u16, count: i32) -> i32;
    fn GdiFlush() -> i32;
}
#[link(name = "user32")]
unsafe extern "system" {
    fn GetSystemMetrics(index: i32) -> i32;
}

pub fn generate_number_icon(
    theme: Theme,
    battery_percent: isize,
    state: BatteryStatus,
) -> anyhow::Result<tray_icon::Icon> {
    let (rgba, size) = render_number_icon(theme, battery_percent, state)?;
    tray_icon::Icon::from_rgba(rgba, size, size).context("creating number icon")
}

fn render_number_icon(
    theme: Theme,
    battery_percent: isize,
    state: BatteryStatus,
) -> anyhow::Result<(Vec<u8>, u32)> {
    let text = if state == BatteryStatus::Unavailable {
        "--".to_owned()
    } else {
        battery_percent.clamp(0, 100).to_string()
    };
    let text: Vec<u16> = text.encode_utf16().collect();
    let face: Vec<u16> = "Segoe UI\0".encode_utf16().collect();
    let size = unsafe { GetSystemMetrics(49) }.max(16); // SM_CXSMICON
    let color = match state {
        BatteryStatus::Charging => [0, 255, 0],
        BatteryStatus::Available if battery_percent <= 20 => [255, 0, 0],
        _ if theme == Theme::Dark => [255, 255, 255],
        _ => [0, 0, 0],
    };
    // Render above native resolution, crop font leading, and downsample the
    // actual ink, keeping the numerals smooth at tray-icon sizes.
    let canvas = 128;
    unsafe {
        let dc = CreateCompatibleDC(std::ptr::null_mut());
        ensure!(!dc.is_null(), "creating text context");
        let mut pixels = std::ptr::null_mut();
        let info = BitmapInfo {
            size: 40,
            width: canvas,
            height: -canvas,
            planes: 1,
            bits: 32,
            compression: 0,
            image_size: 0,
            xppm: 0,
            yppm: 0,
            used: 0,
            important: 0,
            colors: [0],
        };
        let bitmap = CreateDIBSection(dc, &info, 0, &mut pixels, std::ptr::null_mut(), 0);
        if bitmap.is_null() {
            DeleteDC(dc);
            anyhow::bail!("creating text bitmap");
        }
        let old_bitmap = SelectObject(dc, bitmap);
        let result = (|| -> anyhow::Result<_> {
            ensure!(!pixels.is_null(), "accessing text bitmap");
            std::ptr::write_bytes(pixels as *mut u8, 0, (canvas * canvas * 4) as usize);
            SetTextColor(dc, 0x00ffffff);
            SetBkMode(dc, 1);
            let font = CreateFontW(-56, 0, 0, 0, 400, 0, 0, 0, 1, 0, 0, 4, 0, face.as_ptr());
            ensure!(!font.is_null(), "creating UI font");
            let old_font = SelectObject(dc, font);
            let mut extent = TextSize {
                width: 0,
                height: 0,
            };
            let measured = GetTextExtentPoint32W(dc, text.as_ptr(), text.len() as i32, &mut extent);
            let drawn = TextOutW(dc, 0, 0, text.as_ptr(), text.len() as i32);
            SelectObject(dc, old_font);
            DeleteObject(font);
            ensure!(measured != 0 && drawn != 0, "drawing tray numerals");
            GdiFlush();
            let mask =
                std::slice::from_raw_parts(pixels as *const u8, (canvas * canvas * 4) as usize);
            let mut left = canvas;
            let mut top = canvas;
            let mut right = 0;
            let mut bottom = 0;
            for y in 0..canvas {
                for x in 0..canvas {
                    if mask[((y * canvas + x) * 4) as usize] > 0 {
                        left = left.min(x);
                        top = top.min(y);
                        right = right.max(x + 1);
                        bottom = bottom.max(y + 1);
                    }
                }
            }
            ensure!(right > left && bottom > top, "empty text mask");
            let available_height = size;
            let scale = (size as f32 / (right - left) as f32)
                .min(available_height as f32 / (bottom - top) as f32);
            let width = (((right - left) as f32 * scale).floor() as i32).max(1);
            let height = (((bottom - top) as f32 * scale).floor() as i32).max(1);
            let offset_x = (size - width) / 2;
            let offset_y = (available_height - height) / 2;
            let mut rgba = vec![0; (size * size * 4) as usize];
            // Area samples provide smooth grayscale edges on either taskbar theme.
            for y in 0..height {
                for x in 0..width {
                    let mut coverage = 0u32;
                    for sy in 0..4 {
                        for sx in 0..4 {
                            let px = left
                                + (((x as f32 + (sx as f32 + 0.5) / 4.0) / width as f32)
                                    * (right - left) as f32)
                                    as i32;
                            let py = top
                                + (((y as f32 + (sy as f32 + 0.5) / 4.0) / height as f32)
                                    * (bottom - top) as f32)
                                    as i32;
                            coverage += mask[((py * canvas + px) * 4) as usize] as u32;
                        }
                    }
                    let index = (((y + offset_y) * size + x + offset_x) * 4) as usize;
                    rgba[index..index + 4].copy_from_slice(&[
                        color[0],
                        color[1],
                        color[2],
                        (coverage / 16) as u8,
                    ]);
                }
            }
            Ok((rgba, size as u32))
        })();
        SelectObject(dc, old_bitmap);
        DeleteObject(bitmap);
        DeleteDC(dc);
        result
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn all_percentages_render_visible_antialiased_text() {
        for theme in [Theme::Light, Theme::Dark] {
            for state in [
                BatteryStatus::Available,
                BatteryStatus::Charging,
                BatteryStatus::Unavailable,
            ] {
                for percent in 0..=100 {
                    let (pixels, size) = render_number_icon(theme, percent, state).unwrap();
                    assert_eq!(pixels.len(), (size * size * 4) as usize);
                    let alpha: Vec<_> = pixels.chunks_exact(4).map(|p| p[3]).collect();
                    assert!(alpha.iter().any(|&a| a > 0), "blank text at {percent}");
                    assert!(alpha.contains(&0), "missing transparent background");
                    assert!(
                        state == BatteryStatus::Unavailable
                            || alpha.iter().any(|&a| a > 0 && a < 255),
                        "missing antialiasing at {percent}, {state:?}, {theme:?}"
                    );
                }
            }
        }
    }

    #[test]
    fn full_charge_is_distinct_and_out_of_range_values_are_clamped() {
        let render = |percent| {
            render_number_icon(Theme::Dark, percent, BatteryStatus::Available)
                .unwrap()
                .0
        };
        assert_ne!(render(99), render(100));
        assert_eq!(render(100), render(101));
        assert_eq!(render(0), render(-1));
    }
}
