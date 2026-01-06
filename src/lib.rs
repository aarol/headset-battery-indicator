mod headset_control;
mod lang;
mod menu;
mod settings;

use lang::Key::*;
use std::path::PathBuf;
use std::time::{Duration, Instant};

use anyhow::Context;
use log::{error, info};
use tray_icon::{TrayIcon, TrayIconBuilder, menu::MenuEvent};
use win32_notif::{
    NotificationBuilder,
    notification::visual::{
        Text,
        image::{Image, ImageCrop, Placement},
        text::HintStyle,
    },
    notifier::ToastsNotifier,
};
use winit::{
    application::ApplicationHandler,
    event::{StartCause, WindowEvent},
    event_loop::{ActiveEventLoop, ControlFlow, EventLoop},
    window::Theme,
};

#[cfg(windows)]
use windows::{
    Win32::Foundation::HMODULE,
    Win32::Storage::EnhancedStorage::PKEY_AppUserModel_ID,
    Win32::System::Com::{
        CLSCTX_INPROC_SERVER, COINIT_APARTMENTTHREADED, CoCreateInstance, CoInitializeEx,
        IPersistFile,
    },
    Win32::System::LibraryLoader::{GetProcAddress, LoadLibraryA},
    Win32::UI::Shell::PropertiesSystem::IPropertyStore,
    Win32::UI::Shell::{IShellLinkW, SetCurrentProcessExplicitAppUserModelID, ShellLink},
    core::{HSTRING, PCSTR, PCWSTR},
};

#[cfg(windows)]
use windows::core::{Interface, PROPVARIANT};

use crate::headset_control::BatteryState;

fn battery_res_id_for(theme: Theme, battery_percent: isize, state: BatteryState) -> u16 {
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
    let charging_offset = (state == BatteryState::BatteryCharging) as u16;

    if state == BatteryState::BatteryUnavailable {
        10 + theme_offset
    } else {
        level * 10 + theme_offset + charging_offset
    }
}

fn embedded_notif_png(
    battery_percent: isize,
    charging: bool,
) -> Option<(&'static [u8], &'static str)> {
    // Notification icon set in src/icons/notifs:
    // batt-5/10/25/50/75/full, with optional -charg.
    let bucket = match battery_percent {
        0..=7 => "5",
        8..=17 => "10",
        18..=37 => "25",
        38..=62 => "50",
        63..=87 => "75",
        _ => "full",
    };

    let key = match (bucket, charging) {
        ("5", false) => (
            include_bytes!("icons/notifs/batt-5.png").as_slice(),
            "batt-5.png",
        ),
        ("5", true) => (
            include_bytes!("icons/notifs/batt-5-charg.png").as_slice(),
            "batt-5-charg.png",
        ),
        ("10", false) => (
            include_bytes!("icons/notifs/batt-10.png").as_slice(),
            "batt-10.png",
        ),
        ("10", true) => (
            include_bytes!("icons/notifs/batt-10-charg.png").as_slice(),
            "batt-10-charg.png",
        ),
        ("25", false) => (
            include_bytes!("icons/notifs/batt-25.png").as_slice(),
            "batt-25.png",
        ),
        ("25", true) => (
            include_bytes!("icons/notifs/batt-25-charg.png").as_slice(),
            "batt-25-charg.png",
        ),
        ("50", false) => (
            include_bytes!("icons/notifs/batt-50.png").as_slice(),
            "batt-50.png",
        ),
        ("50", true) => (
            include_bytes!("icons/notifs/batt-50-charg.png").as_slice(),
            "batt-50-charg.png",
        ),
        ("75", false) => (
            include_bytes!("icons/notifs/batt-75.png").as_slice(),
            "batt-75.png",
        ),
        ("75", true) => (
            include_bytes!("icons/notifs/batt-75-charg.png").as_slice(),
            "batt-75-charg.png",
        ),
        ("full", false) => (
            include_bytes!("icons/notifs/batt-full.png").as_slice(),
            "batt-full.png",
        ),
        ("full", true) => (
            include_bytes!("icons/notifs/batt-full-charg.png").as_slice(),
            "batt-full-charg.png",
        ),
        _ => return None,
    };

    Some(key)
}

