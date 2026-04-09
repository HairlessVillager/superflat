use std::path::PathBuf;

use anyhow::Context;
use clap::{Parser, Subcommand};
use clap_verbosity_flag::{InfoLevel, Verbosity};
use superflat::{
    checkout, commit, flatten, repack, unflatten,
    utils::cmd::{git_cmd, git_count_objects, git_repo_exists},
};

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
        /// Minecraft version (e.g. 1.21.11)
        #[arg(long)]
        mc_version: String,
    },
    /// Restore save from repo dir
    Unflatten {
        /// Path to your save
        save_dir: PathBuf,
        /// Path to the flatten Git repository
        repo_dir: PathBuf,
        /// Minecraft version (e.g. 1.21.11)
        #[arg(long)]
        mc_version: String,
    },
    /// Flatten save and commit to Git
    Commit {
        /// Path to your save
        save_dir: PathBuf,
        /// Path to the bare Git repository
        #[arg(value_parser = git_repo_exists)]
        git_dir: PathBuf,
        /// Commit to this branch.
        #[arg(short, long)]
        branch: String,
        /// Commit as initial commit.
        #[arg(long)]
        init: bool,
        /// Commit message.
        #[arg(short, long)]
        message: String,
        /// Automatically repack loose objects.
        #[arg(long = "repack", default_value_t = false)]
        use_repack: bool,
        /// Minecraft version (e.g. 1.21.11)
        #[arg(long)]
        mc_version: String,
    },
    /// Restore save from commit
    Checkout {
        /// Path to your save
        save_dir: PathBuf,
        /// Path to the bare Git repository
        git_dir: PathBuf,
        /// Commit-ish to checkout (commit ID or revision expression, e.g. HEAD^1, branch~2)
        #[arg(short, long)]
        commit: String,
        /// Minecraft version (e.g. 1.21.11)
        #[arg(long)]
        mc_version: String,
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
    /// Dump section block or biome data to stdout
    Section {
        /// Path to region file
        region_path: PathBuf,
        /// Chunk X
        chunk_x: i32,
        /// Chunk Z
        chunk_z: i32,
        /// Section Y index
        section_y: i8,
        /// Dump block state IDs (4096 x u16 LE)
        #[arg(long, group = "data_type", required = true)]
        block: bool,
        /// Dump biome IDs (64 x u8)
        #[arg(long, group = "data_type", required = true)]
        biome: bool,
    },
}

