mod headset_control;
mod lang;
mod menu;
mod overlay;
mod settings;

use lang::Key::*;
use std::num::NonZeroU32;
use std::sync::Arc;
use std::time::{Duration, Instant};

use anyhow::Context;
use log::{error, info};
use softbuffer::Surface;
use tray_icon::{TrayIcon, TrayIconBuilder, menu::MenuEvent};
use winit::{
    application::ApplicationHandler,
    dpi::{LogicalSize, PhysicalPosition, PhysicalSize},
    event::{StartCause, WindowEvent},
    event_loop::{ActiveEventLoop, ControlFlow, EventLoop},
    platform::windows::WindowAttributesExtWindows,
    window::{Theme, Window, WindowAttributes, WindowLevel},
};

use crate::headset_control::BatteryState;

struct AppState {
    tray_icon: TrayIcon,
    devices: Vec<headset_control::Device>,
    context_menu: menu::ContextMenu,

    last_update: Instant,
    should_update_icon: bool,

    overlay_window: Option<Arc<Window>>,
    overlay_surface: Option<Surface<Arc<Window>, Arc<Window>>>,
    overlay_animation: Option<overlay::AnimationTimer>,
    overlay_focused: bool,
    overlay_enabled: bool,
    show_startup_overlay: bool,
    current_battery_level: isize,
    current_device_name: String,
    last_warning_battery_level: Option<isize>,
    current_charging_state: bool,
    battery_full_notified: bool,
}

pub const VERSION: &str = env!("CARGO_PKG_VERSION");

pub fn run() -> anyhow::Result<()> {
    info!("Starting application");
    info!("Version {VERSION}");

    let event_loop = EventLoop::new().context("Error initializing event loop")?;

    let mut app = AppState::init()?;

    Ok(event_loop.run_app(&mut app)?)
}

impl AppState {
    pub fn init() -> anyhow::Result<Self> {
        let settings = settings::load();

        let icon = Self::load_icon(Theme::Dark, 0, BatteryState::BatteryUnavailable)
            .context("loading fallback disconnected icon")?;

        let context_menu =
            menu::ContextMenu::new(settings.overlay_enabled).context("creating context menu")?;

        let tray_icon = TrayIconBuilder::new()
            .with_icon(icon)
            .with_menu(Box::new(context_menu.menu.clone()))
            .build()
            .context("Failed to create tray icon")?;

        Ok(Self {
            tray_icon,
            context_menu,

            devices: vec![],
            last_update: Instant::now(),
            should_update_icon: true,

            overlay_window: None,
            overlay_surface: None,
            overlay_animation: None,
            overlay_focused: false,
            overlay_enabled: settings.overlay_enabled,
            show_startup_overlay: settings.overlay_enabled,
            current_battery_level: 100,
            current_device_name: String::new(),
            last_warning_battery_level: None,
            current_charging_state: false,
            battery_full_notified: false,
        })
    }

    fn create_overlay(&mut self, event_loop: &ActiveEventLoop) -> anyhow::Result<()> {
        if self.overlay_window.is_some() {
            return Ok(());
        }

        let monitor = event_loop
            .primary_monitor()
            .or_else(|| event_loop.available_monitors().next())
            .context("No monitor found")?;
        
        let screen_size = monitor.size();
        let window_width = 380u32;
        let window_height = 90u32;
        let top_margin = 20i32;
        
        let start_x = screen_size.width as i32;
        let y_pos = top_margin;

        let window_attributes = WindowAttributes::default()
            .with_title("Low Battery Warning")
            .with_inner_size(LogicalSize::new(window_width, window_height))
            .with_resizable(false)
            .with_decorations(false)
            .with_transparent(true)
            .with_window_level(WindowLevel::AlwaysOnTop)
            .with_skip_taskbar(true);

        let window = Arc::new(event_loop.create_window(window_attributes)?);
        
        window.set_outer_position(PhysicalPosition::new(start_x, y_pos));

        let context = softbuffer::Context::new(window.clone())
            .map_err(|e| anyhow::anyhow!("Failed to create softbuffer context: {e}"))?;
        let mut surface = Surface::new(&context, window.clone())
            .map_err(|e| anyhow::anyhow!("Failed to create softbuffer surface: {e}"))?;

        let PhysicalSize { width, height } = window.inner_size();
        surface.resize(
            NonZeroU32::new(width).unwrap_or(NonZeroU32::new(1).unwrap()),
            NonZeroU32::new(height).unwrap_or(NonZeroU32::new(1).unwrap()),
        ).map_err(|e| anyhow::anyhow!("Failed to resize surface: {e}"))?;

        let mut animation = overlay::AnimationTimer::new();
        animation.start_slide_in();

        self.overlay_window = Some(window);
        self.overlay_surface = Some(surface);
        self.overlay_animation = Some(animation);

        if let Some(win) = &self.overlay_window {
            win.request_redraw();
        }

        Ok(())
    }

