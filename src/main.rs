use std::path::PathBuf;

use clap::{Parser, Subcommand};
use clap_verbosity_flag::{InfoLevel, Verbosity};
use superflat::{checkout, commit, flatten, unflatten, utils::git_cmd::git_cmd};

/// Superflat - A bridge between Git and Minecraft save
#[derive(Parser)]
#[command(version, about, long_about = None)]
struct Cli {
    #[command(flatten)]
    verbosity: Verbosity<InfoLevel>,
    #[command(subcommand)]
    action: CliSubcommand,
}

#[derive(Subcommand)]
enum CliSubcommand {
    /// Flatten save to the repo dir
    Flatten {
        /// Path to your save
        save_dir: PathBuf,
        /// Path to the flatten Git repository
        repo_dir: PathBuf,
    },
    /// Restore save from repo dir
    Unflatten {
        /// Path to your save
        save_dir: PathBuf,
        /// Path to the flatten Git repository
        repo_dir: PathBuf,
    },
    /// Flatten save and commit to Git
    Commit {
        /// Path to your save
        save_dir: PathBuf,
        /// Path to the bare Git repository
        git_dir: PathBuf,
        /// Commit ID of the first source. Leave empty to create a initial commit.
        #[arg(long)]
        from: Option<String>,
        /// Commit IDs of other sources.
        #[arg(long)]
        merge: Vec<String>,
        /// Commit message.
        #[arg(short, long)]
        message: String,
        /// Ref name, for branch 'main', it's 'refs/heads/main'
        #[arg(short, long)]
        r#ref: String,
        /// Automatically repack loose objects.
        #[arg(long, default_value_t = false)]
        repack: bool,
    },
    /// Restore save from commit
    Checkout {
        /// Path to your save
        save_dir: PathBuf,
        /// Path to the bare Git repository
        git_dir: PathBuf,
        /// Commit ID to checkout
        #[arg(short, long)]
        commit: String,
    },
    /// Utility tools for debug
    Utils {
        #[command(subcommand)]
        action: UtilsSubcommand,
    },
}

#[derive(Subcommand)]
enum UtilsSubcommand {
    /// Dump chunk nbt data to stdout
    Chunk {
        /// Path to region file
        region_path: PathBuf,
        /// Chunk X
        chunk_x: i32,
        /// Chunk Z
        chunk_z: i32,
    },
}

fn main() {
    let cli = Cli::parse();
    env_logger::Builder::new()
        .filter_level(cli.verbosity.log_level_filter())
        .init();

    log::info!("Welcome to superflat!");
    match cli.action {
        CliSubcommand::Flatten { save_dir, repo_dir } => flatten(save_dir, repo_dir),
        CliSubcommand::Unflatten { save_dir, repo_dir } => unflatten(save_dir, repo_dir),
        CliSubcommand::Commit {
            save_dir,
            git_dir,
            from,
            merge,
            message,
            r#ref,
            repack,
        } => {
            let mut parents = Vec::new();
            if let Some(from) = from {
                parents.push(from);
            }
            parents.extend(merge);
            commit(save_dir, git_dir.to_owned(), parents, &message, Some(r#ref));

            if repack {
                log::info!("Repacking");
                let _repack_out = git_cmd(git_dir.to_owned())
                    .args(["repack", "--depth", "4095", "--window", "2", "-a", "-d"])
                    .output()
                    .unwrap();
            } else {
                log::warn!("--repack is not enabled, Git repository can get bloated")
            }
            log::info!("Counting objects");
            let count_out = git_cmd(git_dir.to_owned())
                .args(["count-objects", "-vH"])
                .output()
                .unwrap()
                .stdout;
            for line in String::from_utf8(count_out).unwrap().lines() {
                log::info!("git-count-objects: {line}")
            }
        }
        CliSubcommand::Checkout {
            save_dir,
            git_dir,
            commit,
        } => checkout(save_dir, git_dir, commit),

        CliSubcommand::Utils { action } => match action {
            UtilsSubcommand::Chunk {
                region_path,
                chunk_x,
                chunk_z,
            } => {
                use std::fs;
                use std::io::{self, Write};
                use superflat::utils::region::{parse_xz, read_region};

                let (region_x, region_z) =
                    parse_xz(region_path.file_name().unwrap().to_str().unwrap());
                let (_, xz_nbts) =
                    read_region(fs::File::open(region_path).unwrap(), region_x, region_z).unwrap();
                let (_, _, nbt) = xz_nbts
                    .iter()
                    .find(|(x, z, _)| *x == chunk_x && *z == chunk_z)
                    .unwrap();
                io::stdout().write_all(nbt).unwrap();
            }
        },
    }
}
