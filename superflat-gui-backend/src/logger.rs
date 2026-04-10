use std::sync::{
    Mutex,
    atomic::{AtomicU8, Ordering},
};

use chrono::Local;
use log::{Level, LevelFilter, Log, Metadata, Record};
use tauri::{AppHandle, Emitter};

use crate::EVENT_OUTPUT;

pub struct GuiLogger {
    level: AtomicU8,
    app: Mutex<Option<AppHandle>>,
}

impl GuiLogger {
    pub const fn new() -> Self {
        Self {
            level: AtomicU8::new(Self::encode_level(LevelFilter::Info)),
            app: Mutex::new(None),
        }
    }

    const fn encode_level(level: LevelFilter) -> u8 {
        match level {
            LevelFilter::Off => 0,
            LevelFilter::Error => 1,
            LevelFilter::Warn => 2,
            LevelFilter::Info => 3,
            LevelFilter::Debug => 4,
            LevelFilter::Trace => 5,
        }
    }

    fn decode_level(value: u8) -> LevelFilter {
        match value {
            0 => LevelFilter::Off,
            1 => LevelFilter::Error,
            2 => LevelFilter::Warn,
            3 => LevelFilter::Info,
            4 => LevelFilter::Debug,
            _ => LevelFilter::Trace,
        }
    }

    fn current_level(&self) -> LevelFilter {
        Self::decode_level(self.level.load(Ordering::Relaxed))
    }

    pub fn configure(&self, app: AppHandle, debug: bool) {
        self.level.store(
            Self::encode_level(if debug {
                LevelFilter::Debug
            } else {
                LevelFilter::Info
            }),
            Ordering::Relaxed,
        );
        *self.app.lock().expect("gui logger mutex is poisoned") = Some(app);
        log::set_max_level(self.current_level());
    }
}

impl Log for GuiLogger {
    fn enabled(&self, metadata: &Metadata) -> bool {
        metadata.level() <= self.current_level()
    }

    fn log(&self, record: &Record) {
        if !self.enabled(record.metadata()) {
            return;
        }

        let timestamp = Local::now().format("%Y-%m-%d %H:%M:%S%.3f");
        let line = format!(
            "{} [{}] {}",
            timestamp,
            match record.level() {
                Level::Error => "ERROR",
                Level::Warn => "WARN",
                Level::Info => "INFO",
                Level::Debug => "DEBUG",
                Level::Trace => "TRACE",
            },
            record.args()
        );

        if let Some(app) = self.app.lock().expect("gui logger mutex is poisoned").clone() {
            let _ = app.emit(EVENT_OUTPUT, line);
        }
    }

    fn flush(&self) {}
}
