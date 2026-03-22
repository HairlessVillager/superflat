use std::path::PathBuf;

use clap::Parser;
use superflat::{checkout, commit, flatten, unflatten};

/// Superflat - A bridge between Git and Minecraft save
#[derive(Parser)]
enum Cli {
    Flatten {
        /// Path to your save
        save_dir: PathBuf,
        /// Path to the flatten Git repository
        repo_dir: PathBuf,
    },
    Unflatten {
        /// Path to your save
        save_dir: PathBuf,
        /// Path to the flatten Git repository
        repo_dir: PathBuf,
    },
    Commit {
        /// Path to your save
        save_dir: PathBuf,
        /// Path to the bare Git repository
        git_dir: PathBuf,
        /// Commit ID of the first source. Leave empty to create a initial commit.
        from: Option<String>,
        /// Commit IDs of other sources.
        merge: Vec<String>,
        /// Commit message
        commit_message: String,
    },
    Checkout {
        /// Path to your save
        save_dir: PathBuf,
        /// Path to the bare Git repository
        git_dir: PathBuf,
        /// Commit ID to checkout
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