fn toast_cache_dir() -> Option<std::path::PathBuf> {
    // Use LocalAppData instead of Roaming config dir.
    // Toast image loading is more reliable from LocalAppData for unpackaged apps.
    let mut dir = dirs::data_local_dir().unwrap_or_else(|| std::env::temp_dir());
    dir.push("headset-battery-indicator");
    dir.push("toast-icons");
    std::fs::create_dir_all(&dir).ok()?;
    Some(dir)
}

fn path_to_file_uri(path: &std::path::Path) -> Option<String> {
    // Avoid `canonicalize()` here: on Windows it often returns a `\\?\C:\...` path
    // which produces a broken `file://///?/C:/...` URI and the toast image silently fails.
    let mut s = path.to_string_lossy().to_string();
    if let Some(stripped) = s.strip_prefix(r"\\?\") {
        s = stripped.to_string();
    }
    s = s.replace('\\', "/");

    // Drive-letter absolute path
    if s.len() >= 2 && s.as_bytes().get(1) == Some(&b':') {
        return Some(format!("file:///{s}"));
    }

    // Fallback: treat as already-rooted
    if s.starts_with('/') {
        return Some(format!("file://{s}"));
    }

    None
}

fn toast_notif_logo_uri(battery_percent: isize, state: BatteryState) -> Option<String> {
    let charging = state == BatteryState::BatteryCharging;
    let (png_bytes, filename) = embedded_notif_png(battery_percent, charging)?;

    let dir = toast_cache_dir()?;

    // App logo override must be square; generate a square version of the wide 113x51 PNG.
    let logo_name = format!("logo-{filename}");
    let logo_path = dir.join(logo_name);
    if !logo_path.exists() {
        let decoded = match image::load_from_memory_with_format(png_bytes, image::ImageFormat::Png)
        {
            Ok(img) => img.to_rgba8(),
            Err(e) => {
                error!("Failed to decode notif png for toast logo: {e:?}");
                return None;
            }
        };

        let (w, h) = decoded.dimensions();
        let side = w.max(h);
        let mut square = image::RgbaImage::from_pixel(side, side, image::Rgba([0, 0, 0, 0]));
        let x = (side - w) / 2;
        let y = (side - h) / 2;

        for yy in 0..h {
            for xx in 0..w {
                let p = *decoded.get_pixel(xx, yy);
                square.put_pixel(x + xx, y + yy, p);
            }
        }

        if let Err(e) = image::DynamicImage::ImageRgba8(square)
            .save_with_format(&logo_path, image::ImageFormat::Png)
        {
            error!("Failed to write toast logo png to {:?}: {e:?}", logo_path);
            return None;
        }
    }

    path_to_file_uri(&logo_path)
}

struct AppState {
    tray_icon: TrayIcon,
    devices: Vec<headset_control::Device>,
    context_menu: menu::ContextMenu,
    settings: settings::Settings,
    last_notification_state: Option<(isize, BatteryState)>,
    notifier: ToastsNotifier,

    last_update: Instant,
    should_update_icon: bool,
}

pub const VERSION: &str = env!("CARGO_PKG_VERSION");

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
fn enable_dark_mode() {
    unsafe {
        // Load uxtheme.dll
        let dll_name = b"uxtheme.dll\0";
        let module: HMODULE = match LoadLibraryA(PCSTR::from_raw(dll_name.as_ptr())) {
            Ok(m) => m,
            Err(_) => {
                log::warn!("Failed to load uxtheme.dll");
                return;
            }
        };

        // SetPreferredAppMode is ordinal 135 in uxtheme.dll
        let ordinal = 135u16;
        let proc = GetProcAddress(module, PCSTR::from_raw(ordinal as *const u8));

        if let Some(proc) = proc {
            let set_preferred_app_mode: SetPreferredAppModeFn = std::mem::transmute(proc);
            set_preferred_app_mode(PreferredAppMode::AllowDark);
        } else {
            log::warn!("Failed to get SetPreferredAppMode function");
        }
    }
}

#[cfg(windows)]
fn to_wide_null(s: &std::ffi::OsStr) -> Vec<u16> {
    use std::os::windows::ffi::OsStrExt;
    s.encode_wide().chain(std::iter::once(0)).collect()
}