fn main() {
    let cli = Cli::parse();
    env_logger::Builder::new()
        .filter_level(cli.verbosity.log_level_filter())
        .init();

    log::info!("Welcome to superflat!");
    match cli.action {
        CliSubcommand::Flatten {
            save_dir,
            repo_dir,
            mc_version,
        } => flatten(save_dir, repo_dir, &mc_version),
        CliSubcommand::Unflatten {
            save_dir,
            repo_dir,
            mc_version,
        } => unflatten(save_dir, repo_dir, &mc_version),
        CliSubcommand::Commit {
            save_dir,
            git_dir,
            branch,
            init,
            message,
            use_repack,
            mc_version,
        } => {
            let parents = {
                let mut cmd = git_cmd(&git_dir, ["rev-parse", &format!("{branch}^{{commit}}")]);
                let out = cmd.output().expect("failed to run git rev-parse");
                let branch_exists = out.status.success();
                match (branch_exists, init) {
                    (true, false) => {
                        vec![
                            String::from_utf8(out.stdout)
                                .expect("git output is not valid UTF-8")
                                .trim()
                                .to_owned(),
                        ]
                    }
                    (false, true) => vec![],
                    (true, true) => panic!("Branch '{branch}' exists, remove --init"),
                    (false, false) => panic!(
                        "Invalid branch name '{branch}'. Self-check via 'git --git-dir {:?} rev-parse {branch}^{{commit}}'",
                        git_dir.as_os_str()
                    ),
                }
            };
            let r#ref = format!("refs/heads/{}", &branch);

            commit(
                save_dir,
                git_dir.to_owned(),
                parents,
                &message,
                Some(r#ref),
                &mc_version,
            );

            if use_repack {
                git_count_objects(&git_dir).expect("failed to count git objects");
                repack(git_dir.to_owned());
            } else {
                log::warn!("--repack is not enabled, Git repository can get bloated") // TODO: opt prompt
            }

            git_count_objects(git_dir.to_owned()).expect("failed to count git objects");
        }
        CliSubcommand::Checkout {
            save_dir,
            git_dir,
            commit,
            mc_version,
        } => {
            if save_dir.exists() {
                let bak = save_dir.with_extension("bak");
                log::warn!("save_dir {save_dir:?} already exists, renaming to {bak:?}");
                std::fs::rename(&save_dir, &bak).expect("failed to rename save directory");
            }
            checkout(save_dir, git_dir, commit, &mc_version)
        }

        CliSubcommand::Utils { action } => match action {
            UtilsSubcommand::Chunk {
                region_path,
                chunk_x,
                chunk_z,
            } => {
                use std::fs;
                use std::io::{self, Write};
                use superflat::utils::region::{parse_xz, read_region};

                let (region_x, region_z) = parse_xz(
                    region_path
                        .file_name()
                        .expect("invalid region path")
                        .to_str()
                        .expect("region path contains invalid UTF-8"),
                )
                .expect("failed to parse region filename");
                let (_, xz_nbts) = read_region(
                    fs::File::open(region_path).expect("failed to open region file"),
                    region_x,
                    region_z,
                )
                .expect("failed to read region file")
                .expect("region file is empty");
                let (_, _, nbt) = xz_nbts
                    .iter()
                    .find(|(x, z, _)| *x == chunk_x && *z == chunk_z)
                    .with_context(|| {
                        format!(
                            "Missing chunk, all chunk positions: {:#?}",
                            xz_nbts
                                .iter()
                                .map(|(x, z, _)| format!("({x}, {z})"))
                                .collect::<Vec<_>>()
                        )
                    })
                    .expect("chunk not found");
                io::stdout()
                    .write_all(nbt)
                    .expect("failed to write to stdout");
            }
            UtilsSubcommand::Section {
                region_path,
                chunk_x,
                chunk_z,
                section_y,
                block,
                biome: _,
            } => {
                use std::fs;
                use std::io::{self, Cursor, Write};
                use superflat::utils::nbt::load_nbt;
                use superflat::utils::region::{parse_xz, read_region, split_chunk};

                let (region_x, region_z) = parse_xz(
                    region_path
                        .file_name()
                        .expect("invalid region path")
                        .to_str()
                        .expect("region path contains invalid UTF-8"),
                )
                .expect("failed to parse region filename");
                let (_, xz_nbts) = read_region(
                    fs::File::open(region_path).expect("failed to open region file"),
                    region_x,
                    region_z,
                )
                .expect("failed to read region file")
                .expect("region file is empty");
                let (_, _, nbt_bytes) = xz_nbts
                    .iter()
                    .find(|(x, z, _)| *x == chunk_x && *z == chunk_z)
                    .expect("chunk not found");
                let nbt = load_nbt(Cursor::new(nbt_bytes), true);
                let (_, sections_dump) =
                    split_chunk(nbt).expect("failed to load sections dump from chunk nbt");
                let section = sections_dump
                    .sections
                    .iter()
                    .find(|s| s.y == section_y)
                    .expect("section not found");
                let mut stdout = io::stdout().lock();
                if block {
                    let bytes: Vec<u8> = section
                        .block_state
                        .iter()
                        .flat_map(|&v| v.to_le_bytes())
                        .collect();
                    stdout.write_all(&bytes).expect("failed to write to stdout");
                } else {
                    stdout
                        .write_all(&section.biome)
                        .expect("failed to write to stdout");
                }
            }
        },
    }
}
