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
#[cfg(windows)]
use std::os::raw::c_void;
use std::{sync::LazyLock, thread, time::Duration};
#[cfg(windows)]
use windows::Win32::System::Threading::{
    GetCurrentProcess, IDLE_PRIORITY_CLASS, PROCESS_POWER_THROTTLING_CURRENT_VERSION,
    PROCESS_POWER_THROTTLING_EXECUTION_SPEED, PROCESS_POWER_THROTTLING_STATE,
    ProcessPowerThrottling, SetPriorityClass, SetProcessInformation,
};
use windows::{
    Win32::System::LibraryLoader::{GetProcAddress, LoadLibraryA},
    core::PCSTR,
};

use anyhow::Context;
use log::{debug, error, info, warn};
use tray_icon::{TrayIcon, TrayIconBuilder, menu::MenuEvent};
use winit::{
    application::ApplicationHandler,
    event::WindowEvent,
    event_loop::{ActiveEventLoop, EventLoop},
    window::Theme,
};

use crate::{headset_control::BatteryStatus, icon::IconProvider, notify::Notifier};

struct AppState {
    tray_icon: TrayIcon,
    icon_provider: IconProvider,
    context_menu: menu::ContextMenu,
    settings: settings::Settings,
    notifier: Option<Notifier>,
    last_headset: Option<headset_control::Device>,
}

enum UserEvent {
    HeadsetUpdated(Option<headset_control::Device>),
    MenuEvent(MenuEvent),
    UpdateAvailable(bool),
}

pub const VERSION: &str = env!("CARGO_PKG_VERSION");

pub fn run() -> anyhow::Result<()> {
    info!("Starting application");
    info!("Version {VERSION}");
    debug!("Using locale {:?}", *lang::LANG);

    if let Err(err) = enable_menu_dark_mode_support() {
        warn!("Failed to enable dark mode support: {:?}", err);
    }

    if let Err(err) = set_efficiency_mode(true) {
        warn!("Failed to set efficiency mode: {:?}", err);
    }

    let event_loop = EventLoop::<UserEvent>::with_user_event()
        .build()
        .context("Error initializing event loop")?;

    let mut app = AppState::init()?;

    let menu_proxy = event_loop.create_proxy();
    MenuEvent::set_event_handler(Some(move |event| {
        let _ = menu_proxy.send_event(UserEvent::MenuEvent(event));
    }));

    let update_proxy = event_loop.create_proxy();
    std::thread::spawn(move || match version_check::check_for_updates(VERSION) {
        Ok(has_update) => {
            let _ = update_proxy.send_event(UserEvent::UpdateAvailable(has_update));
        }
        Err(err) => {
            error!("Failed to check for updates: {:?}", err);
        }
    });

    let headsetcontrol_proxy = event_loop.create_proxy();
    spawn_headset_poll_thread(headsetcontrol_proxy, Duration::from_millis(2000));

    Ok(event_loop.run_app(&mut app)?)
}

impl AppState {
    pub fn init() -> anyhow::Result<Self> {
        let settings = settings::Settings::load()
            .context("loading config from registry")
            .unwrap_or_default();

        let mut icon_provider = IconProvider::new();

        let icon = icon_provider
            .get_icon(Theme::Dark, 0, BatteryStatus::Unavailable, settings)
            .context("loading fallback icon")?;

        let context_menu = menu::ContextMenu::new(settings).context("creating context menu")?;

        let tray_icon = TrayIconBuilder::new()
            .with_icon(icon)
            .with_menu(Box::new(context_menu.menu.clone()))
            .build()
            .context("Failed to create tray icon")?;

        let notifier = match Notifier::new() {
            Ok(notifier) => Some(notifier),
            Err(err) => {
                error!("Failed to initialize notification system: {err:?}");
                None
            }
        };

        Ok(Self {
            tray_icon,
            icon_provider,
            context_menu,
            settings,
            notifier,
            last_headset: None,
        })
    }

