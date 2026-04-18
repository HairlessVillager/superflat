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
            log::trace!("stdin: {line:?}");
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
        log::debug!("stderr: {line:?}");
    }
    let stdout = String::from_utf8(out.stdout)
        .with_context(|| format!("failed to encoding stdout by UTF-8"))?;
    for line in stdout.lines() {
        log::trace!("stdout: {line:?}");
    }
    anyhow::ensure!(out.status.success(), "command {cmd:?} status is failed");
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

pub struct RepoStats {
    pub count: u64,
    pub size_mib: f64,
    pub in_pack: u64,
    pub packs: u64,
    pub size_pack_mib: f64,
    pub prune_packable: u64,
    pub garbage: u64,
    pub size_garbage_mib: f64,
}

impl RepoStats {
    pub fn total_size_mib(&self) -> f64 {
        self.size_mib + self.size_pack_mib + self.size_garbage_mib
    }
}

pub fn git_count_objects(git_dir: impl AsRef<OsStr>) -> Result<RepoStats> {
    let cmd = git_cmd(git_dir, ["count-objects", "-v"]);
    let result = exec(cmd, None)?;

    let mut stats = RepoStats {
        count: 0,
        size_mib: 0.0,
        in_pack: 0,
        packs: 0,
        size_pack_mib: 0.0,
        prune_packable: 0,
        garbage: 0,
        size_garbage_mib: 0.0,
    };

    for line in result.lines() {
        if let Some((key, val)) = line.split_once(": ") {
            let val = val.trim();
            match key {
                "count" => stats.count = val.parse().unwrap_or(0),
                "size" => stats.size_mib = val.parse::<f64>().unwrap_or(0.0) / 1024.0,
                "in-pack" => stats.in_pack = val.parse().unwrap_or(0),
                "packs" => stats.packs = val.parse().unwrap_or(0),
                "size-pack" => stats.size_pack_mib = val.parse::<f64>().unwrap_or(0.0) / 1024.0,
                "prune-packable" => stats.prune_packable = val.parse().unwrap_or(0),
                "garbage" => stats.garbage = val.parse().unwrap_or(0),
                "size-garbage" => {
                    stats.size_garbage_mib = val.parse::<f64>().unwrap_or(0.0) / 1024.0
                }
                _ => {}
            }
        }
        log::info!("git-count-objects: {line}");
    }

    Ok(stats)
}

pub fn git_repack_ad(git_dir: impl AsRef<OsStr>, depth: usize, window: usize) -> Result<()> {
    log::info!("Repacking");
    let cmd = git_cmd(
        git_dir,
        [
            "-c",
            "pack.deltaCacheLimit=65535",
            "-c",
            "pack.deltaCacheSize=1073741824", // 1GiB
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
