// libretro logger: bridges Rust `log` crate to retro_log_printf_t.

use super::callbacks;
use super::types::retro_log_level;
use log::{Level, LevelFilter, Log, Metadata, Record};

struct LibretroLogger;

impl Log for LibretroLogger {
    fn enabled(&self, _metadata: &Metadata) -> bool {
        true
    }

    fn log(&self, record: &Record) {
        if !self.enabled(record.metadata()) {
            return;
        }

        let level = match record.level() {
            Level::Error => retro_log_level::RETRO_LOG_ERROR,
            Level::Warn => retro_log_level::RETRO_LOG_WARN,
            Level::Info => retro_log_level::RETRO_LOG_INFO,
            Level::Debug | Level::Trace => retro_log_level::RETRO_LOG_DEBUG,
        };

        let msg = format!("{}", record.args());
        callbacks::log_message(level, &msg);
    }

    fn flush(&self) {}
}

static LOGGER: LibretroLogger = LibretroLogger;

/// Initialize the Rust `log` crate to forward to libretro's log interface.
pub fn init() {
    // Ignore error if logger is already set
    let _ = log::set_logger(&LOGGER);
    log::set_max_level(LevelFilter::Debug);
}