    fn update(&mut self, headset: Option<headset_control::Device>) -> anyhow::Result<()> {
        if self.notifier.is_none() {
            debug!("Notifier not initialized, attempting to initialize..");
            self.notifier = match Notifier::new() {
                Ok(notifier) => Some(notifier),
                Err(err) => {
                    debug!("Failed to initialize notification system: {err:?}");
                    None
                }
            };
        }

        match &headset {
            None => {
                if let Some(notifier) = &mut self.notifier {
                    notifier.update(0, BatteryStatus::Unavailable, "");
                }
            }
            Some(device) => {
                if let Some(notifier) = &mut self.notifier {
                    notifier.update(
                        device.battery.level_percent as isize,
                        device.battery.status,
                        &device.product_name,
                    );
                }
            }
        };
        self.last_headset = headset;
        if let Err(err) = self.refresh_tray_icon() {
            error!("Failed to refresh tray icon: {err:?}");
        }

        Ok(())
    }

    fn refresh_tray_icon(&mut self) -> anyhow::Result<()> {
        match &self.last_headset {
            Some(headset) => {
                let battery_level = headset.battery.level_percent as isize;
                let battery_status = headset.battery.status;
                let tooltip_text = format!(
                    "{}{}",
                    headset,
                    if cfg!(debug_assertions) {
                        " (Debug)"
                    } else {
                        ""
                    }
                );

                self.tray_icon.set_tooltip(Some(tooltip_text))?;

                match self.icon_provider.get_icon(
                    system_theme(),
                    battery_level,
                    battery_status,
                    self.settings,
                ) {
                    Ok(icon) => self.tray_icon.set_icon(Some(icon))?,
                    Err(err) => error!("Failed to load icon: {err:?}"),
                }
            }
            None => {
                self.tray_icon
                    .set_tooltip(Some(lang::t(no_headset_found)))?;

                match self.icon_provider.get_icon(
                    system_theme(),
                    0,
                    BatteryStatus::Unavailable,
                    self.settings,
                ) {
                    Ok(icon) => self.tray_icon.set_icon(Some(icon))?,
                    Err(err) => error!("Failed to load icon: {err:?}"),
                }
            }
        }
        Ok(())
    }
}

impl ApplicationHandler<UserEvent> for AppState {
    fn resumed(&mut self, _: &winit::event_loop::ActiveEventLoop) {}

    fn window_event(
        &mut self,
        _event_loop: &ActiveEventLoop,
        _window_id: winit::window::WindowId,
        _event: WindowEvent,
    ) {
        // Since we don't have a window attached, this will never be called
    }

    fn user_event(&mut self, event_loop: &ActiveEventLoop, event: UserEvent) {
        match event {
            UserEvent::MenuEvent(event) => {
                let context_menu = &mut self.context_menu;
                match event.id {
                    id if id == context_menu.menu_enable_notifications.id() => {
                        self.settings.notifications_enabled = !self.settings.notifications_enabled;
                        context_menu
                            .menu_enable_notifications
                            .set_checked(self.settings.notifications_enabled);
                        if let Err(e) = self.settings.save() {
                            error!("Failed to save settings: {e:?}");
                        }

                        if self.settings.notifications_enabled {
                            let msg = lang::t(notifications_enabled_message);

                            if let Some(notifier) = &mut self.notifier
                                && let Err(err) =
                                    notifier.show_notification("Headset Battery Indicator", msg)
                            {
                                error!("Failed to show notification: {:?}", err);
                            }
                        }
                    }

                    id if [
                        context_menu.icon_image.id(),
                        context_menu.icon_text_gdi.id(),
                        context_menu.icon_text_bitmap.id(),
                    ]
                    .contains(&&id) =>
                    {
                        self.settings.icon_type = if id == *context_menu.icon_image.id() {
                            settings::IconType::Image
                        } else if id == *context_menu.icon_text_gdi.id() {
                            settings::IconType::TextGdi
                        } else {
                            settings::IconType::TextBitmap
                        };
                        context_menu.set_icon_type(self.settings.icon_type);
                        self.refresh_tray_icon().unwrap_or_else(|err| {
                            error!("Failed to refresh tray icon: {:?}", err);
                        });

                        if let Err(e) = self.settings.save() {
                            error!("Failed to save settings: {e:?}");
                        }
                    }

                    id if id == self.context_menu.menu_trigger_notification.id() => {
                        #[cfg(debug_assertions)]
                        {
                            if let Some(notifier) = &mut self.notifier {
                                notifier
                                    .show_notification("Test Device", "Battery low (50%)")
                                    .expect("Sending test notification");
                            }
                        }
                    }

                    _ => self.context_menu.handle_event(event, event_loop),
                }
            }
            UserEvent::HeadsetUpdated(headset) => {
                if let Err(err) = self.update(headset) {
                    error!("Failed to update headset state: {:?}", err);
                }
            }
            UserEvent::UpdateAvailable(has_update) => {
                if has_update && let Err(e) = self.context_menu.show_update_available() {
                    error!("Failed to show update menu item: {e:?}");
                }
            }
        }
    }

