pub fn t(key: Key) -> &'static str {
    use Key::*;
    match *LANG {
        Lang::En => match key {
            no_adapter_found => "No headphone adapter found",
            view_logs => "View logs",
            quit_program => "Close",
            device_charging => "(Charging)",
            device_disconnected => "(Disconnected)",
            battery_unavailable => "(Battery unavailable)",
            show_notifications => "Show notifications",
            show_text_icon => "Show battery percentage as number icon",
            notifications_enabled_message => "Notifications enabled",
            version => "Version",
            update_available => "Update available",
            choose_icon_color => "Choose icon color",
        },
        Lang::Fi => match key {
            no_adapter_found => "Kuulokeadapteria ei löytynyt",
            view_logs => "Näytä lokitiedostot",
            quit_program => "Sulje",
            device_charging => "(Latautuu)",
            device_disconnected => "(Ei yhteyttä)",
            battery_unavailable => "(Akku ei saatavilla)",
            show_notifications => "Näytä ilmoitukset",
            notifications_enabled_message => "Ilmoitukset käytössä",
            show_text_icon => "Näytä akun tila numerona",
            version => "Versio",
            update_available => "Päivitys saatavilla",
            choose_icon_color => "Valitse ikonin väri",
        },
        Lang::De => match key {
            no_adapter_found => "Kein Kopfhöreradapter gefunden",
            view_logs => "Protokolle anzeigen",
            quit_program => "Beenden",
            device_charging => "(Wird geladen)",
            device_disconnected => "(Getrennt)",
            battery_unavailable => "(Akkustand nicht verfügbar)",
            show_notifications => "Benachrichtigungen aktivieren",
            notifications_enabled_message => "Benachrichtigungen aktiviert",
            show_text_icon => "Batteriestatus als Zahlensymbol anzeigen",
            version => "Version",
            update_available => "Update verfügbar",
            choose_icon_color => "Symbolfarbe wählen",
        },
        Lang::It => match key {
            no_adapter_found => "Nessun adattatore per cuffie trovato",
            view_logs => "Visualizza file di log",
            quit_program => "Chiudi",
            device_charging => "(In carica)",
            device_disconnected => "(Disconnesso)",
            battery_unavailable => "(Batteria non disponibile)",
            show_notifications => "Mostra notifiche",
            notifications_enabled_message => "Notifiche attivate",
            show_text_icon => "Mostra stato batteria come icona numerica",
            version => "Versione",
            update_available => "Aggiornamento disponibile",
            choose_icon_color => "Scegli colore icona",
        },
        Lang::Pt => match key {
            no_adapter_found => "Não foi encontrado adaptador de auscultadores",
            view_logs => "Ver registos",
            quit_program => "Fechar",
            device_charging => "(A carregar)",
            device_disconnected => "(Desconectado)",
            battery_unavailable => "(Bateria indisponível)",
            show_notifications => "Mostrar notificações",
            notifications_enabled_message => "Notificações habilitadas",
            show_text_icon => "Mostrar estado da bateria como ícone numérico",
            version => "Versão",
            update_available => "Atualização disponível",
            choose_icon_color => "Escolher cor do ícone",
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
    no_adapter_found,
    view_logs,
    quit_program,
    device_charging,
    device_disconnected,
    battery_unavailable,
    show_notifications,
    show_text_icon,
    notifications_enabled_message,
    version,
    update_available,
    choose_icon_color,
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
