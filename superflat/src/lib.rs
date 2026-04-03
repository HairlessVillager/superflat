use std::path::PathBuf;

use crate::{
    crafter::{ChunkRegionCrafter, Crafter, GzipNbtCrafter, OtherRegionCrafter, RawCrafter},
    odb::{LocalFsOdb, LocalGitOdb},
    utils::{
        cmd::{exec, git_cmd},
        mc_data::init_mc_data,
    },
};

mod crafter;
pub mod odb;
pub mod utils;

pub fn flatten(save_dir: PathBuf, repo_dir: PathBuf, mc_version: &str) {
    init_mc_data(mc_version);
    let save = LocalFsOdb::from_dir(save_dir);
    let mut repo = LocalFsOdb::from_dir(repo_dir);

    RawCrafter.flatten(&save, &mut repo);
    GzipNbtCrafter.flatten(&save, &mut repo);
    ChunkRegionCrafter.flatten(&save, &mut repo);
    OtherRegionCrafter.flatten(&save, &mut repo);
}

pub fn unflatten(save_dir: PathBuf, repo_dir: PathBuf, mc_version: &str) {
    init_mc_data(mc_version);
    let mut save = LocalFsOdb::from_dir(save_dir);
    let repo = LocalFsOdb::from_dir(repo_dir);

    RawCrafter.unflatten(&mut save, &repo);
    GzipNbtCrafter.unflatten(&mut save, &repo);
    ChunkRegionCrafter.unflatten(&mut save, &repo);
    OtherRegionCrafter.unflatten(&mut save, &repo);
}

pub fn commit(
    save_dir: PathBuf,
    git_dir: PathBuf,
    parents: Vec<String>,
    message: &str,
    r#ref: Option<String>,
    mc_version: &str,
) {
    init_mc_data(mc_version);
    let save = LocalFsOdb::from_dir(save_dir);
    let mut git = if let Some(from) = parents.first() {
        LocalGitOdb::from_commit(git_dir.to_owned(), from.clone())
    } else {
        LocalGitOdb::new(git_dir.to_owned())
    };

    RawCrafter.flatten(&save, &mut git);
    GzipNbtCrafter.flatten(&save, &mut git);
    ChunkRegionCrafter.flatten(&save, &mut git);
    OtherRegionCrafter.flatten(&save, &mut git);

    let commit = git.commit(parents.as_slice(), message);

    if let Some(r#ref) = r#ref {
        let cmd = git_cmd(git_dir, ["update-ref", &r#ref, &commit]);
        let _ = exec(cmd, None).unwrap();
        log::info!("{:?} -> {commit}", r#ref);
    } else {
        log::warn!("Dangling commit {commit}");
    }
}

pub fn checkout(save_dir: PathBuf, git_dir: PathBuf, commit: String, mc_version: &str) {
    init_mc_data(mc_version);
    let mut save = LocalFsOdb::from_dir(save_dir);
    let git = LocalGitOdb::from_commit(git_dir, commit);

    RawCrafter.unflatten(&mut save, &git);
    GzipNbtCrafter.unflatten(&mut save, &git);
    ChunkRegionCrafter.unflatten(&mut save, &git);
    OtherRegionCrafter.unflatten(&mut save, &git);
}
