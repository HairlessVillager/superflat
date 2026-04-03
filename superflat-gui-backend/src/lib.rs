use std::{
    path::PathBuf,
    sync::{
        Mutex,
        atomic::{AtomicU8, Ordering},
    },
};

use chrono::Local;
use log::{Level, LevelFilter, Log, Metadata, Record};
use tauri::{AppHandle, Emitter};
use tauri_plugin_dialog::DialogExt;
use tauri_plugin_store::StoreExt;
use tokio::sync::oneshot;

use serde::{Deserialize, Serialize};

const STORE_FILE: &str = "settings.json";
const PROFILES_FILE: &str = "profiles.json";
const KEY_BRANCH: &str = "branch";
const DEFAULT_BRANCH: &str = "main";
const KEY_MC_VERSION: &str = "mc_version";
const DEFAULT_MC_VERSION: &str = "1.21.11";
const KEY_DEFAULT_COMMIT: &str = "default_commit";
const DEFAULT_DEFAULT_COMMIT: &str = "main@{10 minutes ago}";
const KEY_DEBUG: &str = "debug";
const DEFAULT_DEBUG: bool = false;

struct GuiLogger {
    level: AtomicU8,
    app: Mutex<Option<AppHandle>>,
}

impl GuiLogger {
    const fn new() -> Self {
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

    fn configure(&self, app: AppHandle, debug: bool) {
        self.level.store(
            Self::encode_level(if debug {
                LevelFilter::Debug
            } else {
                LevelFilter::Info
            }),
            Ordering::Relaxed,
        );
        *self.app.lock().unwrap() = Some(app);
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

        if let Some(app) = self.app.lock().unwrap().clone() {
            let _ = app.emit("commit-output", line);
        }
    }

    fn flush(&self) {}
}

static GUI_LOGGER: GuiLogger = GuiLogger::new();

#[tauri::command]
async fn pick_directory(app: AppHandle) -> Option<String> {
    let (tx, rx) = oneshot::channel();
    app.dialog().file().pick_folder(move |folder| {
        let _ = tx.send(folder);
    });
    rx.await
        .ok()
        .flatten()
        .and_then(|p| p.into_path().ok())
        .map(|p| p.to_string_lossy().into_owned())
}

#[derive(Serialize)]
struct Settings {
    branch: String,
    mc_version: String,
    default_commit: String,
    debug: bool,
}

#[tauri::command]
fn get_settings(app: AppHandle) -> Settings {
    let store = app.store(STORE_FILE).expect("failed to open store");
    let branch = store
        .get(KEY_BRANCH)
        .and_then(|v| v.as_str().map(str::to_owned))
        .unwrap_or_else(|| DEFAULT_BRANCH.to_owned());
    let mc_version = store
        .get(KEY_MC_VERSION)
        .and_then(|v| v.as_str().map(str::to_owned))
        .unwrap_or_else(|| DEFAULT_MC_VERSION.to_owned());
    let default_commit = store
        .get(KEY_DEFAULT_COMMIT)
        .and_then(|v| v.as_str().map(str::to_owned))
        .unwrap_or_else(|| DEFAULT_DEFAULT_COMMIT.to_owned());
    let debug = store
        .get(KEY_DEBUG)
        .and_then(|v| v.as_bool())
        .unwrap_or(DEFAULT_DEBUG);
    Settings {
        branch,
        mc_version,
        default_commit,
        debug,
    }
}

#[tauri::command]
fn save_settings(
    app: AppHandle,
    branch: String,
    mc_version: String,
    default_commit: String,
    debug: bool,
) {
    let store = app.store(STORE_FILE).expect("failed to open store");
    store.set(KEY_BRANCH, branch);
    store.set(KEY_MC_VERSION, mc_version);
    store.set(KEY_DEFAULT_COMMIT, default_commit);
    store.set(KEY_DEBUG, debug);
    store.save().expect("failed to save store");
    GUI_LOGGER.configure(app, debug);
}

#[derive(Serialize, Deserialize, Clone)]
struct Profile {
    save_dir: String,
    mc_version: String,
    branch: String,
    default_commit: String,
}

#[tauri::command]
fn get_profiles(app: AppHandle) -> Vec<Profile> {
    let store = app
        .store(PROFILES_FILE)
        .expect("failed to open profiles store");
    let mut profiles: Vec<Profile> = store
        .keys()
        .into_iter()
        .filter_map(|k| store.get(&k).and_then(|v| serde_json::from_value(v).ok()))
        .collect();
    // sort by save_dir for stable ordering
    profiles.sort_by(|a, b| a.save_dir.cmp(&b.save_dir));
    profiles
}

#[tauri::command]
fn upsert_profile(app: AppHandle, profile: Profile) {
    let store = app
        .store(PROFILES_FILE)
        .expect("failed to open profiles store");
    store.set(
        profile.save_dir.clone(),
        serde_json::to_value(&profile).expect("failed to serialize profile"),
    );
    store.save().expect("failed to save profiles store");
}

#[tauri::command]
async fn run_commit(
    save_dir: String,
    branch: String,
    message: String,
    mc_version: String,
    app: AppHandle,
) {
    let save_path = PathBuf::from(&save_dir);

    let save_name = match save_path.file_name().and_then(|n| n.to_str()) {
        Some(n) => n.to_owned(),
        None => {
            log::error!("Invalid save directory path");
            let _ = app.emit("commit-done", ());
            return;
        }
    };

    // {save_dir}/../../backups/<save-name>.git
    let git_dir = save_path
        .join("../..")
        .join("backups")
        .join(format!("{}.git", save_name))
        .canonicalize()
        .unwrap_or_else(|_| {
            save_path
                .join("../..")
                .join("backups")
                .join(format!("{}.git", save_name))
        });

    let init = !git_dir.exists();

    log::info!(
        "Commit params: save_dir={} git_dir={} init={} branch={} message={}",
        save_dir,
        git_dir.display(),
        init,
        branch,
        message
    );

    // Resolve parent commits via git rev-parse
    let parents = if init {
        if let Err(e) = std::fs::create_dir_all(&git_dir) {
            log::error!("Failed to create git_dir: {}", e);
            let _ = app.emit("commit-done", ());
            return;
        }
        let git_dir_clone = git_dir.clone();
        let init_result = tokio::task::spawn_blocking(move || {
            let cmd = superflat::utils::cmd::git_cmd(&git_dir_clone, ["init", "--bare"]);
            superflat::utils::cmd::exec(cmd, None).unwrap();
        })
        .await;
        match init_result {
            Ok(()) => {
                vec![]
            }
            Err(e) => {
                log::error!("Failed to init repository: {}", e);
                let _ = app.emit("commit-done", ());
                return;
            }
        }
    } else {
        let git_dir_clone = git_dir.clone();
        let branch_clone = branch.clone();
        let rev = tokio::task::spawn_blocking(move || {
            let cmd = superflat::utils::cmd::git_cmd(
                &git_dir_clone,
                ["rev-parse", &format!("{}^{{commit}}", branch_clone)],
            );
            superflat::utils::cmd::exec(cmd, None).unwrap()
        })
        .await;
        match rev {
            Ok(rev) => {
                let hash = rev.trim().to_owned();
                vec![hash]
            }
            Err(e) => {
                log::error!("Failed to parse {branch}: {}", e);
                let _ = app.emit("commit-done", ());
                return;
            }
        }
    };

    let r#ref = format!("refs/heads/{}", branch);
    let git_dir_for_commit = git_dir.clone();
    let result = tokio::task::spawn_blocking(move || {
        superflat::commit(
            save_path,
            git_dir_for_commit,
            parents,
            &message,
            Some(r#ref),
            &mc_version,
        )
    })
    .await;

    match result {
        Ok(()) => {
            let git_dir_clone = git_dir.clone();
            let repack_result = tokio::task::spawn_blocking(move || {
                superflat::utils::cmd::git_count_objects(&git_dir_clone)
                    .map_err(|e| e.to_string())?;
                superflat::utils::cmd::git_repack_ad(&git_dir_clone, 4095, 2)
                    .map_err(|e| e.to_string())?;
                superflat::utils::cmd::git_count_objects(&git_dir_clone)
                    .map_err(|e| e.to_string())?;
                Ok::<(), String>(())
            })
            .await;

            match repack_result {
                Ok(Ok(())) => {
                    log::info!("Done");
                }
                Ok(Err(e)) => {
                    log::error!("Commit succeeded but repack failed: {}", e);
                }
                Err(e) => {
                    log::error!("Commit succeeded but repack task failed: {}", e);
                }
            }
        }
        Err(e) => {
            log::error!("Failed to commit: {}", e);
        }
    }

    let _ = app.emit("commit-done", ());
}

#[tauri::command]
async fn run_checkout(save_dir: String, commit: String, mc_version: String, app: AppHandle) {
    let save_path = PathBuf::from(&save_dir);

    let save_name = match save_path.file_name().and_then(|n| n.to_str()) {
        Some(n) => n.to_owned(),
        None => {
            let _ = app.emit("commit-output", "Error: invalid save directory path");
            let _ = app.emit("commit-done", ());
            return;
        }
    };

    let git_dir = save_path
        .join("../..")
        .join("backups")
        .join(format!("{}.git", save_name))
        .canonicalize()
        .unwrap_or_else(|_| {
            save_path
                .join("../..")
                .join("backups")
                .join(format!("{}.git", save_name))
        });

    if save_path.exists() {
        let bak = save_path.with_extension("bak");
        let _ = app.emit(
            "commit-output",
            format!(
                "save_dir {:?} already exists, renaming to {:?}",
                save_path, bak
            ),
        );
        if let Err(e) = std::fs::rename(&save_path, &bak) {
            let _ = app.emit(
                "commit-output",
                format!("Error: failed to rename save_dir: {}", e),
            );
            let _ = app.emit("commit-done", ());
            return;
        }
    }

    let result = tokio::task::spawn_blocking(move || {
        superflat::checkout(save_path, git_dir, commit, &mc_version)
    })
    .await;

    match result {
        Ok(()) => {
            let _ = app.emit("commit-output", "Done.");
        }
        Err(e) => {
            let _ = app.emit("commit-output", format!("Error: {}", e));
        }
    }

    let _ = app.emit("commit-done", ());
}

pub fn run() {
    log::set_logger(&GUI_LOGGER).expect("failed to initialize GUI logger");
    log::set_max_level(LevelFilter::Info);
    tauri::Builder::default()
        .plugin(tauri_plugin_opener::init())
        .plugin(tauri_plugin_dialog::init())
        .plugin(tauri_plugin_store::Builder::default().build())
        .setup(|app| {
            let debug = app
                .store(STORE_FILE)
                .ok()
                .and_then(|store| store.get(KEY_DEBUG))
                .and_then(|v| v.as_bool())
                .unwrap_or(DEFAULT_DEBUG);
            GUI_LOGGER.configure(app.handle().clone(), debug);
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            pick_directory,
            get_settings,
            save_settings,
            get_profiles,
            upsert_profile,
            run_commit,
            run_checkout
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