    fn exiting(&mut self, _event_loop: &ActiveEventLoop) {
        info!("Exiting application..");
    }
}

fn spawn_headset_poll_thread(
    proxy: winit::event_loop::EventLoopProxy<UserEvent>,
    interval: Duration,
) {
    std::thread::spawn(move || {
        loop {
            let device = headset_control::query_device();
            if proxy.send_event(UserEvent::HeadsetUpdated(device)).is_err() {
                break; // Event loop closed
            }
            thread::sleep(interval);
        }
    });
}

// Enable dark mode support on Windows 10/11

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
fn enable_menu_dark_mode_support() -> Result<()> {
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

fn system_theme() -> Theme {
    if should_system_use_dark_mode() {
        Theme::Dark
    } else {
        Theme::Light
    }
}

// Since the tray icon is on the task bar, it should follow
// the *system* level dark mode setting, not the application level setting.
fn should_system_use_dark_mode() -> bool {
    type ShouldSystemUseDarkMode = unsafe extern "system" fn() -> bool;

    static SHOULD_SYSTEM_USE_DARK_MODE: LazyLock<Option<ShouldSystemUseDarkMode>> =
        LazyLock::new(|| unsafe {
            const UXTHEME_SHOULDSYSTEMUSEDARKMODE_ORDINAL: PCSTR =
                PCSTR::from_raw(138 as *const u8);

            match LoadLibraryA(windows::core::s!("uxtheme.dll")) {
                Ok(module) => {
                    let handle = GetProcAddress(module, UXTHEME_SHOULDSYSTEMUSEDARKMODE_ORDINAL);
                    handle.map(|handle| std::mem::transmute(handle))
                }
                Err(_) => None,
            }
        });

    SHOULD_SYSTEM_USE_DARK_MODE
        .map(|shoud_system_use_dark_mode| unsafe { (shoud_system_use_dark_mode)() })
        .unwrap_or(true)
}

#[cfg(windows)]
pub fn set_efficiency_mode(enable: bool) -> windows::core::Result<()> {
    unsafe {
        let process = GetCurrentProcess();

        // EcoQoS: ControlMask says "I'm taking control of this flag",
        // StateMask says whether it's on (flag set) or off (0).
        let state = PROCESS_POWER_THROTTLING_STATE {
            Version: PROCESS_POWER_THROTTLING_CURRENT_VERSION,
            ControlMask: PROCESS_POWER_THROTTLING_EXECUTION_SPEED,
            StateMask: if enable {
                PROCESS_POWER_THROTTLING_EXECUTION_SPEED
            } else {
                0
            },
        };

        SetProcessInformation(
            process,
            ProcessPowerThrottling,
            &state as *const _ as *const c_void,
            size_of::<PROCESS_POWER_THROTTLING_STATE>() as u32,
        )?;

        // Task Manager's Efficiency mode also drops the priority class.
        use windows::Win32::System::Threading::NORMAL_PRIORITY_CLASS;
        SetPriorityClass(
            process,
            if enable {
                IDLE_PRIORITY_CLASS
            } else {
                NORMAL_PRIORITY_CLASS
            },
        )?;
    }
    Ok(())
}
