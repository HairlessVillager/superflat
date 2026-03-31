use std::path::PathBuf;

use tauri::{AppHandle, Emitter};
use tauri_plugin_dialog::DialogExt;
use tauri_plugin_store::StoreExt;
use tokio::io::{AsyncBufReadExt, BufReader};
use tokio::process::Command;
use tokio::sync::oneshot;

use serde::{Deserialize, Serialize};

const STORE_FILE: &str = "settings.json";
const PROFILES_FILE: &str = "profiles.json";
const KEY_BRANCH: &str = "branch";
const DEFAULT_BRANCH: &str = "main";
const KEY_MC_VERSION: &str = "mc_version";
const DEFAULT_MC_VERSION: &str = "1.21.1";
const KEY_DEFAULT_COMMIT: &str = "default_commit";
const DEFAULT_DEFAULT_COMMIT: &str = "main@{10 minutes ago}";

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
    Settings {
        branch,
        mc_version,
        default_commit,
    }
}

#[tauri::command]
fn save_settings(app: AppHandle, branch: String, mc_version: String, default_commit: String) {
    let store = app.store(STORE_FILE).expect("failed to open store");
    store.set(KEY_BRANCH, branch);
    store.set(KEY_MC_VERSION, mc_version);
    store.set(KEY_DEFAULT_COMMIT, default_commit);
    store.save().expect("failed to save store");
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
            let _ = app.emit("commit-output", "Error: invalid save directory path");
            let _ = app.emit("commit-done", ());
            return;
        }
    };

    // {save_dir}/../../backups/<save-name>.git
    let git_dir = save_path
        .join("../..")
        .join("backups")
        .join(format!("{}.git", save_name));

    let init = !git_dir.exists();

    let git_dir_str = git_dir.to_string_lossy().into_owned();

    let _ = app.emit(
        "commit-output",
        format!(
            "save_dir={} git_dir={} init={} branch={} message={}",
            save_dir, git_dir_str, init, branch, message
        ),
    );

    let mut args = vec![
        "commit".to_owned(),
        save_dir.clone(),
        git_dir_str.clone(),
        "--branch".to_owned(),
        branch,
        "--message".to_owned(),
        message,
        "--repack".to_owned(),
        "--mc-version".to_owned(),
        mc_version,
    ];
    if init {
        args.push("--init".to_owned());
    }

    let mut child = match Command::new("superflat")
        .args(&args)
        .stdout(std::process::Stdio::piped())
        .stderr(std::process::Stdio::piped())
        .spawn()
    {
        Ok(c) => c,
        Err(e) => {
            let _ = app.emit("commit-output", format!("Error: {}", e));
            let _ = app.emit("commit-done", ());
            return;
        }
    };

    if let Some(stdout) = child.stdout.take() {
        let mut lines = BufReader::new(stdout).lines();
        while let Ok(Some(line)) = lines.next_line().await {
            let _ = app.emit("commit-output", line);
        }
    }

    if let Some(stderr) = child.stderr.take() {
        let mut lines = BufReader::new(stderr).lines();
        while let Ok(Some(line)) = lines.next_line().await {
            let _ = app.emit("commit-output", format!("stderr: {}", line));
        }
    }

    let _ = child.wait().await;
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
        .join(format!("{}.git", save_name));
    let git_dir_str = git_dir.to_string_lossy().into_owned();

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

    let args = vec![
        "checkout".to_owned(),
        save_dir.clone(),
        git_dir_str,
        "--commit".to_owned(),
        commit,
        "--mc-version".to_owned(),
        mc_version,
    ];

    let mut child = match Command::new("superflat")
        .args(&args)
        .stdout(std::process::Stdio::piped())
        .stderr(std::process::Stdio::piped())
        .spawn()
    {
        Ok(c) => c,
        Err(e) => {
            let _ = app.emit("commit-output", format!("Error: {}", e));
            let _ = app.emit("commit-done", ());
            return;
        }
    };

    if let Some(stdout) = child.stdout.take() {
        let mut lines = BufReader::new(stdout).lines();
        while let Ok(Some(line)) = lines.next_line().await {
            let _ = app.emit("commit-output", line);
        }
    }

    if let Some(stderr) = child.stderr.take() {
        let mut lines = BufReader::new(stderr).lines();
        while let Ok(Some(line)) = lines.next_line().await {
            let _ = app.emit("commit-output", format!("stderr: {}", line));
        }
    }

    let _ = child.wait().await;
    let _ = app.emit("commit-done", ());
}

pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_opener::init())
        .plugin(tauri_plugin_dialog::init())
        .plugin(tauri_plugin_store::Builder::default().build())
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
