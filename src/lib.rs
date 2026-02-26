mod headset_control;
mod icon;
mod lang;
mod menu;
mod notify;
mod settings;
mod version_check;

#[cfg(windows)]
use anyhow::Result;
use lang::Key::*;
use std::time::{Duration, Instant};

use anyhow::Context;
use log::{debug, error, info, warn};
use tray_icon::{TrayIcon, TrayIconBuilder, menu::MenuEvent};
use winit::{
    application::ApplicationHandler,
    event::{StartCause, WindowEvent},
    event_loop::{ActiveEventLoop, ControlFlow, EventLoop},
    window::Theme,
};

use crate::{headset_control::BatteryStatus, notify::Notifier, settings::Settings};
use std::sync::mpsc;

struct AppState {
    tray_icon: TrayIcon,
    context_menu: menu::ContextMenu,
    settings: settings::Settings,
    notifier: Notifier,

    last_update: Instant,
    should_update_icon: bool,
    update_receiver: Option<mpsc::Receiver<bool>>,
}

pub const VERSION: &str = env!("CARGO_PKG_VERSION");

pub fn run() -> anyhow::Result<()> {
    info!("Starting application");
    info!("Version {VERSION}");
    debug!("Using locale {:?}", *lang::LANG);

    if let Err(err) = enable_dark_mode_support() {
        warn!("Failed to enable dark mode support: {:?}", err);
    }

    let event_loop = EventLoop::new().context("Error initializing event loop")?;

    let mut app = AppState::init()?;

    Ok(event_loop.run_app(&mut app)?)
}

impl AppState {
    pub fn init() -> anyhow::Result<Self> {
        let settings = settings::Settings::load().context("loading config from registry")?;

        let icon = Self::load_icon(
            settings,
            Theme::Dark,
            0,
            BatteryStatus::Unavailable,
        ).context("loading fallback icon")?;

        let context_menu = menu::ContextMenu::new(settings).context("creating context menu")?;

        let tray_icon = TrayIconBuilder::new()
            .with_icon(icon)
            .with_menu(Box::new(context_menu.menu.clone()))
            .build()
            .context("Failed to create tray icon")?;

        let notifier = Notifier::new().context("initializing notifier")?;

        // Check for updates in the background (non-blocking)
        let update_receiver = version_check::check_for_updates_async(VERSION);

        Ok(Self {
            tray_icon,
            context_menu,
            settings,
            notifier,

            last_update: Instant::now(),
            should_update_icon: true,
            update_receiver: Some(update_receiver),
        })
    }

    fn update(&mut self, event_loop: &ActiveEventLoop) -> anyhow::Result<()> {
        let result = headset_control::query_device();

        match result {
            None => {
                self.notifier.update(0, BatteryStatus::Unavailable, "");

                self.tray_icon
                    .set_tooltip(Some(lang::t(no_adapter_found)))?;
                match Self::load_icon(
                    self.settings,
                    event_loop.system_theme().unwrap_or(Theme::Dark),
                    0,
                    BatteryStatus::Unavailable,
                ) {
                    Ok(icon) => self.tray_icon.set_icon(Some(icon))?,
                    Err(err) => error!("Failed to load icon: {err:?}"),
                }
                Ok(())
            }
            Some(device) => {
                let battery_level = device.battery.level_percent as isize;
                let battery_status = device.battery.status;
                let product_name = device.product_name.to_string();
                let tooltip_text = format!(
                    "{}{}",
                    device,
                    if cfg!(debug_assertions) {
                        " (Debug)"
                    } else {
                        ""
                    }
                );

                self.notifier
                    .update(battery_level, battery_status, &product_name);

                self.tray_icon
                    .set_tooltip(Some(&tooltip_text))
                    .with_context(|| format!("setting tooltip text: {tooltip_text}"))?;

                match Self::load_icon(
                    self.settings,
                    event_loop.system_theme().unwrap_or(Theme::Dark),
                    battery_level,
                    battery_status,
                ) {
                    Ok(icon) => self.tray_icon.set_icon(Some(icon))?,
                    Err(err) => error!("Failed to load icon: {err:?}"),
                }

                self.should_update_icon = false;

                Ok(())
            }
        }
    }

    fn load_icon(
        settings: Settings,
        theme: Theme,
        battery_percent: isize,
        state: BatteryStatus,
    ) -> anyhow::Result<tray_icon::Icon> {
        if settings.use_number_icon {
            icon::generate_number_icon(theme, battery_percent, state, settings.icon_color).context("generating number icon")
        } else {
            icon::load_from_resource(theme, battery_percent, state).context("loading icon from resource")
        }
    }
}

impl ApplicationHandler<()> for AppState {
    fn resumed(&mut self, event_loop: &winit::event_loop::ActiveEventLoop) {
        // Kick off polling every 1 second
        event_loop.set_control_flow(ControlFlow::WaitUntil(
            Instant::now() + Duration::from_secs(1),
        ));
    }
    fn new_events(&mut self, event_loop: &ActiveEventLoop, cause: winit::event::StartCause) {
        if let StartCause::ResumeTimeReached { .. } = cause {
            // Overwrite the current polling time
            //
            // If not overwritten, it starts polling multiple times a second
            // since the timer is already elapsed.
            event_loop.set_control_flow(ControlFlow::WaitUntil(
                Instant::now() + Duration::from_secs(1),
            ));
        }
    }

