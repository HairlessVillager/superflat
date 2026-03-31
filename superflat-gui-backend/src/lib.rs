use std::path::PathBuf;

use tauri::{AppHandle, Emitter};
use tauri_plugin_dialog::DialogExt;
use tauri_plugin_store::StoreExt;
use tokio::io::{AsyncBufReadExt, BufReader};
use tokio::process::Command;
use tokio::sync::oneshot;

const STORE_FILE: &str = "settings.json";
const KEY_BRANCH: &str = "branch";
const DEFAULT_BRANCH: &str = "main";

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

#[tauri::command]
fn get_settings(app: AppHandle) -> String {
    let store = app.store(STORE_FILE).expect("failed to open store");
    store
        .get(KEY_BRANCH)
        .and_then(|v| v.as_str().map(str::to_owned))
        .unwrap_or_else(|| DEFAULT_BRANCH.to_owned())
}

#[tauri::command]
fn save_settings(app: AppHandle, branch: String) {
    let store = app.store(STORE_FILE).expect("failed to open store");
    store.set(KEY_BRANCH, branch);
    store.save().expect("failed to save store");
}

#[tauri::command]
async fn run_commit(save_dir: String, branch: String, message: String, app: AppHandle) {
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
        "1.21.11".to_owned(),
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

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_opener::init())
        .plugin(tauri_plugin_dialog::init())
        .plugin(tauri_plugin_store::Builder::default().build())
        .invoke_handler(tauri::generate_handler![
            pick_directory,
            get_settings,
            save_settings,
            run_commit
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
