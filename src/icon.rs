use anyhow::Context;

use winit::window::Theme;

use crate::{headset_control::BatteryStatus, settings::Settings};

type CacheParams = (Settings, Theme, isize, BatteryStatus);

pub struct IconProvider {
    text_renderer: Option<gdi::TextIconRenderer>,
    icon_cache: Option<(tray_icon::Icon, CacheParams)>,
}

impl IconProvider {
    pub fn new() -> Self {
        let text_renderer = gdi::TextIconRenderer::new("Segoe UI", 16.0)
            .inspect_err(|err| {
                log::error!(
                    "Failed to create GDI+ text renderer, using fallback bitmap.\nError: {err:?}"
                )
            })
            .ok();
        Self {
            text_renderer,
            icon_cache: None,
        }
    }

    pub fn get_icon(
        &mut self,
        theme: Theme,
        battery_percent: isize,
        state: BatteryStatus,
        settings: Settings,
    ) -> anyhow::Result<tray_icon::Icon> {
        if let Some((icon, cache_params)) = &self.icon_cache
            && *cache_params == (settings, theme, battery_percent, state)
        {
            return Ok(icon.clone());
        }

        log::trace!("Re-render icon");

        let icon = if settings.use_number_icon {
            if let Some(renderer) = &mut self.text_renderer {
                let text = match state {
                    BatteryStatus::Unavailable => "--",
                    _ => &battery_percent.min(99).to_string(), // Don't show 100 as it would overflow
                };

                renderer
                    .render(text, theme)
                    .or_else(|_| fallback::generate_number_icon(theme, battery_percent, state))?
            } else {
                fallback::generate_number_icon(theme, battery_percent, state)?
            }
        } else {
            let res_id = battery_res_id_for(theme, battery_percent, state);

            tray_icon::Icon::from_resource(res_id, None)
                .with_context(|| format!("loading icon from resource {res_id}"))?
        };

        self.icon_cache = Some((icon.clone(), (settings, theme, battery_percent, state)));
        Ok(icon)
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

// The older method using a fixed bit-map. It's more blocky compared to the GDI version below, but it's used as a fallback in case GDI fails
mod fallback {
    use anyhow::Context;
    use winit::window::Theme;

    use crate::headset_control::BatteryStatus;

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

    // Hardcoded digit bitmaps (6x12 pixels each)
    // Each digit is represented as 12 rows, each row is a u8 where the lower 6 bits represent pixels
    // 1 = pixel on, 0 = pixel off
    const DIGIT_BITMAPS: [[u8; 12]; 10] = [
        // 0
        [
            0b111111, 0b110011, 0b110011, 0b110011, 0b110011, 0b110011, 0b110011, 0b110011,
            0b110011, 0b110011, 0b111111, 0b000000,
        ],
        // 1
        [
            0b001100, 0b001100, 0b001100, 0b001100, 0b001100, 0b001100, 0b001100, 0b001100,
            0b001100, 0b001100, 0b001100, 0b000000,
        ],
        // 2
        [
            0b111111, 0b000011, 0b000011, 0b000011, 0b000011, 0b111111, 0b110000, 0b110000,
            0b110000, 0b110000, 0b111111, 0b000000,
        ],
        // 3
        [
            0b111111, 0b000011, 0b000011, 0b000011, 0b000011, 0b111111, 0b000011, 0b000011,
            0b000011, 0b000011, 0b111111, 0b000000,
        ],
        // 4
        [
            0b110011, 0b110011, 0b110011, 0b110011, 0b110011, 0b111111, 0b000011, 0b000011,
            0b000011, 0b000011, 0b000011, 0b000000,
        ],
        // 5
        [
            0b111111, 0b110000, 0b110000, 0b110000, 0b110000, 0b111111, 0b000011, 0b000011,
            0b000011, 0b000011, 0b111111, 0b000000,
        ],
        // 6
        [
            0b111111, 0b110000, 0b110000, 0b110000, 0b110000, 0b111111, 0b110011, 0b110011,
            0b110011, 0b110011, 0b111111, 0b000000,
        ],
        // 7
        [
            0b111111, 0b000011, 0b000011, 0b000011, 0b000110, 0b001100, 0b011000, 0b011000,
            0b011000, 0b011000, 0b011000, 0b000000,
        ],
        // 8
        [
            0b111111, 0b110011, 0b110011, 0b110011, 0b110011, 0b111111, 0b110011, 0b110011,
            0b110011, 0b110011, 0b111111, 0b000000,
        ],
        // 9
        [
            0b111111, 0b110011, 0b110011, 0b110011, 0b110011, 0b111111, 0b000011, 0b000011,
            0b000011, 0b000011, 0b111111, 0b000000,
        ],
    ];

    /// Green dot shown above the number while charging. RGBA.
    fn charging_green(theme: Theme) -> [u8; 4] {
        match theme {
            Theme::Dark => [76, 255, 76, 255],
            // Darker green, so the dot stays visible on a light taskbar.
            Theme::Light => [10, 138, 56, 255],
        }
    }

    fn draw_charging_dot(img: &mut [u8], size: u32, color: [u8; 4]) {
        // Small dot centered at the top of the icon.
        for y in 0..2 {
            for x in 7..9 {
                put_pixel(img, size, x, y, color);
            }
        }
    }

    pub fn generate_number_icon(
        theme: Theme,
        battery_percent: isize,
        state: BatteryStatus,
    ) -> anyhow::Result<tray_icon::Icon> {
        let size = 16;
        let mut img = vec![0u8; (size * size * 4) as usize];

        let color = match state {
            BatteryStatus::Charging => charging_green(theme), // Green when charging
            BatteryStatus::Available if battery_percent <= 20 => [255, 0, 0, 255], // Red when low
            // Otherwise, use white for dark theme and black for light theme
            _ if theme == Theme::Dark => [255, 255, 255, 255],
            _ => [0, 0, 0, 255], // Black for light theme
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

            if state == BatteryStatus::Charging {
                draw_charging_dot(&mut img, size, charging_green(theme));
            }
        }

        tray_icon::Icon::from_rgba(img, size, size)
            .context("creating icon from generated image buffer")
    }
}

mod gdi {
    use anyhow::Context;
    use windows::{
        Win32::{
            Graphics::GdiPlus::{
                FlushIntentionSync, GdipCloneStringFormat, GdipCreateBitmapFromScan0,
                GdipCreateFont, GdipCreateFontFamilyFromName, GdipCreateSolidFill, GdipDeleteBrush,
                GdipDeleteFont, GdipDeleteFontFamily, GdipDeleteGraphics, GdipDeleteStringFormat,
                GdipDisposeImage, GdipDrawString, GdipFlush, GdipGetImageGraphicsContext,
                GdipGraphicsClear, GdipSetStringFormatAlign, GdipSetStringFormatFlags,
                GdipSetStringFormatLineAlign, GdipSetTextRenderingHint,
                GdipStringFormatGetGenericTypographic, GdiplusShutdown, GdiplusStartup,
                GdiplusStartupInput, GpBitmap, GpBrush, GpFont, GpFontFamily, GpGraphics, GpImage,
                GpSolidFill, GpStringFormat, RectF, Status, StringAlignmentCenter,
                TextRenderingHintAntiAliasGridFit, TextRenderingHintClearTypeGridFit, UnitPixel,
            },
            UI::WindowsAndMessaging::{GetSystemMetrics, SM_CXSMICON},
        },
        core::PCWSTR,
    };
    use winit::window::Theme;
    // PixelFormat32bppARGB = 10 | (32 << 8) | Alpha | GDI | Canonical
    const PIXEL_FORMAT_32BPP_ARGB: i32 = 0x0026200A;

    pub struct TextIconRenderer {
        token: usize,
        size: i32,
        pixels: Vec<u8>,
        bitmap: *mut GpBitmap,
        graphics: *mut GpGraphics,
        family: *mut GpFontFamily,
        font: *mut GpFont,
        brush_white: *mut GpSolidFill,
        brush_black: *mut GpSolidFill,
        format: *mut GpStringFormat,
    }

    fn check(status: Status, what: &str) -> anyhow::Result<()> {
        if status == windows::Win32::Graphics::GdiPlus::Ok {
            Ok(())
        } else {
            anyhow::bail!("{what} failed: GDI+ status {}", status.0)
        }
    }

    impl TextIconRenderer {
        /// `px`: font height in pixels
        pub fn new(font_name: &str, px: f32) -> anyhow::Result<Self> {
            let size = unsafe { GetSystemMetrics(SM_CXSMICON) }.max(16);
            let mut this = Self {
                token: 0,
                size,
                pixels: vec![0u8; (size * size * 4) as usize],
                bitmap: std::ptr::null_mut(),
                graphics: std::ptr::null_mut(),
                family: std::ptr::null_mut(),
                font: std::ptr::null_mut(),
                brush_black: std::ptr::null_mut(),
                brush_white: std::ptr::null_mut(),
                format: std::ptr::null_mut(),
            };
            // If anything below fails, returning early drops `this`,
            // and Drop frees whatever was created so far.

            unsafe {
                let input = GdiplusStartupInput {
                    GdiplusVersion: 1,
                    ..Default::default()
                };
                check(
                    GdiplusStartup(&mut this.token, &input, std::ptr::null_mut()),
                    "GdiplusStartup",
                )?;

                check(
                    GdipCreateSolidFill(0xFF000000, &mut this.brush_black),
                    "GdipCreateSolidFill",
                )?;
                check(
                    GdipCreateSolidFill(0xFFFFFFFF, &mut this.brush_white),
                    "GdipCreateSolidFill",
                )?;

                check(
                    GdipCreateBitmapFromScan0(
                        size,
                        size,
                        size * 4,
                        PIXEL_FORMAT_32BPP_ARGB,
                        Some(this.pixels.as_mut_ptr()),
                        &mut this.bitmap,
                    ),
                    "GdipCreateBitmapFromScan0",
                )?;

                check(
                    GdipGetImageGraphicsContext(this.bitmap as *mut GpImage, &mut this.graphics),
                    "GdipGetImageGraphicsContext",
                )?;

                let name: Vec<u16> = font_name.encode_utf16().chain(Some(0)).collect();
                check(
                    GdipCreateFontFamilyFromName(
                        PCWSTR(name.as_ptr()),
                        std::ptr::null_mut(),
                        &mut this.family,
                    ),
                    "GdipCreateFontFamilyFromName (is the font installed?)",
                )?;

                // style 0 = regular, size in pixels
                check(
                    GdipCreateFont(this.family, px, 0, UnitPixel, &mut this.font),
                    "GdipCreateFont",
                )?;

                // Typographic format = no side padding. Clone it so we can set our own flags.
                let mut generic: *mut GpStringFormat = std::ptr::null_mut();
                check(
                    GdipStringFormatGetGenericTypographic(&mut generic),
                    "GdipStringFormatGetGenericTypographic",
                )?;
                check(
                    GdipCloneStringFormat(generic, &mut this.format),
                    "GdipCloneStringFormat",
                )?;
                check(GdipDeleteStringFormat(generic), "GdipDeleteStringFormat")?;

                // NoWrap (0x1000) | NoClip (0x4000)
                check(
                    GdipSetStringFormatFlags(this.format, 0x1000 | 0x4000),
                    "GdipSetStringFormatFlags",
                )?;
                check(
                    GdipSetStringFormatAlign(this.format, StringAlignmentCenter),
                    "GdipSetStringFormatAlign",
                )?;
                check(
                    GdipSetStringFormatLineAlign(this.format, StringAlignmentCenter),
                    "GdipSetStringFormatLineAlign",
                )?;
            }

            Result::Ok(this)
        }

        /// Draws `text` centered on a transparent canvas and returns a tray icon.
        /// When `charging`, a small green dot is drawn at the top center.
        pub fn render(&mut self, text: &str, theme: Theme) -> anyhow::Result<tray_icon::Icon> {
            let wide: Vec<u16> = text.encode_utf16().collect();
            let rect = RectF {
                X: 0.0,
                Y: 0.0,
                Width: self.size as f32,
                Height: self.size as f32,
            };

            unsafe {
                check(
                    GdipGraphicsClear(self.graphics, 0x0000_0000),
                    "GdipGraphicsClear",
                )?;

                // This seems to make the nicest looking text at 100% zoom at least
                let rendering_hint = match theme {
                    Theme::Dark => TextRenderingHintClearTypeGridFit,
                    Theme::Light => TextRenderingHintAntiAliasGridFit,
                };

                check(
                    GdipSetTextRenderingHint(self.graphics, rendering_hint),
                    "GdipSetTextRenderingHint",
                )?;

                let brush = match theme {
                    Theme::Dark => self.brush_white,
                    Theme::Light => self.brush_black,
                };

                check(
                    GdipDrawString(
                        self.graphics,
                        PCWSTR(wide.as_ptr()),
                        wide.len() as i32,
                        self.font,
                        &rect,
                        self.format,
                        brush as *const GpBrush,
                    ),
                    "GdipDrawString",
                )?;

                check(GdipFlush(self.graphics, FlushIntentionSync), "GdipFlush")?;
            }

            // Copy out and convert BGRA -> RGBA
            let mut rgba = self.pixels.clone();
            for p in rgba.as_chunks_mut::<4>().0 {
                p.swap(0, 2);
            }

            tray_icon::Icon::from_rgba(rgba, self.size as u32, self.size as u32)
                .context("Icon::from_rgba")
        }
    }

    impl Drop for TextIconRenderer {
        fn drop(&mut self) {
            unsafe {
                if !self.format.is_null() {
                    GdipDeleteStringFormat(self.format);
                }
                if !self.brush_black.is_null() {
                    GdipDeleteBrush(self.brush_black as *mut GpBrush);
                }
                if !self.brush_white.is_null() {
                    GdipDeleteBrush(self.brush_white as *mut GpBrush);
                }
                if !self.font.is_null() {
                    GdipDeleteFont(self.font);
                }
                if !self.family.is_null() {
                    GdipDeleteFontFamily(self.family);
                }
                if !self.graphics.is_null() {
                    GdipDeleteGraphics(self.graphics);
                }
                if !self.bitmap.is_null() {
                    GdipDisposeImage(self.bitmap as *mut GpImage);
                }
                // `pixels` drops after this body, so the bitmap is gone before its memory.
                if self.token != 0 {
                    GdiplusShutdown(self.token);
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn load_all_icons() {
        let mut icon_provider = IconProvider::new();
        for i in 0..=100 {
            let _ = icon_provider
                .get_icon(
                    Theme::Dark,
                    i,
                    BatteryStatus::Available,
                    Settings::default(),
                )
                .unwrap();
        }
        for i in 0..=100 {
            let _ = icon_provider
                .get_icon(
                    Theme::Light,
                    i,
                    BatteryStatus::Available,
                    Settings::default(),
                )
                .unwrap();
        }
    }
}
