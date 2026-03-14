use std::{
    collections::{HashMap, HashSet},
    ffi::OsStr,
    path::PathBuf,
};

pub mod delta;
mod git_helper;
pub mod pack;

use git_helper::ObjID;

use crate::_repack::git_helper::{cat_files, get_commit_topo, get_commit_tree};

#[allow(dead_code)] // TODO: remove allow
fn repack(git_dir: impl AsRef<OsStr>, commit: &ObjID, basename: impl AsRef<OsStr>) {
    // git rev-list --parents --topo-order commit
    let commit_child2parents = get_commit_topo(&git_dir, commit);

    // for every commit: git ls-tree -r <commit>
    let commit2blobs = commit_child2parents
        .keys()
        .map(|commit| (commit.clone(), get_commit_tree(&git_dir, commit)))
        .collect::<HashMap<_, _>>();
    // collect all paths

    let paths = commit2blobs
        .values()
        .flat_map(|path2blob| path2blob.keys())
        .collect::<HashSet<_>>();

    // I used to thought the history is linear, but it's actually a DAG,
    // which means so many optimization suitable for linear history
    // (eg. LSM Tree, Binary Indexed Tree, etc)
    // cannot be applied to this application. :(

    // for every path: git pack-objects
    for path in paths {
        todo!()
    }
}
