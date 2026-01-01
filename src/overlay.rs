use std::sync::Arc;
use std::sync::OnceLock;
use std::time::{Duration, Instant};

use font_kit::canvas::{Canvas, Format, RasterizationOptions};
use font_kit::family_name::FamilyName;
use font_kit::hinting::HintingOptions;
use font_kit::properties::Properties;
use font_kit::source::SystemSource;
use pathfinder_geometry::transform2d::Transform2F;
use pathfinder_geometry::vector::Vector2I;
use pathfinder_geometry::vector::Vector2F;
use softbuffer::Surface;
use tiny_skia::{FillRule, Paint, PathBuilder, Pixmap, Rect, Transform};
use winit::window::Window;

use svgtypes::{PathParser, PathSegment};

const BATTERY_OUTLINE_SVG: &str = include_str!("MaterialSymbolsBatteryAndroidFrame1.svg");

fn extract_svg_path_data(svg: &str) -> Option<&str> {
    if let Some(start) = svg.find("d=\"") {
        let after_d = &svg[start + 3..];
        if let Some(end) = after_d.find('"') {
            return Some(&after_d[..end]);
        }
    }
    None
}

static BATTERY_OUTLINE_SVG_D: OnceLock<&'static str> = OnceLock::new();

fn battery_svg_path_data() -> &'static str {
    *BATTERY_OUTLINE_SVG_D.get_or_init(|| {
        extract_svg_path_data(BATTERY_OUTLINE_SVG)
            .expect("Failed to extract SVG path data from MaterialSymbolsBatteryAndroidFrame1.svg")
    })
}

static BATTERY_OUTLINE_PATH: OnceLock<tiny_skia::Path> = OnceLock::new();
static SYSTEM_FONT_HANDLE: OnceLock<Option<font_kit::handle::Handle>> = OnceLock::new();

fn parse_svg_path_to_tiny_skia(d: &str) -> Result<tiny_skia::Path, String> {
    let mut pb = PathBuilder::new();
    let mut cur = (0.0f32, 0.0f32);
    let mut sub_start = (0.0f32, 0.0f32);
    let mut last_quad_ctrl: Option<(f32, f32)> = None;

    for seg in PathParser::from(d) {
        let seg = seg.map_err(|e| format!("SVG path parse error: {e}"))?;
        match seg {
            PathSegment::MoveTo { abs, x, y } => {
                let (nx, ny) = if abs {
                    (x as f32, y as f32)
                } else {
                    (cur.0 + x as f32, cur.1 + y as f32)
                };
                pb.move_to(nx, ny);
                cur = (nx, ny);
                sub_start = (nx, ny);
                last_quad_ctrl = None;
            }
            PathSegment::LineTo { abs, x, y } => {
                let (nx, ny) = if abs {
                    (x as f32, y as f32)
                } else {
                    (cur.0 + x as f32, cur.1 + y as f32)
                };
                pb.line_to(nx, ny);
                cur = (nx, ny);
                last_quad_ctrl = None;
            }
            PathSegment::HorizontalLineTo { abs, x } => {
                let nx = if abs { x as f32 } else { cur.0 + x as f32 };
                pb.line_to(nx, cur.1);
                cur = (nx, cur.1);
                last_quad_ctrl = None;
            }
            PathSegment::VerticalLineTo { abs, y } => {
                let ny = if abs { y as f32 } else { cur.1 + y as f32 };
                pb.line_to(cur.0, ny);
                cur = (cur.0, ny);
                last_quad_ctrl = None;
            }
            PathSegment::Quadratic { abs, x1, y1, x, y } => {
                let (cx, cy, nx, ny) = if abs {
                    (x1 as f32, y1 as f32, x as f32, y as f32)
                } else {
                    (
                        cur.0 + x1 as f32,
                        cur.1 + y1 as f32,
                        cur.0 + x as f32,
                        cur.1 + y as f32,
                    )
                };
                pb.quad_to(cx, cy, nx, ny);
                last_quad_ctrl = Some((cx, cy));
                cur = (nx, ny);
            }
            PathSegment::SmoothQuadratic { abs, x, y } => {
                let (nx, ny) = if abs {
                    (x as f32, y as f32)
                } else {
                    (cur.0 + x as f32, cur.1 + y as f32)
                };

                let (cx, cy) = if let Some((pcx, pcy)) = last_quad_ctrl {
                    // Reflect previous quad control point across current point.
                    (2.0 * cur.0 - pcx, 2.0 * cur.1 - pcy)
                } else {
                    cur
                };

                pb.quad_to(cx, cy, nx, ny);
                last_quad_ctrl = Some((cx, cy));
                cur = (nx, ny);
            }
            PathSegment::ClosePath { .. } => {
                pb.close();
                cur = sub_start;
                last_quad_ctrl = None;
            }
            other => {
                return Err(format!("Unsupported SVG path segment: {other:?}"));
            }
        }
    }

    pb.finish().ok_or_else(|| "Failed to build tiny-skia path".to_string())
}

