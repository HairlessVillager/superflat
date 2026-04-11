use std::path::PathBuf;

use tauri::{AppHandle, Emitter};
use tokio::sync::oneshot;
use tauri_plugin_dialog::DialogExt;

use crate::EVENT_DONE;
use crate::git_ops::{apply_repo_config, git_init_bare, save_dir_to_git_dir};

#[tauri::command]
pub async fn pick_directory(app: AppHandle) -> Option<String> {
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

/// Resolve the save name and git dir from a save_dir string, emitting EVENT_DONE on error.
fn resolve_paths(
    save_dir: &str,
    app: &AppHandle,
) -> Option<(PathBuf, PathBuf)> {
    let save_path = PathBuf::from(save_dir);
    if save_dir.trim().is_empty() || !save_path.is_absolute() {
        log::error!("save_dir must be a non-empty absolute path, got: {:?}", save_dir);
        let _ = app.emit(EVENT_DONE, ());
        return None;
    }
    let save_name = match save_path.file_name().and_then(|n| n.to_str()) {
        Some(n) => n.to_owned(),
        None => {
            log::error!("Invalid save directory path");
            let _ = app.emit(EVENT_DONE, ());
            return None;
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
    Some((save_path, git_dir))
}

#[tauri::command]
pub async fn run_commit(
    save_dir: String,
    branch: String,
    message: String,
    mc_version: String,
    app: AppHandle,
) {
    let (save_path, git_dir) = match resolve_paths(&save_dir, &app) {
        Some(p) => p,
        None => return,
    };

    let init = !git_dir.exists();
    log::info!(
        "Commit params: save_dir={} git_dir={} init={} branch={} message={}",
        save_dir, git_dir.display(), init, branch, message
    );

    let parents = if init {
        if let Err(e) = std::fs::create_dir_all(&git_dir) {
            log::error!("Failed to create git_dir: {}", e);
            let _ = app.emit(EVENT_DONE, ());
            return;
        }
        let git_dir_clone = git_dir.clone();
        match tokio::task::spawn_blocking(move || git_init_bare(&git_dir_clone)).await {
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
        match resolve_branch_parent(&git_dir, &branch).await {
            Ok(hash) => vec![hash],
            Err(e) => {
                log::error!("{}", e);
                let _ = app.emit(EVENT_DONE, ());
                return;
            }
        }
    };

    do_commit_and_repack(save_path, git_dir, branch, message, mc_version, parents).await;
    let _ = app.emit(EVENT_DONE, ());
}

async fn resolve_branch_parent(git_dir: &PathBuf, branch: &str) -> Result<String, String> {
    let git_dir_clone = git_dir.clone();
    let branch_clone = branch.to_owned();
    let rev = tokio::task::spawn_blocking(move || {
        let cmd = superflat::utils::cmd::git_cmd(
            &git_dir_clone,
            ["rev-parse", &format!("{}^{{commit}}", branch_clone)],
        );
        superflat::utils::cmd::exec(cmd, None).map_err(|e| e.to_string())
    })
    .await;
    match rev {
        Ok(Ok(rev)) => Ok(rev.trim().to_owned()),
        Ok(Err(e)) => Err(format!("Failed to parse {branch}: {e}")),
        Err(e) => Err(format!("Failed to parse {branch} (task panic): {e}")),
    }
}

async fn do_commit_and_repack(
    save_path: PathBuf,
    git_dir: PathBuf,
    branch: String,
    message: String,
    mc_version: String,
    parents: Vec<String>,
) {
    let r#ref = format!("refs/heads/{}", branch);
    let git_dir_for_commit = git_dir.clone();
    let result = tokio::task::spawn_blocking(move || {
        superflat::commit(save_path, git_dir_for_commit, parents, &message, Some(r#ref), &mc_version)
    })
    .await;

    match result {
        Ok(()) => {
            let git_dir_clone = git_dir.clone();
            let repack = tokio::task::spawn_blocking(move || {
                superflat::utils::cmd::git_count_objects(&git_dir_clone)
                    .map_err(|e| e.to_string())?;
                superflat::utils::cmd::git_repack_ad(&git_dir_clone, 4095, 2)
                    .map_err(|e| e.to_string())?;
                superflat::utils::cmd::git_count_objects(&git_dir_clone)
                    .map_err(|e| e.to_string())?;
                Ok::<(), String>(())
            })
            .await;
            match repack {
                Ok(Ok(())) => log::info!("Done"),
                Ok(Err(e)) => log::error!("Commit succeeded but repack failed: {}", e),
                Err(e) => log::error!("Commit succeeded but repack task failed: {}", e),
            }
        }
        Err(e) => log::error!("Failed to commit: {}", e),
    }
}

#[tauri::command]
pub async fn run_checkout(save_dir: String, commit: String, mc_version: String, app: AppHandle) {
    let (save_path, git_dir) = match resolve_paths(&save_dir, &app) {
        Some(p) => p,
        None => return,
    };

    if save_path.exists() {
        let bak = save_path.with_extension("bak");
        if bak.exists() {
            log::error!("Backup {bak:?} already exists, aborting checkout");
            let _ = app.emit(EVENT_DONE, ());
            return;
        }
        log::info!("save_dir {save_path:?} already exists, renaming to {bak:?}");
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
        Ok(()) => log::info!("Done"),
        Err(e) => log::error!("Error: {e}"),
    }
    let _ = app.emit(EVENT_DONE, ());
}

#[tauri::command]
pub async fn run_clone(save_dir: String, url: String, app: AppHandle) {
    let save_path = PathBuf::from(&save_dir);
    if save_dir.trim().is_empty() || !save_path.is_absolute() {
        log::error!("save_dir must be a non-empty absolute path, got: {:?}", save_dir);
        let _ = app.emit(EVENT_DONE, ());
        return;
    }
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
        for line in String::from_utf8_lossy(&out.stderr).lines() {
            log::info!("{}", line);
        }
        for line in String::from_utf8_lossy(&out.stdout).lines() {
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
pub async fn run_pull(save_dir: String, url: String, app: AppHandle) {
    let save_path = PathBuf::from(&save_dir);
    if save_dir.trim().is_empty() || !save_path.is_absolute() {
        log::error!("save_dir must be a non-empty absolute path, got: {:?}", save_dir);
        let _ = app.emit(EVENT_DONE, ());
        return;
    }
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
            for line in out.lines() { log::info!("{}", line); }
            log::info!("Pull done");
        }
        Ok(Err(e)) => log::error!("Pull failed: {}", e),
        Err(e) => log::error!("Pull task failed: {}", e),
    }
    let _ = app.emit(EVENT_DONE, ());
}

#[tauri::command]
pub async fn run_push(save_dir: String, url: String, app: AppHandle) {
    let save_path = PathBuf::from(&save_dir);
    if save_dir.trim().is_empty() || !save_path.is_absolute() {
        log::error!("save_dir must be a non-empty absolute path, got: {:?}", save_dir);
        let _ = app.emit(EVENT_DONE, ());
        return;
    }
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
            for line in out.lines() { log::info!("{}", line); }
            log::info!("Push done");
        }
        Ok(Err(e)) => log::error!("Push failed: {}", e),
        Err(e) => log::error!("Push task failed: {}", e),
    }
    let _ = app.emit(EVENT_DONE, ());
}
