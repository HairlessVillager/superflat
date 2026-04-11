use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Clone)]
pub struct CommitInfo {
    pub hash: String,
    pub short_hash: String,
    pub subject: String,
    pub author: String,
    pub timestamp: String,
}

/// Convert a save directory path to its corresponding bare git directory.
/// Returns `None` if the path has no file name component.
pub fn save_dir_to_git_dir(save_path: &Path) -> Option<PathBuf> {
    let save_name = save_path.file_name()?.to_str()?.to_owned();
    let path = save_path
        .join("../..")
        .join("backups")
        .join(format!("{}.git", save_name));
    Some(path.canonicalize().unwrap_or(path))
}

/// Apply the two repo config settings required by superflat.
pub fn apply_repo_config(git_dir: &Path) -> Result<(), String> {
    let cmd = superflat::utils::cmd::git_cmd(git_dir, ["config", "core.logAllRefUpdates", "true"]);
    superflat::utils::cmd::exec(cmd, None)
        .map(|_| ())
        .map_err(|e| e.to_string())?;

    let cmd = superflat::utils::cmd::git_cmd(git_dir, ["config", "gc.auto", "0"]);
    superflat::utils::cmd::exec(cmd, None)
        .map(|_| ())
        .map_err(|e| e.to_string())
}

/// Initialize a new bare git repo and apply repo config.
pub fn git_init_bare(git_dir: &Path) -> Result<(), String> {
    let cmd = superflat::utils::cmd::git_cmd(git_dir, ["init", "--bare"]);
    superflat::utils::cmd::exec(cmd, None)
        .map(|_| ())
        .map_err(|e| e.to_string())?;
    apply_repo_config(git_dir)
}

#[tauri::command]
pub fn check_repo_exists(save_dir: String) -> bool {
    if save_dir.is_empty() {
        return false;
    }
    let save_path = PathBuf::from(&save_dir);
    save_dir_to_git_dir(&save_path)
        .map(|p| p.exists())
        .unwrap_or(false)
}

#[tauri::command]
pub fn get_commits(save_dir: String) -> Result<Vec<CommitInfo>, String> {
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
        "--date=format:%Y-%m-%d %H:%M:%S",
        "--format=%H\x1f%h\x1f%s\x1f%aN\x1f%ad",
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