    fn destroy_overlay(&mut self) {
        self.overlay_surface = None;
        self.overlay_window = None;
        self.overlay_animation = None;
        self.overlay_focused = false;
    }

    fn update(&mut self, event_loop: &ActiveEventLoop) -> anyhow::Result<()> {
        let old_device_count = self.devices.len();
        headset_control::query_devices(&mut self.devices)?;

        if self.devices.len() != old_device_count {
            self.context_menu
                .update_device_menu(&self.devices)
                .context("Updating context menu")?;
        }

        if self.devices.is_empty() {
            self.tray_icon
                .set_tooltip(Some(lang::t(no_adapter_found)))?;
            self.current_battery_level = 100;
            self.current_device_name.clear();
            self.destroy_overlay();
            return Ok(());
        }

        let device = &self.devices[self
            .context_menu
            .selected_device_idx
            .min(self.devices.len() - 1)];

        #[allow(unused_mut)]
        let mut tooltip_text = device.to_string();

        #[cfg(debug_assertions)]
        {
            tooltip_text += " (Debug)";
        }

        self.tray_icon
            .set_tooltip(Some(&tooltip_text))
            .with_context(|| format!("setting tooltip text: {tooltip_text}"))?;

        let battery_percent = device.battery.level;
        self.current_battery_level = battery_percent;
        self.current_device_name = device.product.clone();
        
        let is_charging = device.battery.status == BatteryState::BatteryCharging;
        let charging_started = is_charging && !self.current_charging_state;
        self.current_charging_state = is_charging;

        match Self::load_icon(
            event_loop.system_theme().unwrap_or(Theme::Dark),
            battery_percent,
            device.battery.status,
        ) {
            Ok(icon) => self.tray_icon.set_icon(Some(icon))?,
            Err(err) => error!("Failed to load icon: {err:?}"),
        }

        let low_battery_10 = !is_charging && battery_percent <= 10 && self.last_warning_battery_level.is_none();
        let low_battery_3 = !is_charging
            && battery_percent <= 3
            && self.last_warning_battery_level.map_or(false, |last| last > 3);

        let full_battery = is_charging && battery_percent == 100 && !self.battery_full_notified;

        let reason = if self.show_startup_overlay {
            Some("Startup")
        } else if charging_started {
            Some("Charging started")
        } else if low_battery_10 {
            Some("Low battery (10%)")
        } else if low_battery_3 {
            Some("Critical battery (3%)")
        } else if full_battery {
            Some("Battery full")
        } else {
            None
        };

        if let Some(r) = reason {
            if self.overlay_enabled {
                info!("Showing overlay: {} ({}%)", r, battery_percent);
                if self.overlay_window.is_none() {
                    if let Err(e) = self.create_overlay(event_loop) {
                        error!("Failed to create overlay: {e:?}");
                    }
                }
            } else {
                info!("Overlay suppressed (disabled): {} ({}%)", r, battery_percent);
            }

            self.show_startup_overlay = false;
            if low_battery_10 || low_battery_3 {
                self.last_warning_battery_level = Some(battery_percent);
            }
            if full_battery {
                self.battery_full_notified = true;
            }
        } else {
            if !self.overlay_enabled {
                self.destroy_overlay();
            }
            
            if battery_percent > 10 || is_charging {
                self.last_warning_battery_level = None;
            }
            if battery_percent < 100 || !is_charging {
                self.battery_full_notified = false;
            }
        }

        self.should_update_icon = false;

        Ok(())
    }

    fn load_icon(
        theme: winit::window::Theme,
        battery_percent: isize,
        state: BatteryState,
    ) -> anyhow::Result<tray_icon::Icon> {
        let level = match battery_percent {
            -1 => 1,
            0..=12 => 1,
            13..=37 => 2,
            38..=62 => 3,
            63..=87 => 4,
            _ => 5,
        };

        let theme_offset: u16 = if theme == Theme::Light { 5 } else { 0 };
        let charging_offset = (state == BatteryState::BatteryCharging) as u16;

        let res_id = if state == BatteryState::BatteryUnavailable {
            10 + theme_offset
        } else {
            level * 10 + theme_offset + charging_offset
        };

        tray_icon::Icon::from_resource(res_id, None)
            .with_context(|| format!("loading icon from resource {res_id}"))
    }
}