    fn about_to_wait(&mut self, event_loop: &ActiveEventLoop) {
        // Check if update check has completed (non-blocking)
        if let Some(receiver) = &self.update_receiver
            && let Ok(has_update) = receiver.try_recv()
        {
            self.update_receiver = None; // Stop checking

            if has_update && let Err(e) = self.context_menu.show_update_available() {
                error!("Failed to show update menu item: {e:?}");
            }
        }

        // This will be called at least every second
        if self.last_update.elapsed() > Duration::from_millis(1000) {
            if let Err(e) = self.update(event_loop) {
                error!("Failed to update status: {e:?}");
            };
            self.last_update = Instant::now();
        }
        if let Ok(event) = MenuEvent::receiver().try_recv() {
            match event.id {
                id if id == self.context_menu.menu_enable_notifications.id() => {
                    self.settings.notifications_enabled = !self.settings.notifications_enabled;
                    self.context_menu
                        .menu_enable_notifications
                        .set_checked(self.settings.notifications_enabled);
                    if let Err(e) = self.settings.save() {
                        error!("Failed to save settings: {e:?}");
                    }

                    if self.settings.notifications_enabled {
                        let msg = lang::t(notifications_enabled_message);
                        if let Err(err) = self
                            .notifier
                            .show_notification("Headset Battery Indicator", msg)
                        {
                            error!("Failed to show notification: {:?}", err);
                        }
                    }
                }

                id if id == self.context_menu.menu_show_text_icon.id() => {
                    self.settings.use_number_icon = !self.settings.use_number_icon;
                    self.context_menu
                        .menu_show_text_icon
                        .set_checked(self.settings.use_number_icon);
                    self.context_menu
                        .set_color_menu_enabled(self.settings.use_number_icon);

                    _ = self.update(event_loop);

                    if let Err(e) = self.settings.save() {
                        error!("Failed to save settings: {e:?}");
                    }
                }

                id if id == self.context_menu.menu_trigger_notification.id() => {
                    #[cfg(debug_assertions)]
                    {
                        self.notifier
                            .show_notification("Test Device", "Battery critical (50%)")
                            .expect("Sending test notification");
                    }
                }

                id if id == self.context_menu.menu_choose_color.id() => {
                    if let Some(new_color) = open_color_picker(self.settings.icon_color) {
                        self.settings.icon_color = new_color;
                        if let Err(e) = self.settings.save() {
                            error!("Failed to save settings: {e:?}");
                        }
                        _ = self.update(event_loop);
                    }
                }

                _ => self.context_menu.handle_event(event, event_loop),
            }
        }
    }
    fn window_event(
        &mut self,
        _event_loop: &ActiveEventLoop,
        _window_id: winit::window::WindowId,
        _event: WindowEvent,
    ) {
        // Since we don't have a window attached, this will never be called
    }

    fn exiting(&mut self, _event_loop: &ActiveEventLoop) {
        info!("Exiting application..");
    }
}

// Enable dark mode support on Windows 10/11

/// Opens the native Windows color picker dialog.
/// Returns:
/// - `Some(Some([r, g, b]))` if the user selected a color
/// - `Some(None)` if the user cancelled (resets to theme-adaptive)
/// - `None` if the dialog could not be opened
#[cfg(windows)]
fn open_color_picker(current_color: Option<[u8; 3]>) -> Option<Option<[u8; 3]>> {
    use windows::Win32::Foundation::{COLORREF, HWND};
    use windows::Win32::UI::Controls::Dialogs::{CC_FULLOPEN, CC_RGBINIT, CHOOSECOLORW, ChooseColorW};

    let initial_color: u32 = match current_color {
        Some([r, g, b]) => r as u32 | ((g as u32) << 8) | ((b as u32) << 16),
        None => 0x00FFFFFF, // Default to white
    };

    let mut custom_colors = [COLORREF(0u32); 16];
    let mut cc = CHOOSECOLORW {
        lStructSize: std::mem::size_of::<CHOOSECOLORW>() as u32,
        hwndOwner: HWND::default(),
        rgbResult: COLORREF(initial_color),
        lpCustColors: custom_colors.as_mut_ptr(),
        Flags: CC_RGBINIT | CC_FULLOPEN,
        ..Default::default()
    };

    let picked = unsafe { ChooseColorW(&mut cc).as_bool() };
    if picked {
        let v = cc.rgbResult.0;
        Some(Some([
            (v & 0xFF) as u8,
            ((v >> 8) & 0xFF) as u8,
            ((v >> 16) & 0xFF) as u8,
        ]))
    } else {
        Some(None) // Cancelled → reset to theme-adaptive
    }
}

#[cfg(windows)]
#[repr(C)]
#[allow(dead_code)]
enum PreferredAppMode {
    Default = 0,
    AllowDark = 1,
    ForceDark = 2,
    ForceLight = 3,
}

#[cfg(windows)]
type SetPreferredAppModeFn = unsafe extern "system" fn(PreferredAppMode) -> i32;

#[cfg(windows)]
fn enable_dark_mode_support() -> Result<()> {
    unsafe {
        // Load uxtheme.dll

        use windows::{
            Win32::{
                Foundation::HMODULE,
                System::LibraryLoader::{GetProcAddress, LoadLibraryA},
            },
            core::PCSTR,
        };
        let module: HMODULE =
            LoadLibraryA(windows::core::s!("uxtheme.dll")).context("loading uxtheme.dll")?;

        // SetPreferredAppMode is ordinal 135 in uxtheme.dll
        let ordinal = 135u16;
        let proc = GetProcAddress(module, PCSTR::from_raw(ordinal as *const u8))
            .context("Failed to get proc address")?;

        let set_preferred_app_mode: SetPreferredAppModeFn = std::mem::transmute(proc);
        set_preferred_app_mode(PreferredAppMode::AllowDark);

        Ok(())
    }
}