#[cfg(windows)]
fn ensure_toast_shortcut(app_id: &str) -> anyhow::Result<()> {
    // Win32 Toast notifications typically require a Start Menu shortcut whose
    // AppUserModelID matches the notifier ID. Without this, `show()` can succeed
    // but nothing appears.
    let exe_path = std::env::current_exe().context("getting current exe path")?;

    let appdata = std::env::var_os("APPDATA").context("APPDATA env var not set")?;
    let mut shortcut_path = PathBuf::from(appdata);
    shortcut_path.push("Microsoft\\Windows\\Start Menu\\Programs");
    shortcut_path.push("Headset Battery Indicator.lnk");

    if shortcut_path.exists() {
        return Ok(());
    }

    if let Some(parent) = shortcut_path.parent() {
        std::fs::create_dir_all(parent).context("creating Start Menu Programs directory")?;
    }

    unsafe {
        // Initialize COM (ignore mode mismatch if already initialized differently).
        let _ = CoInitializeEx(None, COINIT_APARTMENTTHREADED);

        let shell_link: IShellLinkW = CoCreateInstance(&ShellLink, None, CLSCTX_INPROC_SERVER)
            .context("CoCreateInstance(ShellLink)")?;

        let exe_w = to_wide_null(exe_path.as_os_str());
        shell_link
            .SetPath(PCWSTR::from_raw(exe_w.as_ptr()))
            .context("IShellLinkW::SetPath")?;

        // Set a stable AppUserModelID on the shortcut.
        let property_store: IPropertyStore = shell_link
            .cast()
            .context("QueryInterface(IPropertyStore)")?;

        let pv: PROPVARIANT = PROPVARIANT::from(app_id);
        property_store
            .SetValue(&PKEY_AppUserModel_ID, &pv)
            .context("IPropertyStore::SetValue(PKEY_AppUserModel_ID)")?;
        property_store.Commit().context("IPropertyStore::Commit")?;

        let persist_file: IPersistFile =
            shell_link.cast().context("QueryInterface(IPersistFile)")?;
        let shortcut_w = to_wide_null(shortcut_path.as_os_str());
        persist_file
            .Save(PCWSTR::from_raw(shortcut_w.as_ptr()), true)
            .context("IPersistFile::Save")?;
    }

    Ok(())
}

#[cfg(windows)]
fn register_toast_app(app_id: &str) -> anyhow::Result<()> {
    // Ensure the system associates this running EXE with the same AUMID.
    unsafe {
        SetCurrentProcessExplicitAppUserModelID(&HSTRING::from(app_id))
            .context("SetCurrentProcessExplicitAppUserModelID")?;
    }

    ensure_toast_shortcut(app_id)
}

pub fn run() -> anyhow::Result<()> {
    info!("Starting application");
    info!("Version {VERSION}");

    #[cfg(windows)]
    enable_dark_mode();

    let event_loop = EventLoop::new().context("Error initializing event loop")?;

    let mut app = AppState::init()?;

    Ok(event_loop.run_app(&mut app)?)
}

impl AppState {
    pub fn init() -> anyhow::Result<Self> {
        let settings = settings::Settings::load();

        let icon = Self::load_icon(Theme::Dark, 0, BatteryState::BatteryUnavailable)
            .context("loading fallback disconnected icon")?;

        let context_menu = menu::ContextMenu::new(settings.notifications_enabled)
            .context("creating context menu")?;

        let tray_icon = TrayIconBuilder::new()
            .with_icon(icon)
            .with_menu(Box::new(context_menu.menu.clone()))
            .build()
            .context("Failed to create tray icon")?;

        const TOAST_APP_ID: &str = "HeadsetBatteryIndicator.App";

        let notifier = {
            #[cfg(windows)]
            {
                if let Err(e) = register_toast_app(TOAST_APP_ID) {
                    error!("Toast registration failed; falling back to Explorer AUMID: {e:?}");
                    ToastsNotifier::new("Microsoft.Windows.Explorer").unwrap()
                } else {
                    ToastsNotifier::new(TOAST_APP_ID).unwrap_or_else(|e| {
                        error!(
                            "Failed to create notifier for app id; falling back to Explorer: {e:?}"
                        );
                        ToastsNotifier::new("Microsoft.Windows.Explorer").unwrap()
                    })
                }
            }
            #[cfg(not(windows))]
            {
                ToastsNotifier::new("Microsoft.Windows.Explorer").unwrap()
            }
        };

        Ok(Self {
            tray_icon,
            context_menu,
            settings,
            last_notification_state: None,
            notifier,

            devices: vec![],
            last_update: Instant::now(),
            should_update_icon: true,
        })
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
            return Ok(());
        }

