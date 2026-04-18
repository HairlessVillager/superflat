use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};

use superflat::utils::cmd::{exec as sf_exec, git_cmd};

/// Canonicalize a path and strip the `\\?\` verbatim prefix that
/// [`std::fs::canonicalize`] adds on Windows.  Git for Windows does not
/// understand the extended-length path syntax and misinterprets it as a
/// POSIX-style absolute path (e.g. `C:\foo` becomes `/C:/foo`).
///
/// Falls back to the original path if canonicalization fails (e.g. the
/// path does not yet exist on disk).
pub fn canonicalize_portable(path: PathBuf) -> PathBuf {
    match path.canonicalize() {
        Ok(canonical) => strip_verbatim_prefix(canonical),
        Err(_) => path,
    }
}

/// Remove the `\\?\` or `\\?\UNC\` prefix that Windows adds to verbatim
/// paths returned by [`std::fs::canonicalize`].
fn strip_verbatim_prefix(path: PathBuf) -> PathBuf {
    #[cfg(windows)]
    {
        use std::path::Component;
        // Verbatim paths start with a Prefix component like `\\?\C:` or
        // `\\?\UNC\server\share`.  Collect all components after skipping
        // the verbatim prefix and rebuild a plain path.
        let mut components = path.components();
        if let Some(Component::Prefix(prefix)) = components.next() {
            use std::path::Prefix;
            match prefix.kind() {
                Prefix::VerbatimDisk(drive) => {
                    // \\?\C:\foo  →  C:\foo
                    let root: PathBuf = format!("{}:\\", drive as char).into();
                    return components.fold(root, |acc, c| acc.join(c));
                }
                Prefix::VerbatimUNC(host, share) => {
                    // \\?\UNC\host\share\foo  →  \\host\share\foo
                    let root: PathBuf = format!(
                        "\\\\{}\\{}",
                        host.to_string_lossy(),
                        share.to_string_lossy()
                    )
                    .into();
                    return components.fold(root, |acc, c| acc.join(c));
                }
                Prefix::Verbatim(_) => {
                    // \\?\foo  →  just skip the verbatim marker
                    return components.collect();
                }
                _ => {}
            }
        }
        path
    }
    #[cfg(not(windows))]
    {
        path
    }
}

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
    Some(canonicalize_portable(path))
}

/// Apply the two repo config settings required by superflat.
pub fn apply_repo_config(git_dir: &Path) -> Result<(), String> {
    let cmd = git_cmd(git_dir, ["config", "core.logAllRefUpdates", "true"]);
    sf_exec(cmd, None).map(|_| ()).map_err(|e| e.to_string())?;

    let cmd = git_cmd(git_dir, ["config", "gc.auto", "0"]);
    sf_exec(cmd, None).map(|_| ()).map_err(|e| e.to_string())
}

/// Initialize a new bare git repo and apply repo config.
pub fn git_init_bare(git_dir: &Path) -> Result<(), String> {
    let cmd = git_cmd(git_dir, ["init", "--bare"]);
    sf_exec(cmd, None).map(|_| ()).map_err(|e| e.to_string())?;
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

    let cmd = git_cmd(
        &git_dir,
        [
            "log",
            "--all",
            "--date=format:%Y-%m-%d %H:%M:%S",
            "--format=%H\x1f%h\x1f%s\x1f%aN\x1f%ad",
            "-n",
            "50",
        ],
    );
    let stdout = sf_exec(cmd, None).map_err(|e| format!("git log failed: {e}"))?;

    Ok(stdout
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
