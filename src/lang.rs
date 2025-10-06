use log::info;

enum Lang {
    En,
    Fi,
}

#[allow(non_camel_case_types)]
pub enum Key {
    battery_remaining,
    no_adapter_found,
    view_logs,
    view_updates,
    quit_program,
    device_charging,
    device_disconnected,
    battery_unavailable,
    version,
}

use std::sync::LazyLock;

static LANG: LazyLock<Lang> = LazyLock::new(|| {
    let locale = &sys_locale::get_locale().unwrap_or("en-US".to_owned());
    info!("Using locale {locale}");
    match locale.as_str() {
        "fi" | "fi-FI" => Lang::Fi,
        _ => Lang::En,
    }
});

pub fn t(key: Key) -> &'static str {
    use Key::*;
    match *LANG {
        Lang::En => match key {
            battery_remaining => "remaining",
            no_adapter_found => "No headphone adapter found",
            view_logs => "View logs",
            view_updates => "View updates",
            quit_program => "Close",
            device_charging => "(Charging)",
            device_disconnected => "(Disconnected)",
            battery_unavailable => "(Battery unavailable)",
            version => "Version",
        },
        Lang::Fi => match key {
            battery_remaining => "jäljellä",
            
            no_adapter_found => "Kuulokeadapteria ei löytynyt",
            view_logs => "Näytä lokitiedostot",
            view_updates => "Näytä päivitykset",
            quit_program => "Sulje",
            device_charging => "(Latautuu)",
            device_disconnected => "(Ei yhteyttä)",
            battery_unavailable => "(Akku ei saatavilla)",
            version => "Versio",
        },
    }
}
