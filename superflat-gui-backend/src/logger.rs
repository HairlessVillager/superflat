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

pub struct GuiLogger {
    app: Mutex<Option<AppHandle>>,
    file: Mutex<Option<BufWriter<File>>>,
    op_start: Mutex<Option<Instant>>,
}

impl GuiLogger {
    pub const fn new() -> Self {
        Self {
            app: Mutex::new(None),
            file: Mutex::new(None),
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
        match File::create(&log_path) {
            Ok(f) => {
                *self.file.lock().expect("gui logger file mutex poisoned") =
                    Some(BufWriter::new(f));
            }
            Err(e) => eprintln!("Failed to open log file {:?}: {}", log_path, e),
        }
        *self.app.lock().expect("gui logger mutex is poisoned") = Some(app);
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

        // Always write to file
        if let Ok(mut guard) = self.file.lock() {
            if let Some(w) = guard.as_mut() {
                let elapsed_s = self
                    .op_start
                    .lock()
                    .ok()
                    .and_then(|g| g.map(|t| t.elapsed().as_secs_f64()))
                    .unwrap_or(0.0);
                let int_part = elapsed_s.floor() as u64;
                let frac_digits = ((elapsed_s - int_part as f64) * 1000.0).round() as u64;
                let _ = writeln!(w, "[{:>4}.{:03}] [{}] {}", int_part, frac_digits, level_str, message);
            }
        }

        // Only forward Info and above to the GUI
        if record.level() > Level::Info {
            return;
        }

        let payload = LogPayload { level: level_str, message };
        if let Some(app) = self.app.lock().expect("gui logger mutex is poisoned").clone() {
            let _ = app.emit(EVENT_OUTPUT, payload);
        }
    }

    fn flush(&self) {
        if let Ok(mut guard) = self.file.lock() {
            if let Some(w) = guard.as_mut() {
                let _ = w.flush();
            }
        }
    }
}

