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

pub fn git_count_objects(git_dir: impl AsRef<OsStr>) {
    log::info!("Counting objects");
    let count_out = git_cmd(git_dir)
        .args(["count-objects", "-vH"])
        .output()
        .unwrap()
        .stdout;
    for line in String::from_utf8(count_out).unwrap().lines() {
        log::info!("git-count-objects: {line}")
    }
}

pub fn git_repack_all(git_dir: impl AsRef<OsStr>, depth: usize, window: usize) {
    log::info!("Repacking");
    let _repack_out = git_cmd(git_dir)
        .args([
            "repack",
            "--depth",
            &depth.to_string(),
            "--window",
            &window.to_string(),
            "-a",
            "-d",
        ])
        .output() // TODO: use .status()
        .unwrap();
}