impl ApplicationHandler<()> for AppState {
    fn resumed(&mut self, event_loop: &winit::event_loop::ActiveEventLoop) {
        event_loop.set_control_flow(ControlFlow::WaitUntil(
            Instant::now() + Duration::from_secs(1),
        ));
    }
    fn new_events(&mut self, event_loop: &ActiveEventLoop, cause: winit::event::StartCause) {
        if let StartCause::ResumeTimeReached { .. } = cause {
            event_loop.set_control_flow(ControlFlow::WaitUntil(
                Instant::now() + Duration::from_secs(1),
            ));
        }
    }

    fn about_to_wait(&mut self, event_loop: &ActiveEventLoop) {
        if self.last_update.elapsed() > Duration::from_millis(1000) {
            if let Err(e) = self.update(event_loop) {
                error!("Failed to update status: {e:?}");
            };
            self.last_update = Instant::now();
        }
        if let Ok(event) = MenuEvent::receiver().try_recv() {
            if self.context_menu.is_overlay_toggle(&event.id) {
                self.overlay_enabled = !self.overlay_enabled;
                self.context_menu.set_overlay_enabled(self.overlay_enabled);

                let _ = settings::save(&settings::Settings {
                    overlay_enabled: self.overlay_enabled,
                });

                if !self.overlay_enabled {
                    self.destroy_overlay();
                    self.show_startup_overlay = false;
                }
            } else {
                self.context_menu.handle_event(event, event_loop);
            }
        }
    }
    fn window_event(
        &mut self,
        event_loop: &ActiveEventLoop,
        window_id: winit::window::WindowId,
        event: WindowEvent,
    ) {
        if let Some(ref overlay_win) = self.overlay_window {
            if overlay_win.id() == window_id {
                match event {
                    WindowEvent::Focused(focused) => {
                        self.overlay_focused = focused;
                        if focused {
                            if let Some(animation) = &mut self.overlay_animation {
                                if animation.state == overlay::AnimationState::Visible {
                                    animation.visible_start = Some(Instant::now());
                                }
                            }
                        }
                    }
                    WindowEvent::RedrawRequested => {
                        if let (Some(surface), Some(animation)) = 
                            (&mut self.overlay_surface, &mut self.overlay_animation) {
                            
                            let should_continue = animation.update(self.overlay_focused);
                            
                            if !should_continue {
                                self.destroy_overlay();
                                return;
                            }
                            
                            if animation.state == overlay::AnimationState::Hidden {
                                self.destroy_overlay();
                                return;
                            }
                            
                            if let Some(monitor) = event_loop
                                .primary_monitor()
                                .or_else(|| event_loop.available_monitors().next())
                            {
                                let screen_size = monitor.size();
                                let monitor_pos = monitor.position();
                                let window_width = 380i32;
                                let top_margin = 20i32;

                                let base_x = monitor_pos.x;
                                let base_y = monitor_pos.y;

                                let start_x = base_x + screen_size.width as i32;
                                let target_x = base_x + screen_size.width as i32 - window_width;
                                
                                let current_x = match animation.state {
                                    overlay::AnimationState::SlidingIn => {
                                        let progress = overlay::ease_out_quad(animation.get_progress());
                                        start_x + ((target_x - start_x) as f32 * progress) as i32
                                    }
                                    overlay::AnimationState::Visible => target_x,
                                    overlay::AnimationState::SlidingOut => {
                                        let progress = overlay::ease_out_quad(animation.get_progress());
                                        let pos = target_x + ((start_x - target_x) as f32 * progress) as i32;
                                        pos.max(target_x).min(start_x)
                                    }
                                    overlay::AnimationState::Hidden => start_x,
                                };
                                
                                overlay_win.set_outer_position(PhysicalPosition::new(
                                    current_x,
                                    base_y + top_margin,
                                ));
                            }
                            
                            let size = overlay_win.inner_size();
                            if size.width == 0 || size.height == 0 {
                                return;
                            }
                            overlay::draw_overlay(
                                surface,
                                size.width,
                                size.height,
                                &self.current_device_name,
                                self.current_battery_level,
                                self.current_charging_state,
                            );
                            
                            overlay_win.request_redraw();
                        }
                    }
                    WindowEvent::CloseRequested => {
                        // User closed overlay
                        self.destroy_overlay();
                    }
                    _ => {}
                }
            }
        }
    }

    fn exiting(&mut self, _event_loop: &ActiveEventLoop) {
        info!("Exiting application..");
    }
}

#[test]
fn load_all_icons() {
    for i in 0..=100 {
        let _ = AppState::load_icon(Theme::Dark, i, BatteryState::BatteryAvailable);
    }
    for i in 0..=100 {
        let _ = AppState::load_icon(Theme::Light, i, BatteryState::BatteryAvailable);
    }
}
