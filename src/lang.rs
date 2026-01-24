pub fn t(key: Key) -> &'static str {
    use Key::*;
    match *LANG {
        Lang::En => match key {
            battery_remaining => "remaining",
            no_adapter_found => "No headphone adapter found",
            view_logs => "View logs",
            quit_program => "Close",
            device_charging => "(Charging)",
            device_disconnected => "(Disconnected)",
            battery_unavailable => "(Battery unavailable)",
            show_notifications => "Show notifications",
            notifications_enabled_message => "Notifications enabled",
            version => "Version",
            update_available => "Update available",
        },
        Lang::Fi => match key {
            battery_remaining => "jäljellä",
            no_adapter_found => "Kuulokeadapteria ei löytynyt",
            view_logs => "Näytä lokitiedostot",
            quit_program => "Sulje",
            device_charging => "(Latautuu)",
            device_disconnected => "(Ei yhteyttä)",
            battery_unavailable => "(Akku ei saatavilla)",
            show_notifications => "Näytä ilmoitukset",
            notifications_enabled_message => "Ilmoitukset käytössä",
            version => "Versio",
            update_available => "Päivitys saatavilla",
        },
        Lang::De => match key {
            battery_remaining => "verbleibend",
            no_adapter_found => "Kein Kopfhöreradapter gefunden",
            view_logs => "Protokolle anzeigen",
            quit_program => "Beenden",
            device_charging => "(Wird geladen)",
            device_disconnected => "(Getrennt)",
            battery_unavailable => "(Akkustand nicht verfügbar)",
            show_notifications => "Benachrichtigungen aktivieren",
            notifications_enabled_message => "Benachrichtigungen aktiviert",
            version => "Version",
            update_available => "Update verfügbar",
        },
        Lang::It => match key {
            battery_remaining => "rimanente",
            no_adapter_found => "Nessun adattatore per cuffie trovato",
            view_logs => "Visualizza file di log",
            quit_program => "Chiudi",
            device_charging => "(In carica)",
            device_disconnected => "(Disconnesso)",
            battery_unavailable => "(Batteria non disponibile)",
            show_notifications => "Mostra notifiche",
            notifications_enabled_message => "Notifiche attivate",
            version => "Versione",
            update_available => "Aggiornamento disponibile",
        },
        Lang::Pt => match key {
            battery_remaining => "restante",
            no_adapter_found => "Não foi encontrado adaptador de auscultadores",
            view_logs => "Ver registos",
            quit_program => "Fechar",
            device_charging => "(A carregar)",
            device_disconnected => "(Desconectado)",
            battery_unavailable => "(Bateria indisponível)",
            show_notifications => "Mostrar notificações",
            notifications_enabled_message => "Notificações habilitadas",
            version => "Versão",
            update_available => "Atualização disponível",
        },
    }
}

#[derive(Debug)]
pub enum Lang {
    En,
    Fi,
    De,
    It,
    Pt,
}

#[allow(non_camel_case_types)]
pub enum Key {
    battery_remaining,
    no_adapter_found,
    view_logs,
    quit_program,
    device_charging,
    device_disconnected,
    battery_unavailable,
    show_notifications,
    notifications_enabled_message,
    version,
    update_available,
}

use std::sync::LazyLock;

use log::debug;

pub static LANG: LazyLock<Lang> = LazyLock::new(|| {
    let locale = &sys_locale::get_locale().unwrap_or("en-US".to_owned());
    debug!("Detected system locale: {}", locale);
    match locale.as_str() {
        "fi" | "fi-FI" => Lang::Fi,
        "de" | "de-DE" | "de-AT" | "de-CH" => Lang::De,
        "it" | "it-IT" | "it-CH" => Lang::It,
        "pt" | "pt-PT" | "pt-BR" => Lang::Pt,
        _ => Lang::En,
    }
});
