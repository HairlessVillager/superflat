use std::{
    collections::HashSet,
    fs, io,
    path::Path,
    path::PathBuf,
    sync::{
        Mutex,
        atomic::{AtomicU8, Ordering},
    },
};

use chrono::Local;
use log::{Level, LevelFilter, Log, Metadata, Record};
use tauri::{AppHandle, Emitter, Manager, path::BaseDirectory};
use tauri_plugin_dialog::DialogExt;
use tokio::sync::oneshot;

use serde::{Deserialize, Serialize};

const PROFILES_FILE: &str = "profiles.json";
const DEFAULT_DEBUG: bool = false;
const EVENT_OUTPUT: &str = "commit-output";
const EVENT_DONE: &str = "commit-done";

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

#[derive(Serialize, Deserialize, Clone)]
struct Profile {
    save_dir: String,
    mc_version: String,
    branch: String,
    #[serde(default)]
    remote_url: String,
}

fn normalize_profiles(profiles: Vec<Profile>) -> Vec<Profile> {
    let mut seen_save_dirs = HashSet::with_capacity(profiles.len());
    let mut normalized = Vec::with_capacity(profiles.len());

    for profile in profiles {
        if profile.save_dir.trim().is_empty() {
            continue;
        }
        if seen_save_dirs.insert(profile.save_dir.clone()) {
            normalized.push(profile);
        }
    }

    normalized.sort_by(|a, b| a.save_dir.cmp(&b.save_dir));
    normalized
}

fn app_data_file(app: &AppHandle, file_name: &str) -> io::Result<PathBuf> {
    app.path()
        .resolve(file_name, BaseDirectory::AppData)
        .map_err(io::Error::other)
}

fn read_profiles_file(path: &Path) -> io::Result<Vec<Profile>> {
    match fs::read(path) {
        Ok(bytes) => serde_json::from_slice::<Vec<Profile>>(&bytes)
            .map(normalize_profiles)
            .map_err(io::Error::other),
        Err(err) if err.kind() == io::ErrorKind::NotFound => Ok(Vec::new()),
        Err(err) => Err(err),
    }
}

fn write_profiles_file(path: &Path, profiles: &[Profile]) -> io::Result<()> {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)?;
    }
    let normalized = normalize_profiles(profiles.to_vec());
    let bytes = serde_json::to_vec_pretty(&normalized).map_err(io::Error::other)?;
    fs::write(path, bytes)
}

#[tauri::command]
fn get_profiles(app: AppHandle) -> Result<Vec<Profile>, String> {
    let path = app_data_file(&app, PROFILES_FILE).map_err(|e| e.to_string())?;
    read_profiles_file(&path).map_err(|e| e.to_string())
}

#[tauri::command]
fn upsert_profile(app: AppHandle, profile: Profile) -> Result<(), String> {
    let path = app_data_file(&app, PROFILES_FILE).map_err(|e| e.to_string())?;
    let mut profiles = read_profiles_file(&path).map_err(|e| e.to_string())?;

    if let Some(existing) = profiles.iter_mut().find(|p| p.save_dir == profile.save_dir) {
        *existing = profile;
    } else {
        profiles.push(profile);
    }

    write_profiles_file(&path, &profiles).map_err(|e| e.to_string())
}

#[tauri::command]
fn delete_profile(app: AppHandle, save_dir: String) -> Result<(), String> {
    let path = app_data_file(&app, PROFILES_FILE).map_err(|e| e.to_string())?;
    let profiles = read_profiles_file(&path).map_err(|e| e.to_string())?;
    let profiles: Vec<Profile> = profiles.into_iter().filter(|p| p.save_dir != save_dir).collect();
    write_profiles_file(&path, &profiles).map_err(|e| e.to_string())
}

#[tauri::command]
fn set_debug_logging(app: AppHandle, debug: bool) {
    GUI_LOGGER.configure(app, debug);
}