fn battery_outline_path() -> &'static tiny_skia::Path {
    BATTERY_OUTLINE_PATH.get_or_init(|| {
        parse_svg_path_to_tiny_skia(battery_svg_path_data())
            .expect("Failed to parse embedded battery SVG path")
    })
}

fn system_font_handle() -> Option<&'static font_kit::handle::Handle> {
    SYSTEM_FONT_HANDLE
        .get_or_init(|| {
            let source = SystemSource::new();
            let families = [
                FamilyName::Title("Readex Pro".to_string()),
                FamilyName::Title("Segoe UI".to_string()),
                FamilyName::SansSerif,
            ];
            let mut props = Properties::new();
            props.weight = font_kit::properties::Weight::SEMIBOLD;
            source.select_best_match(&families, &props).ok()
        })
        .as_ref()
}

fn draw_text(
    pixmap: &mut Pixmap,
    text: &str,
    x: f32,
    y: f32,
    px_size: f32,
    rgba: (u8, u8, u8, u8),
) {
    let Some(handle) = system_font_handle() else {
        return;
    };

    let Ok(font) = handle.load() else {
        return;
    };

    let metrics = font.metrics();
    let scale = px_size / metrics.units_per_em as f32;
    let ascent = metrics.ascent as f32 * scale;
    let baseline_y = y + ascent;

    let mut caret_x = x;
    let color_a = rgba.3 as f32 / 255.0;

    for ch in text.chars() {
        let Some(gid) = font.glyph_for_char(ch) else {
            continue;
        };

        let local_transform = Transform2F::default();
        let local_bounds = match font.raster_bounds(
            gid,
            px_size,
            local_transform,
            HintingOptions::None,
            RasterizationOptions::GrayscaleAa,
        ) {
            Ok(b) => b,
            Err(_) => {
                if let Ok(advance) = font.advance(gid) {
                    caret_x += advance.x() * scale;
                }
                continue;
            }
        };

        let shift_transform = Transform2F::from_translation(Vector2F::new(
            -(local_bounds.origin_x() as f32),
            -(local_bounds.origin_y() as f32),
        ));

        let shifted_bounds = font
            .raster_bounds(
                gid,
                px_size,
                shift_transform,
                HintingOptions::None,
                RasterizationOptions::GrayscaleAa,
            )
            .unwrap_or(local_bounds);

        let size = shifted_bounds.size();
        if size.x() <= 0 || size.y() <= 0 {
            if let Ok(advance) = font.advance(gid) {
                caret_x += advance.x() * scale;
            }
            continue;
        }

        let mut canvas = Canvas::new(Vector2I::new(size.x(), size.y()), Format::A8);
        if font
            .rasterize_glyph(
                &mut canvas,
                gid,
                px_size,
                shift_transform,
                HintingOptions::None,
                RasterizationOptions::GrayscaleAa,
            )
            .is_err()
        {
            if let Ok(advance) = font.advance(gid) {
                caret_x += advance.x() * scale;
            }
            continue;
        }

        let w = canvas.size.x() as usize;
        let h = canvas.size.y() as usize;
        if w == 0 || h == 0 {
            if let Ok(advance) = font.advance(gid) {
                caret_x += advance.x() * scale;
            }
            continue;
        }

        let dst_x0 = (x + caret_x + local_bounds.origin_x() as f32).round() as i32;
        let dst_y0 = (baseline_y + local_bounds.origin_y() as f32).round() as i32;

        let pixmap_width = pixmap.width();
        let pix_w = pixmap_width as i32;
        let pix_h = pixmap.height() as i32;
        let data = pixmap.data_mut();

        for row in 0..h {
            let py = dst_y0 + row as i32;
            if py < 0 || py >= pix_h {
                continue;
            }
            for col in 0..w {
                let px = dst_x0 + col as i32;
                if px < 0 || px >= pix_w {
                    continue;
                }

                let src_a = (canvas.pixels[row * canvas.stride + col] as f32 / 255.0) * color_a;
                if src_a <= 0.0 {
                    continue;
                }

                let idx = ((py as u32 * pixmap_width + px as u32) * 4) as usize;
                let dst_r = data[idx] as f32;
                let dst_g = data[idx + 1] as f32;
                let dst_b = data[idx + 2] as f32;

                let src_r = rgba.0 as f32;
                let src_g = rgba.1 as f32;
                let src_b = rgba.2 as f32;

                data[idx] = (src_r * src_a + dst_r * (1.0 - src_a)) as u8;
                data[idx + 1] = (src_g * src_a + dst_g * (1.0 - src_a)) as u8;
                data[idx + 2] = (src_b * src_a + dst_b * (1.0 - src_a)) as u8;
                data[idx + 3] = 255;
            }
        }

        if let Ok(advance) = font.advance(gid) {
            caret_x += advance.x() * scale;
        }
    }
}

