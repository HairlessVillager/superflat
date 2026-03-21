use std::path::PathBuf;

use crate::{
    crafter::{ChunkRegionCrafter, Crafter, GzipNbtCrafter, OtherRegionCrafter, RawCrafter},
    odb::{LocalFsOdb, LocalGitOdb},
};

mod crafter;
mod odb;
mod utils;

pub async fn flatten(save_dir: PathBuf, repo_dir: PathBuf) {
    let save = LocalFsOdb::from_dir(save_dir);
    let mut repo = LocalFsOdb::from_dir(repo_dir);

    RawCrafter.flatten(&save, &mut repo).await;
    GzipNbtCrafter.flatten(&save, &mut repo).await;
    ChunkRegionCrafter.flatten(&save, &mut repo).await;
    OtherRegionCrafter.flatten(&save, &mut repo).await;
}

pub async fn unflatten(save_dir: PathBuf, repo_dir: PathBuf) {
    let mut save = LocalFsOdb::from_dir(save_dir);
    let repo = LocalFsOdb::from_dir(repo_dir);

    RawCrafter.unflatten(&mut save, &repo).await;
    GzipNbtCrafter.unflatten(&mut save, &repo).await;
    ChunkRegionCrafter.unflatten(&mut save, &repo).await;
    OtherRegionCrafter.unflatten(&mut save, &repo).await;
}

pub async fn commit(save_dir: PathBuf, git_dir: PathBuf, parents: Vec<String>, message: &str) {
    let save = LocalFsOdb::from_dir(save_dir);
    let mut git = if let Some(from) = parents.first() {
        LocalGitOdb::from_commit(git_dir, from.clone())
    } else {
        LocalGitOdb::new(git_dir)
    };

    RawCrafter.flatten(&save, &mut git).await;
    GzipNbtCrafter.flatten(&save, &mut git).await;
    ChunkRegionCrafter.flatten(&save, &mut git).await;
    OtherRegionCrafter.flatten(&save, &mut git).await;

    git.commit(parents.as_slice(), message).await;
}

pub async fn checkout(save_dir: PathBuf, git_dir: PathBuf, commit: String) {
    let mut save = LocalFsOdb::from_dir(save_dir);
    let git = LocalGitOdb::from_commit(git_dir, commit);

    RawCrafter.unflatten(&mut save, &git).await;
    GzipNbtCrafter.unflatten(&mut save, &git).await;
    ChunkRegionCrafter.unflatten(&mut save, &git).await;
    OtherRegionCrafter.unflatten(&mut save, &git).await;
}
