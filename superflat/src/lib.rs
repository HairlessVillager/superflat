use std::path::PathBuf;

use anyhow::{Context, Result};
use versions::Versioning;

use crate::{
    crafter::{Crafter, CrafterImpl},
    odb::{LocalFsOdb, LocalGitOdb},
    utils::{
        cmd::{exec, git_cmd},
        mc_data::init_mc_data,
    },
};

mod crafter;
pub mod odb;
pub mod utils;

pub fn flatten(save_dir: PathBuf, repo_dir: PathBuf, mc_version: Versioning) -> Result<()> {
    init_mc_data(&mc_version);
    let save = LocalFsOdb::from_dir(save_dir);
    let mut repo = LocalFsOdb::from_dir(repo_dir);

    for crafter in CrafterImpl::get_crafters(mc_version) {
        crafter.flatten(&save, &mut repo)?;
    }

    Ok(())
}

pub fn unflatten(save_dir: PathBuf, repo_dir: PathBuf, mc_version: Versioning) -> Result<()> {
    init_mc_data(&mc_version);
    let mut save = LocalFsOdb::from_dir(save_dir);
    let repo = LocalFsOdb::from_dir(repo_dir);

    for crafter in CrafterImpl::get_crafters(mc_version) {
        crafter.unflatten(&mut save, &repo)?;
    }

    Ok(())
}

pub fn commit(
    save_dir: PathBuf,
    git_dir: PathBuf,
    parents: Vec<String>,
    message: &str,
    r#ref: Option<String>,
    mc_version: Versioning,
) -> Result<()> {
    init_mc_data(&mc_version);
    let save = LocalFsOdb::from_dir(save_dir);
    let mut git = if let Some(from) = parents.first() {
        LocalGitOdb::from_commit(git_dir.to_owned(), from.clone())
    } else {
        LocalGitOdb::new(git_dir.to_owned())
    }?;

    for crafter in CrafterImpl::get_crafters(mc_version) {
        crafter.flatten(&save, &mut git)?;
    }

    let commit = git.commit(parents.as_slice(), message)?;

    if let Some(r#ref) = r#ref {
        let cmd = git_cmd(git_dir, ["update-ref", &r#ref, &commit]);
        exec(cmd, None).context("failed to run update-ref")?;
        log::info!("{:?} -> {commit}", r#ref);
    } else {
        log::warn!("Dangling commit {commit}");
    }
    Ok(())
}

pub fn checkout(
    save_dir: PathBuf,
    git_dir: PathBuf,
    commit: String,
    mc_version: Versioning,
) -> Result<()> {
    init_mc_data(&mc_version);
    let mut save = LocalFsOdb::from_dir(save_dir);
    let git = LocalGitOdb::from_commit(git_dir, commit)?;

    for crafter in CrafterImpl::get_crafters(mc_version) {
        crafter.unflatten(&mut save, &git)?;
    }

    Ok(())
}
