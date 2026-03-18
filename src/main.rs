use std::path::PathBuf;

use clap::Parser;
use superflat_rs::{flatten, unflatten};

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
}

fn main() {
    match Cli::parse() {
        Cli::Flatten { save_dir, repo_dir } => flatten(save_dir, repo_dir),
        Cli::Unflatten { save_dir, repo_dir } => unflatten(save_dir, repo_dir),
    }
}