/// Apply repo config settings shared by all superflat git repos.
fn apply_repo_config(git_dir: &Path) -> Result<(), String> {
    let cmd = superflat::utils::cmd::git_cmd(
        git_dir,
        ["config", "core.logAllRefUpdates", "true"],
    );
    superflat::utils::cmd::exec(cmd, None).map(|_| ()).map_err(|e| e.to_string())?;
    let cmd = superflat::utils::cmd::git_cmd(git_dir, ["config", "gc.auto", "0"]);
    superflat::utils::cmd::exec(cmd, None).map(|_| ()).map_err(|e| e.to_string())
}

/// Initialize a new bare git repo and apply repo config.
fn git_init_bare(git_dir: &Path) -> Result<(), String> {
    let cmd = superflat::utils::cmd::git_cmd(git_dir, ["init", "--bare"]);
    superflat::utils::cmd::exec(cmd, None).map(|_| ()).map_err(|e| e.to_string())?;
    apply_repo_config(git_dir)
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
            let _ = app.emit(EVENT_DONE, ());
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
            let _ = app.emit(EVENT_DONE, ());
            return;
        }
        let git_dir_clone = git_dir.clone();
        let init_result = tokio::task::spawn_blocking(move || git_init_bare(&git_dir_clone)).await;
        match init_result {
            Ok(Ok(())) => vec![],
            Ok(Err(e)) => {
                log::error!("Failed to init repository: {}", e);
                let _ = app.emit(EVENT_DONE, ());
                return;
            }
            Err(e) => {
                log::error!("Failed to init repository (task panic): {}", e);
                let _ = app.emit(EVENT_DONE, ());
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
            superflat::utils::cmd::exec(cmd, None).map_err(|e| e.to_string())
        })
        .await;
        match rev {
            Ok(Ok(rev)) => {
                let hash = rev.trim().to_owned();
                vec![hash]
            }
            Ok(Err(e)) => {
                log::error!("Failed to parse {branch}: {}", e);
                let _ = app.emit(EVENT_DONE, ());
                return;
            }
            Err(e) => {
                log::error!("Failed to parse {branch} (task panic): {}", e);
                let _ = app.emit(EVENT_DONE, ());
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

    let _ = app.emit(EVENT_DONE, ());
}

#[tauri::command]
async fn run_checkout(save_dir: String, commit: String, mc_version: String, app: AppHandle) {
    let save_path = PathBuf::from(&save_dir);

    let save_name = match save_path.file_name().and_then(|n| n.to_str()) {
        Some(n) => n.to_owned(),
        None => {
            log::error!("Invalid save directory path");
            let _ = app.emit(EVENT_DONE, ());
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
        if bak.exists() {
            log::error!("Backup {bak:?} already exists, aborting checkout");
            let _ = app.emit(EVENT_DONE, ());
            return;
        }
        log::info!("save_dir {save_path:?} already exists, renaming to {bak:?}",);
        if let Err(e) = std::fs::rename(&save_path, &bak) {
            log::error!("Failed to rename save_dir: {e}");
            let _ = app.emit(EVENT_DONE, ());
            return;
        }
    }

    let result = tokio::task::spawn_blocking(move || {
        superflat::checkout(save_path, git_dir, commit, &mc_version)
    })
    .await;

    match result {
        Ok(()) => {
            log::info!("Done");
        }
        Err(e) => {
            log::error!("Error: {e}");
        }
    }

    let _ = app.emit(EVENT_DONE, ());
}

fn save_dir_to_git_dir(save_path: &Path) -> Option<PathBuf> {
    let save_name = save_path.file_name()?.to_str()?.to_owned();
    let path = save_path
        .join("../..")
        .join("backups")
        .join(format!("{}.git", save_name));
    Some(path.canonicalize().unwrap_or(path))
}

#[tauri::command]
fn check_repo_exists(save_dir: String) -> bool {
    if save_dir.is_empty() {
        return false;
    }
    let save_path = PathBuf::from(&save_dir);
    save_dir_to_git_dir(&save_path)
        .map(|p| p.exists())
        .unwrap_or(false)
}

#[tauri::command]
async fn run_clone(save_dir: String, url: String, app: AppHandle) {
    let save_path = PathBuf::from(&save_dir);

    let git_dir = match save_dir_to_git_dir(&save_path) {
        Some(d) => d,
        None => {
            log::error!("Invalid save directory path");
            let _ = app.emit(EVENT_DONE, ());
            return;
        }
    };

    log::info!("Cloning {} into {}", url, git_dir.display());

    let result = tokio::task::spawn_blocking(move || {
        // Ensure the backups/ parent directory exists
        if let Some(parent) = git_dir.parent() {
            std::fs::create_dir_all(parent)
                .map_err(|e| format!("Failed to create parent dir: {e}"))?;
        }

        let mut cmd = std::process::Command::new("git");
        cmd.args(["clone", "--bare", &url]).arg(&git_dir);

        #[cfg(windows)]
        {
            use std::os::windows::process::CommandExt;
            cmd.creation_flags(0x08000000);
        }

        let out = cmd.output().map_err(|e| e.to_string())?;

        // git clone writes its progress to stderr
        let stderr = String::from_utf8_lossy(&out.stderr);
        for line in stderr.lines() {
            log::info!("{}", line);
        }
        let stdout = String::from_utf8_lossy(&out.stdout);
        for line in stdout.lines() {
            log::info!("{}", line);
        }

        if !out.status.success() {
            return Err(format!("git clone exited with {}", out.status));
        }

        apply_repo_config(&git_dir)
    })
    .await;

    match result {
        Ok(Ok(())) => log::info!("Clone done"),
        Ok(Err(e)) => log::error!("Clone failed: {}", e),
        Err(e) => log::error!("Clone task failed: {}", e),
    }

    let _ = app.emit(EVENT_DONE, ());
}

#[tauri::command]
async fn run_pull(save_dir: String, url: String, app: AppHandle) {
    let save_path = PathBuf::from(&save_dir);

    let git_dir = match save_dir_to_git_dir(&save_path) {
        Some(d) => d,
        None => {
            log::error!("Invalid save directory path");
            let _ = app.emit(EVENT_DONE, ());
            return;
        }
    };

    log::info!("Pulling from {}", url);

    let result = tokio::task::spawn_blocking(move || {
        let cmd = superflat::utils::cmd::git_cmd(
            &git_dir,
            ["fetch", &url, "refs/heads/*:refs/heads/*"],
        );
        superflat::utils::cmd::exec(cmd, None).map_err(|e| e.to_string())
    })
    .await;

    match result {
        Ok(Ok(out)) => {
            for line in out.lines() {
                log::info!("{}", line);
            }
            log::info!("Pull done");
        }
        Ok(Err(e)) => log::error!("Pull failed: {}", e),
        Err(e) => log::error!("Pull task failed: {}", e),
    }

    let _ = app.emit(EVENT_DONE, ());
}

#[tauri::command]
async fn run_push(save_dir: String, url: String, app: AppHandle) {
    let save_path = PathBuf::from(&save_dir);

    let git_dir = match save_dir_to_git_dir(&save_path) {
        Some(d) => d,
        None => {
            log::error!("Invalid save directory path");
            let _ = app.emit(EVENT_DONE, ());
            return;
        }
    };

    log::info!("Pushing to {}", url);

    let result = tokio::task::spawn_blocking(move || {
        let cmd = superflat::utils::cmd::git_cmd(&git_dir, ["push", &url, "--all"]);
        superflat::utils::cmd::exec(cmd, None).map_err(|e| e.to_string())
    })
    .await;

    match result {
        Ok(Ok(out)) => {
            for line in out.lines() {
                log::info!("{}", line);
            }
            log::info!("Push done");
        }
        Ok(Err(e)) => log::error!("Push failed: {}", e),
        Err(e) => log::error!("Push task failed: {}", e),
    }

    let _ = app.emit(EVENT_DONE, ());
}

#[derive(Serialize, Deserialize, Clone)]
struct CommitInfo {
    hash: String,
    short_hash: String,
    subject: String,
    author: String,
    timestamp: String,
}

#[tauri::command]
fn get_commits(save_dir: String) -> Result<Vec<CommitInfo>, String> {
    if save_dir.is_empty() {
        return Ok(vec![]);
    }
    let save_path = PathBuf::from(&save_dir);
    let git_dir = match save_dir_to_git_dir(&save_path) {
        Some(d) => d,
        None => return Ok(vec![]),
    };
    if !git_dir.exists() {
        return Ok(vec![]);
    }

    let mut cmd = std::process::Command::new("git");
    cmd.arg("--git-dir").arg(&git_dir);
    cmd.args([
        "log",
        "--all",
        "--format=%H\x1f%h\x1f%s\x1f%aN\x1f%ai",
        "-n",
        "50",
    ]);

    #[cfg(windows)]
    {
        use std::os::windows::process::CommandExt;
        cmd.creation_flags(0x08000000);
    }

    let out = cmd.output().map_err(|e| format!("git log failed: {e}"))?;

    if !out.status.success() {
        let stderr = String::from_utf8_lossy(&out.stderr);
        return Err(format!("git log exited with {}: {stderr}", out.status));
    }

    Ok(String::from_utf8_lossy(&out.stdout)
        .lines()
        .filter_map(|line| {
            let parts: Vec<&str> = line.splitn(5, '\x1f').collect();
            if parts.len() == 5 {
                Some(CommitInfo {
                    hash: parts[0].to_owned(),
                    short_hash: parts[1].to_owned(),
                    subject: parts[2].to_owned(),
                    author: parts[3].to_owned(),
                    timestamp: parts[4].to_owned(),
                })
            } else {
                None
            }
        })
        .collect())
}


pub fn run() {
    log::set_logger(&GUI_LOGGER).expect("failed to initialize GUI logger");
    log::set_max_level(LevelFilter::Info);
    tauri::Builder::default()
        .plugin(tauri_plugin_opener::init())
        .plugin(tauri_plugin_dialog::init())
        .setup(|app| {
            GUI_LOGGER.configure(app.handle().clone(), DEFAULT_DEBUG);
            if let Ok(settings_path) = app_data_file(app.handle(), "settings.json") {
                match fs::remove_file(settings_path) {
                    Ok(()) => {}
                    Err(err) if err.kind() == io::ErrorKind::NotFound => {}
                    Err(err) => log::warn!("Failed to remove legacy settings.json: {}", err),
                }
            }
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            pick_directory,
            get_profiles,
            upsert_profile,
            delete_profile,
            set_debug_logging,
            run_commit,
            run_checkout,
            check_repo_exists,
            run_clone,
            run_pull,
            run_push,
            get_commits
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}

#[cfg(test)]
mod tests {
    use super::{Profile, normalize_profiles};

    #[test]
    fn normalize_profiles_filters_empty_save_dirs_and_keeps_first_duplicate() {
        let normalized = normalize_profiles(vec![
            Profile {
                save_dir: "/b".into(),
                mc_version: "1.20.1".into(),
                branch: "main".into(),
                remote_url: String::new(),
            },
            Profile {
                save_dir: "".into(),
                mc_version: "1.21.1".into(),
                branch: "empty".into(),
                remote_url: String::new(),
            },
            Profile {
                save_dir: "   ".into(),
                mc_version: "1.21.2".into(),
                branch: "blank".into(),
                remote_url: String::new(),
            },
            Profile {
                save_dir: "/a".into(),
                mc_version: "1.19.4".into(),
                branch: "stable".into(),
                remote_url: String::new(),
            },
            Profile {
                save_dir: "/b".into(),
                mc_version: "1.21.4".into(),
                branch: "newer".into(),
                remote_url: String::new(),
            },
        ]);

        assert_eq!(normalized.len(), 2);
        assert_eq!(normalized[0].save_dir, "/a");
        assert_eq!(normalized[0].branch, "stable");
        assert_eq!(normalized[1].save_dir, "/b");
        assert_eq!(normalized[1].branch, "main");
    }
}
