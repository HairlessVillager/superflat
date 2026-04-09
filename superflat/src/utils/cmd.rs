use std::{
    ffi::OsStr,
    io::Write,
    path::PathBuf,
    process::{Command, Stdio},
};

#[cfg(windows)]
use std::os::windows::process::CommandExt;

use anyhow::{Context, Result};

#[cfg(windows)]
const CREATE_NO_WINDOW: u32 = 0x08000000;

pub fn exec(mut cmd: Command, stdin: Option<String>) -> Result<String> {
    #[cfg(windows)]
    cmd.creation_flags(CREATE_NO_WINDOW);

    log::debug!("command: {:?}", cmd);
    let out = if let Some(stdin) = stdin {
        for line in stdin.lines() {
            log::debug!("stdin: {}", line);
        }
        let mut child = cmd
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .spawn()
            .with_context(|| format!("failed to run command {cmd:?}"))?;
        child
            .stdin
            .as_mut()
            .expect("failed to get stdin handle")
            .write_all(stdin.as_bytes())
            .with_context(|| format!("failed to write stdin to command {cmd:?}"))?;
        child
            .wait_with_output()
            .with_context(|| format!("failed to wait command {cmd:?}"))?
    } else {
        cmd.output()
            .with_context(|| format!("failed to read stdout from command {cmd:?}"))?
    };
    let stderr = String::from_utf8(out.stderr)
        .with_context(|| format!("failed to encoding stderr by UTF-8"))?;
    for line in stderr.lines() {
        log::debug!("stderr: {:?}", line);
    }
    let stdout = String::from_utf8(out.stdout)
        .with_context(|| format!("failed to encoding stdout by UTF-8"))?;
    for line in stdout.lines() {
        log::debug!("stdout: {:?}", line);
    }
    anyhow::ensure!(out.status.success(), "Command status is failed");
    Ok(stdout)
}

pub fn git_cmd(
    git_dir: impl AsRef<OsStr>,
    args: impl IntoIterator<Item = impl AsRef<OsStr>>,
) -> Command {
    let mut cmd = Command::new("git");
    cmd.arg("--git-dir").arg(git_dir);
    for arg in args {
        cmd.arg(arg);
    }
    cmd
}

pub fn git_repo_exists(git_dir: &str) -> Result<PathBuf> {
    let git_dir = PathBuf::from(git_dir);
    let cmd = git_cmd(&git_dir, ["rev-parse", "--is-bare-repository"]);
    let _ = exec(cmd, None)?;
    Ok(git_dir)
}

pub fn git_count_objects(git_dir: impl AsRef<OsStr>) -> Result<()> {
    let cmd = git_cmd(git_dir, ["count-objects", "-vH"]);
    let result = exec(cmd, None)?;
    for line in result.lines() {
        log::info!("git-count-objects: {line}");
    }
    Ok(())
}

pub fn git_repack_ad(git_dir: impl AsRef<OsStr>, depth: usize, window: usize) -> Result<()> {
    log::info!("Repacking");
    let cmd = git_cmd(
        git_dir,
        [
            "repack",
            "--depth",
            &depth.to_string(),
            "--window",
            &window.to_string(),
            "-a",
            "-d",
        ],
    );
    let _ = exec(cmd, None)?;
    Ok(())
}