        let device_idx = self
            .context_menu
            .selected_device_idx
            .min(self.devices.len() - 1);

        let battery_level;
        let battery_status;
        let product_name;
        let tooltip_text;

        {
            let device = &self.devices[device_idx];
            battery_level = device.battery.level;
            battery_status = device.battery.status;
            product_name = device.product.clone();

            #[allow(unused_mut)]
            let mut text = device.to_string();

            #[cfg(debug_assertions)]
            {
                text += " (Debug)";
            }

            tooltip_text = text;
        }

        self.check_notifications(battery_level, battery_status, &product_name);

        self.tray_icon
            .set_tooltip(Some(&tooltip_text))
            .with_context(|| format!("setting tooltip text: {tooltip_text}"))?;

        match Self::load_icon(
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

    fn check_notifications(
        &mut self,
        current_level: isize,
        current_status: BatteryState,
        product_name: &str,
    ) {
        if !self.settings.notifications_enabled {
            self.last_notification_state = Some((current_level, current_status));
            return;
        }

        if let Some((last_level, last_status)) = self.last_notification_state {
            let mut msg = None;

            // Low battery (10%)
            if current_level <= 10
                && last_level > 10
                && current_status != BatteryState::BatteryCharging
                && current_status != BatteryState::BatteryUnavailable
            {
                msg = Some(format!("Battery low ({}%)", current_level));
            }
            // Critical battery (3%)
            else if current_level <= 3
                && last_level > 3
                && current_status != BatteryState::BatteryCharging
                && current_status != BatteryState::BatteryUnavailable
            {
                msg = Some(format!("Battery critical ({}%)", current_level));
            }
            // Charging started
            else if current_status == BatteryState::BatteryCharging
                && last_status != BatteryState::BatteryCharging
            {
                msg = Some(format!("Charging started [{}%]", current_level));
            }
            // Battery full (100%)
            else if current_level == 100
                && last_level < 100
                && current_status == BatteryState::BatteryCharging
            {
                msg = Some("Battery full".to_string());
            }

            if let Some(body) = msg {
                let mut builder = NotificationBuilder::new()
                    .visual(Text::create(0, product_name).with_style(HintStyle::Title))
                    .visual(Text::create(1, &body).with_style(HintStyle::Body));

                if let Some(logo_uri) = toast_notif_logo_uri(current_level, current_status) {
                    builder = builder.visual(
                        Image::create(2, &logo_uri)
                            .with_placement(Placement::AppLogoOverride)
                            .with_crop(ImageCrop::None),
                    );
                }

                match builder.build(
                    current_level as u32,
                    &self.notifier,
                    &format!("battery_{}", current_level),
                    "battery",
                ) {
                    Ok(notif) => {
                        if let Err(e) = notif.show() {
                            error!("Failed to show notification: {e:?}");
                        }
                    }
                    Err(e) => {
                        error!("Failed to build notification: {e:?}");
                    }
                }
            }
        }

        self.last_notification_state = Some((current_level, current_status));
    }

    fn load_icon(
        theme: winit::window::Theme,
        battery_percent: isize,
        state: BatteryState,
    ) -> anyhow::Result<tray_icon::Icon> {
        let res_id = battery_res_id_for(theme, battery_percent, state);

        tray_icon::Icon::from_resource(res_id, None)
            .with_context(|| format!("loading icon from resource {res_id}"))
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
        // This will be called at least every second
        if self.last_update.elapsed() > Duration::from_millis(1000) {
            if let Err(e) = self.update(event_loop) {
                error!("Failed to update status: {e:?}");
            };
            self.last_update = Instant::now();
        }
        if let Ok(event) = MenuEvent::receiver().try_recv() {
            if event.id == self.context_menu.menu_notifications.id() {
                self.settings.notifications_enabled = !self.settings.notifications_enabled;
                self.context_menu
                    .menu_notifications
                    .set_checked(self.settings.notifications_enabled);
                self.settings.save();
            } else {
                self.context_menu.handle_event(event, event_loop);
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

#[test]
fn load_all_icons() {
    for i in 0..=100 {
        let _ = AppState::load_icon(Theme::Dark, i, BatteryState::BatteryAvailable);
    }
    for i in 0..=100 {
        let _ = AppState::load_icon(Theme::Light, i, BatteryState::BatteryAvailable);
    }
}