fn draw_battery_icon(pixmap: &mut Pixmap, x: f32, y: f32, size: f32, battery_level: isize, is_charging: bool) {
    let scale = size / 24.0;
    let transform = Transform::from_row(scale, 0.0, 0.0, scale, x, y);

    let (r, g, b) = if is_charging {
        (110, 255, 90)
    } else if battery_level < 5 {
        (255, 50, 50)
    } else if battery_level < 15 {
        (255, 165, 0)
    } else {
        (110, 255, 90)
    };

    let pct = (battery_level.max(0).min(100)) as f32 / 100.0;
    let fill_min_x = 4.5;
    let fill_max_x = 17.0;
    let fill_y = 8.8;
    let fill_h = 6.4;
    let fill_w = (fill_max_x - fill_min_x) * pct;

    let mut fill_paint = Paint::default();
    fill_paint.set_color_rgba8(r, g, b, 255);
    fill_paint.anti_alias = true;

    if fill_w > 0.0 {
        if let Some(rect) = Rect::from_xywh(
            x + fill_min_x * scale,
            y + fill_y * scale,
            fill_w * scale,
            fill_h * scale,
        ) {
            pixmap.fill_rect(rect, &fill_paint, Transform::identity(), None);
        }
    }

    let mut outline_paint = Paint::default();
    outline_paint.set_color_rgba8(r, g, b, 255);
    outline_paint.anti_alias = true;
    pixmap.fill_path(
        battery_outline_path(),
        &outline_paint,
        FillRule::Winding,
        transform,
        None,
    );

    if is_charging {
        let mut bolt_paint = Paint::default();
        bolt_paint.set_color_rgba8(255, 255, 255, 255);
        bolt_paint.anti_alias = true;

        let mut pb = tiny_skia::PathBuilder::new();
        pb.move_to(14.5, 7.0);
        pb.line_to(9.5, 12.0);
        pb.line_to(12.5, 12.0);
        pb.line_to(10.0, 17.5);
        pb.line_to(15.0, 12.0);
        pb.line_to(12.0, 12.0);
        pb.close();
        
        if let Some(bolt_path) = pb.finish() {
            pixmap.fill_path(&bolt_path, &bolt_paint, FillRule::Winding, transform, None);
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum AnimationState {
    SlidingIn,
    Visible,
    SlidingOut,
    Hidden,
}

pub struct AnimationTimer {
    pub state: AnimationState,
    pub start_time: Instant,
    pub slide_duration: Duration,
    pub visible_duration: Duration,
    pub visible_start: Option<Instant>,
}

impl AnimationTimer {
    pub fn new() -> Self {
        Self {
            state: AnimationState::Hidden,
            start_time: Instant::now(),
            slide_duration: Duration::from_millis(250),
            visible_duration: Duration::from_secs(4),
            visible_start: None,
        }
    }

    pub fn start_slide_in(&mut self) {
        self.state = AnimationState::SlidingIn;
        self.start_time = Instant::now();
        self.visible_start = None;
    }

    pub fn update(&mut self, _is_focused: bool) -> bool {
        match self.state {
            AnimationState::SlidingIn => {
                if self.start_time.elapsed() >= self.slide_duration {
                    self.state = AnimationState::Visible;
                    self.visible_start = Some(Instant::now());
                }
                true
            }
            AnimationState::Visible => {
                if let Some(visible_start) = self.visible_start {
                    if visible_start.elapsed() >= self.visible_duration {
                        self.state = AnimationState::SlidingOut;
                        self.start_time = Instant::now();
                    }
                }
                true
            }
            AnimationState::SlidingOut => {
                if self.start_time.elapsed() >= self.slide_duration {
                    self.state = AnimationState::Hidden;
                    return false;
                }
                true
            }
            AnimationState::Hidden => false,
        }
    }

    pub fn get_progress(&self) -> f32 {
        let elapsed = self.start_time.elapsed().as_secs_f32();
        let duration = self.slide_duration.as_secs_f32();
        (elapsed / duration).min(1.0)
    }
}

pub fn ease_out_quad(t: f32) -> f32 {
    t * (2.0 - t)
}

fn create_rounded_rect(x: f32, y: f32, width: f32, height: f32, radius: f32) -> Option<tiny_skia::Path> {
    let mut pb = PathBuilder::new();
    
    pb.move_to(x + radius, y);
    
    pb.line_to(x + width, y);
    
    pb.line_to(x + width, y + height);
    
    pb.line_to(x + radius, y + height);
    pb.quad_to(x, y + height, x, y + height - radius);
    
    pb.line_to(x, y + radius);
    pb.quad_to(x, y, x + radius, y);
    
    pb.close();
    pb.finish()
}

pub fn draw_overlay(
    surface: &mut Surface<Arc<Window>, Arc<Window>>,
    width: u32,
    height: u32,
    headset_name: &str,
    battery_level: isize,
    is_charging: bool,
) {

    let mut pixmap = Pixmap::new(width, height).expect("Failed to create pixmap");

    let corner_radius = 50.0;
    
    let mut bg_paint = Paint::default();
    bg_paint.set_color_rgba8(0, 0, 0, 204);
    bg_paint.anti_alias = true;
    
    if let Some(bg_path) = create_rounded_rect(0.0, 0.0, width as f32, height as f32, corner_radius) {
        pixmap.fill_path(&bg_path, &bg_paint, FillRule::Winding, Transform::identity(), None);
    }

    let badge_d = (height as f32 - 20.0).min(80.0);
    let badge_x = 15.0;
    let badge_y = (height as f32 - badge_d) / 2.0;
    let badge_r = badge_d / 2.0;

    let mut badge_fill = Paint::default();
    badge_fill.set_color_rgba8(25, 25, 25, 255);
    badge_fill.anti_alias = true;
    if let Some(circle) = tiny_skia::PathBuilder::from_circle(badge_x + badge_r, badge_y + badge_r, badge_r) {
        pixmap.fill_path(&circle, &badge_fill, FillRule::Winding, Transform::identity(), None);
        let mut ring_paint = Paint::default();
        ring_paint.set_color_rgba8(255, 255, 255, 255);
        ring_paint.anti_alias = true;
        pixmap.stroke_path(
            &circle,
            &ring_paint,
            &tiny_skia::Stroke {
                width: 4.0,
                ..Default::default()
            },
            Transform::identity(),
            None,
        );
    }

    let icon_size = badge_d * 0.52;
    let icon_x = badge_x + (badge_d - icon_size) / 2.0;
    let icon_y = badge_y + (badge_d - icon_size) / 2.0;
    draw_battery_icon(&mut pixmap, icon_x, icon_y, icon_size, battery_level, is_charging);

    let text_x = badge_x + badge_d + (-33.0);
    let name_y = badge_y + 10.0;
    let percent_y = badge_y + 32.0;

    let name = if headset_name.len() > 28 {
        let mut s = headset_name.chars().take(28).collect::<String>();
        s.push('…');
        s
    } else {
        headset_name.to_string()
    };

    draw_text(&mut pixmap, &name, text_x, name_y, 22.0, (255, 255, 255, 255));
    draw_text(
        &mut pixmap,
        &format!("{}%", battery_level.max(0).min(100)),
        text_x,
        percent_y,
        26.0,
        (255, 255, 255, 255),
    );

    // Copy pixmap to surface buffer
    let mut buffer = surface.buffer_mut().expect("Failed to get buffer");
    for (i, pixel) in pixmap.pixels().iter().enumerate() {
        let r = pixel.red() as u32;
        let g = pixel.green() as u32;
        let b = pixel.blue() as u32;
        let a = pixel.alpha() as u32;
        buffer[i] = (a << 24) | (r << 16) | (g << 8) | b;
    }
    buffer.present().expect("Failed to present buffer");
}
