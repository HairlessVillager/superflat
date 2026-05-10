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

#[derive(Debug, Clone)]
pub struct Superflat {
    save_dir: PathBuf,
    storage_dir: PathBuf,
    mc_version: Versioning,
}

impl Superflat {
    pub fn new(save_dir: PathBuf, storage_dir: PathBuf, mc_version: Versioning) -> Self {
        Self {
            save_dir,
            storage_dir,
            mc_version,
        }
    }
    pub fn flatten(&self) -> Result<()> {
        init_mc_data(&self.mc_version);
        let save = LocalFsOdb::from_dir(self.save_dir.to_owned());
        let mut repo = LocalFsOdb::from_dir(self.storage_dir.to_owned());

        for crafter in CrafterImpl::get_crafters(self.mc_version.to_owned()) {
            crafter.flatten(&save, &mut repo)?;
        }

        Ok(())
    }

    pub fn unflatten(self) -> Result<()> {
        init_mc_data(&self.mc_version);
        let mut save = LocalFsOdb::from_dir(self.save_dir.to_owned());
        let repo = LocalFsOdb::from_dir(self.storage_dir.to_owned());

        for crafter in CrafterImpl::get_crafters(self.mc_version.to_owned()) {
            crafter.unflatten(&mut save, &repo)?;
        }

        Ok(())
    }

    pub fn commit(self, parents: Vec<String>, message: &str, r#ref: Option<String>) -> Result<()> {
        init_mc_data(&self.mc_version);
        let save = LocalFsOdb::from_dir(self.save_dir.to_owned());
        let mut git = if let Some(from) = parents.first() {
            LocalGitOdb::from_commit(self.storage_dir.to_owned(), from.clone())
        } else {
            LocalGitOdb::new(self.storage_dir.to_owned())
        }?;

        for crafter in CrafterImpl::get_crafters(self.mc_version.to_owned()) {
            crafter.flatten(&save, &mut git)?;
        }

        let commit = git.commit(parents.as_slice(), message)?;

        if let Some(r#ref) = r#ref {
            let cmd = git_cmd(self.storage_dir.to_owned(), ["update-ref", &r#ref, &commit]);
            exec(cmd, None).context("failed to run update-ref")?;
            log::info!("{:?} -> {commit}", r#ref);
        } else {
            log::warn!("Dangling commit {commit}");
        }
        Ok(())
    }

    pub fn checkout(self, commit: String) -> Result<()> {
        init_mc_data(&self.mc_version);
        let mut save = LocalFsOdb::from_dir(self.save_dir.to_owned());
        let git = LocalGitOdb::from_commit(self.storage_dir.to_owned(), commit)?;

        for crafter in CrafterImpl::get_crafters(self.mc_version.to_owned()) {
            crafter.unflatten(&mut save, &git)?;
        }

        Ok(())
    }
}
