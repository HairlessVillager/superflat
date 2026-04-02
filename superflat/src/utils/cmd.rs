use std::{
    ffi::OsStr,
    io::Write,
    path::PathBuf,
    process::{Command, Stdio},
};

use anyhow::Context;

pub fn exec(mut cmd: Command) {
    log::debug!("command (no capture): {cmd:?}");
    let out = cmd
        .output()
        .with_context(|| format!("Failed to run command {cmd:?}"))
        .unwrap();
    for line in String::from_utf8(out.stderr).unwrap().lines() {
        log::debug!("stderr: {}", line);
    }
    assert!(out.status.success());
}

pub fn exec_stdout(mut cmd: Command, stdin: Option<String>) -> String {
    log::debug!("command: {:?}", cmd);
    let out = if let Some(stdin) = stdin {
        for line in stdin.lines() {
            log::debug!("stdin: {}", line);
        }
        let mut child = cmd
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .spawn()
            .with_context(|| format!("Failed to run command {cmd:?}"))
            .unwrap();
        child
            .stdin
            .as_mut()
            .unwrap()
            .write_all(stdin.as_bytes())
            .with_context(|| format!("Failed to write stdin to command {cmd:?}"))
            .unwrap();
        child
            .wait_with_output()
            .with_context(|| format!("Failed to wait command {cmd:?}"))
            .unwrap()
    } else {
        cmd.output()
            .with_context(|| format!("Failed to read stdout from command {cmd:?}"))
            .unwrap()
    };
    assert!(
        out.status.success(),
        "stderr: {}",
        String::from_utf8(out.stderr)
            .with_context(|| format!("Failed to encoding stderr by UTF-8: {cmd:?}"))
            .unwrap()
    );
    let stdout = String::from_utf8(out.stdout)
        .with_context(|| format!("Failed to encoding stdout by UTF-8: {cmd:?}"))
        .unwrap();
    for line in stdout.lines() {
        log::debug!("stdout: {:?}", line);
    }
    stdout
}

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
    if out.status.success() {
        Ok(git_dir)
    } else {
        Err(String::from_utf8_lossy(&out.stderr).into())
    }
}

pub fn git_count_objects(git_dir: impl AsRef<OsStr>) {
    log::info!("Counting objects");
    let mut cmd = git_cmd(git_dir);
    cmd.args(["count-objects", "-vH"]);
    exec(cmd);
}

pub fn git_repack_ad(git_dir: impl AsRef<OsStr>, depth: usize, window: usize) {
    log::info!("Repacking");
    let mut cmd = git_cmd(git_dir);
    cmd.args([
        "repack",
        "--depth",
        &depth.to_string(),
        "--window",
        &window.to_string(),
        "-a",
        "-d",
    ]);
    exec(cmd);
}
