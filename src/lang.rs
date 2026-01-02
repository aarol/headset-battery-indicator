use log::info;

enum Lang {
    En,
    Fi,
    De,
    It,
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
        "de" | "de-DE" | "de-AT" | "de-CH" => Lang::De,
        "it" | "it-IT" | "it-CH" => Lang::It,
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
        Lang::De => match key {
            battery_remaining => "verbleibend",
            no_adapter_found => "Kein Kopfhöreradapter gefunden",
            view_logs => "Protokolle anzeigen",
            view_updates => "Updates anzeigen",
            quit_program => "Beenden",
            device_charging => "(Wird geladen)",
            device_disconnected => "(Getrennt)",
            battery_unavailable => "(Akkustand nicht verfügbar)",
            version => "Version",
        },
        Lang::It => match key {
            battery_remaining => "rimanente",
            no_adapter_found => "Nessun adattatore per cuffie trovato",
            view_logs => "Visualizza file di log",
            view_updates => "Controlla aggiornamenti",
            quit_program => "Chiudi",
            device_charging => "(In carica)",
            device_disconnected => "(Disconnesso)",
            battery_unavailable => "(Batteria non disponibile)",
            version => "Versione",
        },
    }
}
