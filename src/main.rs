#![windows_subsystem = "windows"]

use std::fs::File;

use headset_battery_indicator::run;
use log::error;
use simplelog::{ConfigBuilder, WriteLogger};

fn main() {
    // Cannot really log anything if initializing logging fails
    let _ = init_file_logger();

    if let Err(e) = run() {
        error!("Application stopped unexpectedly: {e:?}");
    }
}

pub fn init_file_logger() -> anyhow::Result<()> {
    match std::env::current_dir() {
        Err(err) => {
            anyhow::bail!("Failed to get current directory: {err}");
        }
        Ok(curr_dir) => {
            let log_file = File::options()
                .append(true)
                .create(true)
                .open(curr_dir.join("headset-battery-indicator.log"))?;

            WriteLogger::init(
                log::LevelFilter::Info,
                ConfigBuilder::new().set_time_format_rfc3339().build(),
                log_file,
            )?;

            Ok(())
        }
    }
}
