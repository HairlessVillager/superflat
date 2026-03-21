use std::path::PathBuf;

use clap::Parser;
use superflat_rs::{checkout, commit, flatten, unflatten};

/// Superflat - A bridge between Git and Minecraft save
#[derive(Parser)]
enum Cli {
    Flatten {
        save_dir: PathBuf,
        repo_dir: PathBuf,
    },
    Unflatten {
        save_dir: PathBuf,
        repo_dir: PathBuf,
    },
    Commit {
        save_dir: PathBuf,
        git_dir: PathBuf,
        from: Option<String>,
        merge: Vec<String>,
        commit_message: String,
    },
    Checkout {
        save_dir: PathBuf,
        git_dir: PathBuf,
        commit_id: String,
    },
}

#[tokio::main]
async fn main() {
    match Cli::parse() {
        Cli::Flatten { save_dir, repo_dir } => flatten(save_dir, repo_dir).await,
        Cli::Unflatten { save_dir, repo_dir } => unflatten(save_dir, repo_dir).await,
        Cli::Commit {
            save_dir,
            git_dir,
            from,
            merge,
            commit_message,
        } => {
            let mut parents = Vec::new();
            if let Some(from) = from {
                parents.push(from);
            }
            parents.extend(merge);
            commit(save_dir, git_dir, parents, &commit_message).await;
        }
        Cli::Checkout {
            save_dir,
            git_dir,
            commit_id,
        } => checkout(save_dir, git_dir, commit_id).await,
    }
}
