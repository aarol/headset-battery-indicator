use std::ffi::OsStr;
use std::fmt::Debug;

use anyhow::Context;
use log::error;
use tray_icon::menu::MenuEvent;
use tray_icon::menu::{CheckMenuItem, Menu, MenuItem, PredefinedMenuItem, SubmenuBuilder};
use winit::event_loop;

use crate::lang;
use crate::lang::Key;
use crate::settings::{IconType, Settings};

pub struct ContextMenu {
    pub menu: Menu,
    pub menu_enable_notifications: CheckMenuItem,
    pub icon_image: CheckMenuItem,
    pub icon_text_gdi: CheckMenuItem,
    pub icon_text_bitmap: CheckMenuItem,
    menu_logs: MenuItem,
    menu_close: MenuItem,
    pub menu_trigger_notification: MenuItem,
    menu_update_available: Option<MenuItem>,
}

impl ContextMenu {
    pub fn new(settings: Settings) -> anyhow::Result<Self> {
        let menu = Menu::new();

        menu.append(&MenuItem::new(
            format!("{} v{}", lang::t(Key::version), crate::VERSION),
            false,
            None,
        ))?;

        let menu_notifications = CheckMenuItem::new(
            lang::t(Key::show_notifications),
            true,
            settings.notifications_enabled,
            None,
        );

        let icon_image = CheckMenuItem::new(
            lang::t(Key::icon_submenu_image),
            true,
            matches!(settings.icon_type, IconType::Image),
            None,
        );
        let icon_text_gdi = CheckMenuItem::new(
            lang::t(Key::icon_submenu_text_gdi),
            true,
            matches!(settings.icon_type, IconType::TextGdi),
            None,
        );
        let icon_text_bitmap = CheckMenuItem::new(
            lang::t(Key::icon_submenu_text_bitmap),
            true,
            matches!(settings.icon_type, IconType::TextBitmap),
            None,
        );

        let icon_submenu = SubmenuBuilder::new()
            .text(lang::t(Key::icon_submenu))
            .items(&[&icon_image, &icon_text_gdi, &icon_text_bitmap])
            .build()?;

        let menu_logs = MenuItem::new(lang::t(Key::view_logs), true, None);
        let menu_close = MenuItem::new(lang::t(Key::quit_program), true, None);
        let menu_trigger_notification = MenuItem::new("Trigger Test Notification", true, None);

        #[cfg(debug_assertions)]
        menu.append(&menu_trigger_notification)?;

        menu.append_items(&[&menu_notifications, &icon_submenu, &menu_logs])?;
        menu.append(&PredefinedMenuItem::separator())?;
        menu.append(&menu_close)?;

        Ok(Self {
            menu,
            menu_enable_notifications: menu_notifications,
            icon_image,
            icon_text_gdi,
            icon_text_bitmap,
            menu_logs,
            menu_close,
            menu_trigger_notification,
            menu_update_available: None,
        })
    }

    /// Shows an "Update available" menu item at the top of the menu
    pub fn show_update_available(&mut self) -> anyhow::Result<()> {
        if self.menu_update_available.is_some() {
            return Ok(()); // Already showing
        }

        let update_text = format!("❗ {}", lang::t(Key::update_available));
        let menu_item = MenuItem::new(update_text, true, None);

        // Insert at position 1 (after version item)
        self.menu.insert(&menu_item, 1)?;
        self.menu_update_available = Some(menu_item);

        Ok(())
    }

    pub fn set_icon_type(&mut self, icon_type: IconType) {
        self.icon_image
            .set_checked(matches!(icon_type, IconType::Image));
        self.icon_text_gdi
            .set_checked(matches!(icon_type, IconType::TextGdi));
        self.icon_text_bitmap
            .set_checked(matches!(icon_type, IconType::TextBitmap));
    }

    pub fn handle_event(&mut self, event: MenuEvent, event_loop: &event_loop::ActiveEventLoop) {
        match event.id {
            id if id == self.menu_close.id() => event_loop.exit(),

            id if self
                .menu_update_available
                .as_ref()
                .is_some_and(|m| *m.id() == id) =>
            {
                let url = "https://github.com/aarol/headset-battery-indicator/releases";
                {
                    if let Err(err) = Self::spawn_command_no_window("explorer", &[&url]) {
                        error!("Failed to open update URL {url}: {err:?}");
                    }
                }
            }
            id if id == self.menu_logs.id() => {
                if let Ok(dir) = std::env::current_exe().map(|p| p.parent().unwrap().to_path_buf())
                {
                    let path = dir.join("headset-battery-indicator.log");
                    {
                        if let Err(e) = Self::spawn_command_no_window("explorer", &[&path]) {
                            error!("Failed to open log file at {}: {e:?}", path.display());
                        }
                    }
                }
            }
            _ => {}
        }
    }

    fn spawn_command_no_window<S>(cmd: &str, args: &[S]) -> anyhow::Result<()>
    where
        S: AsRef<OsStr> + Debug,
    {
        use std::os::windows::process::CommandExt;
        const CREATE_NO_WINDOW: u32 = 0x08000000;

        let mut command = std::process::Command::new(cmd);
        command.args(args).creation_flags(CREATE_NO_WINDOW);

        command
            .spawn()
            .with_context(|| format!("Failed to spawn command: {} {:?}", cmd, args))?;

        Ok(())
    }
}
