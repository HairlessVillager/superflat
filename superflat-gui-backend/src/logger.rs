use std::sync::{
    Mutex,
    atomic::{AtomicU8, Ordering},
};

use log::{Level, LevelFilter, Log, Metadata, Record};
use serde::Serialize;
use tauri::{AppHandle, Emitter};

use crate::EVENT_OUTPUT;

#[derive(Serialize, Clone)]
pub struct LogPayload {
    pub level: &'static str,
    pub message: String,
}

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

        let payload = LogPayload {
            level: match record.level() {
                Level::Error => "ERROR",
                Level::Warn => "WARN",
                Level::Info => "INFO",
                Level::Debug => "DEBUG",
                Level::Trace => "TRACE",
            },
            message: record.args().to_string(),
        };

        if let Some(app) = self.app.lock().expect("gui logger mutex is poisoned").clone() {
            let _ = app.emit(EVENT_OUTPUT, payload);
        }
    }

    fn flush(&self) {}
}
