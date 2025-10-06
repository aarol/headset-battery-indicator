use anyhow::Context;
use log::error;
use tray_icon::menu::MenuEvent;
use tray_icon::menu::{CheckMenuItem, Menu, MenuItem, PredefinedMenuItem};
use winit::event_loop;

use crate::headset_control;
use crate::lang;
use crate::lang::Key::*;

pub struct ContextMenu {
    pub menu: Menu,
    device_menu_items: Vec<(headset_control::Device, CheckMenuItem)>,
    pub selected_device_idx: usize,
    separators: Option<(PredefinedMenuItem, PredefinedMenuItem)>, // (top, bottom)
    menu_logs: MenuItem,
    menu_github: MenuItem,
    menu_close: MenuItem,
}

impl ContextMenu {
    pub fn new() -> anyhow::Result<Self> {
        let menu = Menu::new();

        menu.append(&MenuItem::new(
            format!("{} v{}", lang::t(version), crate::VERSION),
            false,
            None,
        ))?;

        let device_menu_items = Vec::new();

        let menu_logs = MenuItem::new(lang::t(view_logs), true, None);
        let menu_github = MenuItem::new(lang::t(view_updates), true, None);
        let menu_close = MenuItem::new(lang::t(quit_program), true, None);
        let separators = None;

        menu.append_items(&[&menu_logs, &menu_github])?;
        menu.append(&PredefinedMenuItem::separator())?;
        menu.append(&menu_close)?;

        Ok(Self {
            menu,
            device_menu_items,
            selected_device_idx: 0,
            separators,
            menu_logs,
            menu_github,
            menu_close,
        })
    }

    pub fn update_device_menu(
        &mut self,
        devices: &[headset_control::Device],
    ) -> anyhow::Result<()> {
        // Remove separators
        if let Some((top, bottom)) = &self.separators {
            self.menu.remove(top).context("Removing top separator")?;
            self.menu
                .remove(bottom)
                .context("Removing bottom separator")?;
            self.separators = None;
        }

        // Remove old device menu items
        for (_, item) in &self.device_menu_items {
            self.menu.remove(item)?;
        }
        if devices.is_empty() {
            self.selected_device_idx = 0;
            return Ok(());
        }

        let (top_separator, bottom_separator) = (
            PredefinedMenuItem::separator(),
            PredefinedMenuItem::separator(),
        );

        self.device_menu_items.clear();
        self.menu.insert(&top_separator, 1)?;

        self.selected_device_idx = self.selected_device_idx.min(devices.len() - 1);

        // Add new device menu items
        for (i, device) in devices.iter().enumerate() {
            let is_selected = i == self.selected_device_idx;
            let menu_item = CheckMenuItem::new(device.product.clone(), true, is_selected, None);
            self.menu.insert(&menu_item, 2 + i as usize)?; // Insert after version item
            self.device_menu_items.push((device.clone(), menu_item));
        }

        self.menu.insert(&bottom_separator, 2 + devices.len())?;
        self.separators = Some((top_separator, bottom_separator));

        Ok(())
    }

    fn set_selected(&mut self, idx: usize) {
        if idx >= self.device_menu_items.len() {
            return;
        }

        for (i, (_, item)) in self.device_menu_items.iter().enumerate() {
            item.set_checked(i == idx);
        }
        self.selected_device_idx = idx;
    }

    pub fn handle_event(&mut self, event: MenuEvent, event_loop: &event_loop::ActiveEventLoop) {
        match event.id {
            id if id == self.menu_close.id() => event_loop.exit(),

            id if id == self.menu_github.id() => {
                let url = "https://github.com/aarol/headset-battery-indicator/releases";

                if let Err(e) = std::process::Command::new("explorer").arg(url).spawn() {
                    error!("Failed to open {url}: {e:?}");
                }
            }
            id if id == self.menu_logs.id() => {
                if let Ok(dir) = std::env::current_dir() {
                    if let Err(e) = std::process::Command::new("explorer").arg(&dir).spawn() {
                        error!("Failed to open path {dir:?}: {e:?}");
                    }
                }
            }
            id => {
                let idx = self
                    .device_menu_items
                    .iter()
                    .enumerate()
                    .find(|(_, (_, m))| m.id() == &id);
                if let Some((i, _)) = idx {
                    self.set_selected(i);
                }
            }
        }
    }
}
