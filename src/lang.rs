pub fn t(key: Key) -> &'static str {
    use Key::*;
    match *LANG {
        Lang::En => match key {
            no_headset_found => "No headset found",
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
        },
        Lang::Fi => match key {
            no_headset_found => "Kuulokkeita ei löytynyt",
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
        },
        Lang::De => match key {
            no_headset_found => "Kein Headset gefunden",
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
        },
        Lang::It => match key {
            no_headset_found => "Nessuna cuffia trovata",
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
        },
        Lang::Pt => match key {
            no_headset_found => "Nenhum headset encontrado",
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
        },
        Lang::Zh => match key {
            no_headset_found => "未找到耳机",
            view_logs => "查看日志",
            quit_program => "关闭",
            device_charging => "(充电中)",
            device_disconnected => "(未连接)",
            battery_unavailable => "(电池不可用)",
            show_notifications => "显示通知",
            show_text_icon => "以数字百分比显示电池图标",
            notifications_enabled_message => "通知已启用",
            version => "版本",
            update_available => "有更新可用",
        },
        Lang::Es => match key {
            no_headset_found => "No se encontraron auriculares",
            view_logs => "Ver registros",
            quit_program => "Cerrar",
            device_charging => "(Cargando)",
            device_disconnected => "(Desconectado)",
            battery_unavailable => "(Batería no disponible)",
            show_notifications => "Mostrar notificaciones",
            show_text_icon => "Mostrar el porcentaje de batería como un icono numérico",
            notifications_enabled_message => "Notificaciones habilitadas",
            version => "Versión",
            update_available => "Actualización disponible",
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
    Zh,
    Es,
}

#[allow(non_camel_case_types)]
pub enum Key {
    no_headset_found,
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
        "zh" | "zh-CN" => Lang::Zh,
        "es" | "es-ES" | "es-MX" => Lang::Es,
        _ => Lang::En,
    }
});
