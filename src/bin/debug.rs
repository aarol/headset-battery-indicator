use headset_battery_indicator::run;
use log::error;
use simplelog::{Config, TermLogger};

fn main() {
    TermLogger::init(
        log::LevelFilter::Trace,
        Config::default(),
        simplelog::TerminalMode::Mixed,
        simplelog::ColorChoice::Auto,
    )
    .unwrap();

    if let Err(e) = run() {
        error!("Application stopped unexpectedly: {e:?}");
    }
}
