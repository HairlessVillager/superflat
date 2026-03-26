use std::{ffi::OsStr, path::PathBuf, process::Command};

pub fn git_cmd(git_dir: impl AsRef<OsStr>) -> Command {
    let mut cmd = Command::new("git");
    cmd.arg("--git-dir").arg(git_dir);
    cmd
}

pub fn git_repo_exists(git_dir: &str) -> Result<PathBuf, String> {
    let git_dir = PathBuf::from(git_dir);
    let out = git_cmd(&git_dir)
        .args(["rev-parse", "--is-bare-repository"])
        .output()
        .unwrap();
    if out.status.code().unwrap() == 0 {
        Ok(git_dir)
    } else {
        Err(String::from_utf8_lossy(&out.stderr).into())
    }
}
