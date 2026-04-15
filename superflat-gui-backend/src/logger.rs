use std::{
    fs::{self, DirEntry, File},
    io::{BufWriter, Write},
    sync::Mutex,
    time::Instant,
};

use chrono::Utc;
use log::{Level, LevelFilter, Log, Metadata, Record};
use serde::Serialize;
use tauri::{AppHandle, Emitter};

use crate::EVENT_OUTPUT;

pub const LOG_DIR: &str = "logs";
const MAX_LOG_SIZE_BYTES: u64 = 10 * 1024 * 1024; // 10 MiB

#[derive(Serialize, Clone)]
pub struct LogPayload {
    pub level: &'static str,
    pub message: String,
}

struct LoggerState {
    app: Option<AppHandle>,
    file: Option<BufWriter<File>>,
    current_log_path: Option<std::path::PathBuf>,
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
                current_log_path: None,
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

    /// Called once at startup to open a new log file in the logs directory.
    pub fn configure(&self, app: AppHandle, app_data_dir: std::path::PathBuf) {
        let log_dir = app_data_dir.join(LOG_DIR);
        if let Err(e) = fs::create_dir_all(&log_dir) {
            eprintln!("Failed to create log directory {:?}: {}", log_dir, e);
            return;
        }

        // Clean up old logs before creating new one
        if let Err(e) = self.cleanup_old_logs(&log_dir) {
            eprintln!("Failed to cleanup old logs: {}", e);
        }

        // Generate timestamp-based filename
        let timestamp = Utc::now().to_rfc3339_opts(chrono::SecondsFormat::Secs, true);
        let log_filename = format!("{}.log", timestamp);
        let log_path = log_dir.join(&log_filename);

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
        state.current_log_path = Some(log_path);
    }

    /// Get the path to the current log file for the frontend to display
    pub fn get_current_log_path(&self) -> Option<std::path::PathBuf> {
        self.state
            .lock()
            .ok()
            .and_then(|s| s.current_log_path.clone())
    }

    /// Clean up old log files when total size exceeds 10 MiB
    fn cleanup_old_logs(&self, log_dir: &std::path::Path) -> std::io::Result<()> {
        let mut entries: Vec<DirEntry> = Vec::new();
        for entry in fs::read_dir(log_dir)? {
            let entry = entry?;
            if entry.file_type()?.is_file() {
                if let Some(ext) = entry.path().extension() {
                    if ext == "log" {
                        entries.push(entry);
                    }
                }
            }
        }

        // Sort by modification time (oldest first)
        entries.sort_by_key(|e| e.metadata().ok().and_then(|m| m.modified().ok()));

        let mut total_size: u64 = 0;
        for entry in &entries {
            total_size += entry.metadata()?.len();
        }

        // Delete oldest files until under threshold
        for entry in entries {
            if total_size <= MAX_LOG_SIZE_BYTES {
                break;
            }
            let size = entry.metadata()?.len();
            if let Err(e) = fs::remove_file(entry.path()) {
                eprintln!("Failed to delete old log {:?}: {}", entry.path(), e);
            } else {
                total_size -= size;
            }
        }

        Ok(())
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
