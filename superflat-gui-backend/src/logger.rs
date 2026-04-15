use std::{
    fs::{self, File},
    io::{BufWriter, Write},
    sync::Mutex,
    time::Instant,
};

use log::{Level, LevelFilter, Log, Metadata, Record};
use serde::Serialize;
use tauri::{AppHandle, Emitter};

use crate::EVENT_OUTPUT;

pub const LOG_FILE: &str = "latest.log";

#[derive(Serialize, Clone)]
pub struct LogPayload {
    pub level: &'static str,
    pub message: String,
}

struct LoggerState {
    app: Option<AppHandle>,
    file: Option<BufWriter<File>>,
}

pub struct GuiLogger {
    state: Mutex<LoggerState>,
    op_start: Mutex<Option<Instant>>,
}

impl GuiLogger {
    pub fn new() -> Self {
        Self {
            state: Mutex::new(LoggerState {
                app: None,
                file: None,
            }),
            op_start: Mutex::new(None),
        }
    }

    /// Reset the operation start time. Call this at the beginning of each command.
    pub fn reset_op_start(&self) {
        if let Ok(mut guard) = self.op_start.lock() {
            *guard = Some(Instant::now());
        }
    }

    /// Called once at startup to open the log file and wire up the app handle.
    /// Always writes at Debug level; GUI only receives Info and above.
    pub fn configure(&self, app: AppHandle, log_path: std::path::PathBuf) {
        if let Some(parent) = log_path.parent() {
            let _ = fs::create_dir_all(parent);
        }
        let file = match File::create(&log_path) {
            Ok(f) => Some(BufWriter::new(f)),
            Err(e) => {
                eprintln!("Failed to open log file {:?}: {}", log_path, e);
                None
            }
        };
        let mut state = self.state.lock().expect("gui logger mutex is poisoned");
        state.app = Some(app);
        state.file = file;
    }
}

impl Log for GuiLogger {
    fn enabled(&self, metadata: &Metadata) -> bool {
        metadata.level() <= LevelFilter::Debug
    }

    fn log(&self, record: &Record) {
        if !self.enabled(record.metadata()) {
            return;
        }

        let level_str = match record.level() {
            Level::Error => "ERROR",
            Level::Warn => "WARN",
            Level::Info => "INFO",
            Level::Debug => "DEBUG",
            Level::Trace => "TRACE",
        };
        let message = record.args().to_string();

        // Get current UTC time for file log
        let now = chrono::Local::now()
            .to_rfc3339_opts(chrono::SecondsFormat::Micros, true)
            .to_string();

        let mut state = match self.state.lock() {
            Ok(g) => g,
            Err(_) => return,
        };

        // Write to file (all levels, full timestamp)
        if let Some(w) = state.file.as_mut() {
            let _ = writeln!(w, "[{}] [{}] {}", now, level_str, message);
        }

        // Only forward Info and above to the GUI
        if record.level() > Level::Info {
            return;
        }

        let payload = LogPayload {
            level: level_str,
            message,
        };
        let app = state.app.clone();
        drop(state);
        if let Some(app) = app {
            let _ = app.emit(EVENT_OUTPUT, payload);
        }
    }

    fn flush(&self) {
        if let Ok(mut state) = self.state.lock() {
            if let Some(w) = state.file.as_mut() {
                let _ = w.flush();
            }
        }
    }
}
