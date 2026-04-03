[中文](README_ZH.md) | English

# Superflat

Superflat is a Minecraft save format conversion tool designed to convert Minecraft Java Edition saves into a **Git-friendly** format. By leveraging Git’s mature version control and delta compression capabilities, Superflat achieves:

1.  **Extreme Space Efficiency**: The incremental overhead of storing a snapshot is minimal (typically only **2%** of the original Zip volume of the save).
2.  **Fast Backup**: Supports rapid snapshotting (Superflat processing speed ~30MiB/s, Git write speed ~20MiB/s).
3.  **Fast Rollback**: Supports millisecond-level snapshot checkouts (Superflat restoration speed ~45MiB/s).

## Roadmap

- [x] `superflat flatten`: Deconstruct save files into a flattened format.
- [x] `superflat unflatten`: Reconstruct save files from the flattened format.
- [x] Complete Rust refactor
- [x] Basic parallel computing
- [x] `superflat commit`: Stream-flatten and commit to Git.
- [x] `superflat checkout`: Checkout from Git and stream-restore the save.
- [ ] In-depth performance profiling and extreme optimization.
    - [x] `ChunkRegionCrafter` parallelization
    - [x] `LocalGitOdb` parallelization
    - [ ] More optimization...
- [ ] `superflat merge`: Implement chunk-level and game-semantic level merging.
- [x] Reduce dependency on Pumpkin for the Sections Dump feature.
- [x] Write auto compile GitHub Workflows.
- [x] Expand support for legacy versions (pre-1.21.11).
- [ ] Chunk de-duplication based on Minecraft original terrain generation algorithms (storing only modifications).
- [ ] Change the project license to the Rust community standard MIT/Apache 2.0 dual-license, to better integrate into the Rust ecosystem and allow more developers and organizations to use and contribute without barriers.
    - [ ] Replace the `pumpkin-nbt` dependency
    - [ ] Re-implement the Sub-chunk Dump (Sections Dump) feature
    - [ ] Remove `src/utils/palette.rs` from Git history entirely and force-push

## Credits

Special thanks to the [Pumpkin-MC Project](https://github.com/Pumpkin-MC) for inspiration and support. So far, this project relies on [Pumpkin](https://github.com/Pumpkin-MC/Pumpkin) (licensed under GPL-3.0) for its core Sub-chunk Dump (Sections Dump) functionality.

Thanks to the [`gitoxide` project](https://github.com/GitoxideLabs/gitoxide) (licensed under MIT / Apache-2.0) for providing a highly efficient and modern Git-compatible implementation. This project relies on `gitoxide` for high-performance object reading and writing.

Thanks to Lewis for providing the 4.6GiB real-world test save. In the early stages of development, we lacked a large amount of real experimental data.

## Installation

Ensure [Git](https://git-scm.com/install/) is installed, as `sf commit` and `sf checkout` depend on Git for streaming backup and restoration.

There are two ways to get the Superflat executable:

- Download pre-compiled binaries from the [GitHub Release](https://github.com/HairlessVillager/superflat/releases) page.
- Or compile locally from source.

### Local Compilation

Ensure you have [rustup](https://rustup.rs/) installed.

```sh
git clone https://github.com/HairlessVillager/superflat.git
cd superflat
cargo install --path . --bin sf
```

## Quick Start

This section demonstrates a standard workflow:

### 1. Prepare

You need to define the following two paths:

1.  **Save Path (`$SAVE_DIR`)**: The specific world directory under `.minecraft/saves/` (containing `level.dat`).
2.  **Git Repo Path (`$GIT_DIR`)**: A bare Git repository to store the final backup data. Recommended for reliable storage media; reserve at least 3x the space of the original save.

You also need to know the Minecraft version of your save (`$MC_VERSION`), e.g. `1.21.11`.

### 2. Initialize Git Repository

For the first backup, create a bare Git repository:

```sh
git init --initial-branch main --bare $GIT_DIR
git --git-dir $GIT_DIR config gc.auto 0 # Disable auto-GC for smaller repository size later
git --git-dir $GIT_DIR config core.logAllRefUpdates true # Record reflog for simpler commit syntax
```

Use these commands to check your Git commit identity:

```sh
git config user.name
git config user.email
```

If nothing is displayed, you must set it to prevent commit errors. Use the commands below to set your global Git identity:

```sh
git config --global user.name $YOUR_USER_NAME
git config --global user.email $YOUR_USER_EMAIL
```

### 3. Execute Backup

Use the following command to backup and create a commit:

```sh
sf commit $SAVE_DIR $GIT_DIR --mc-version $MC_VERSION --repack -b main --init -m "Your backup note"
```

This command reads the save at `$SAVE_DIR`, parses it as Minecraft version `$MC_VERSION`, creates an initial commit on the `main` branch of the bare repository at `$GIT_DIR`, and automatically repacks loose objects.

`sf commit --help`:

```text
$ sf commit --help
Flatten save and commit to Git

Usage: sf commit [OPTIONS] --branch <BRANCH> --message <MESSAGE> --mc-version <MC_VERSION> <SAVE_DIR> <GIT_DIR>

Arguments:
  <SAVE_DIR>  Path to your save
  <GIT_DIR>   Path to the bare Git repository

Options:
  -b, --branch <BRANCH>          Commit to this branch
  -v, --verbose...               Increase logging verbosity
      --init                     Commit as initial commit
  -q, --quiet...                 Decrease logging verbosity
  -m, --message <MESSAGE>        Commit message
      --repack                   Automatically repack loose objects
      --mc-version <MC_VERSION>  Minecraft version (e.g. 1.21.11)
  -h, --help                     Print help
```

### 4. Restore Backup

**Note:** If `$SAVE_DIR` is not empty, please back up its contents manually (e.g., as a `.zip`) before restoring.

```sh
sf checkout $SAVE_DIR $GIT_DIR -c "main@{10 minutes ago}" # Restore to the latest commit on the main branch 10 minutes ago
```

## How It Works

The design of Superflat is based on the following core insights:

- **Spatial Dimension**: Most of a Minecraft save's volume is concentrated in `region/*.mca` files. While there are many duplicate blocks and biomes, the `.mca` compression mechanism is limited to the interior of a single chunk.
- **Temporal Dimension**: Differences between adjacent backups are minimal. Traditional Zip backups treat each snapshot as an isolated island, wasting massive amounts of spatio-temporal redundant data.

**In short: Minecraft saves are highly repetitive across both space and time.**

Git, as a mature version control tool, uses object ordering and **Delta Compression algorithms** that can precisely identify and eliminate this redundancy. Superflat "flattens" the complex `.mca` binary format into small files that Git can easily recognize, thereby unlocking Git’s full compression potential.

## Experiments and Benchmarks

We verified the tool's effectiveness using 13 consecutive backups of a survival save (Seed: 42), referred to as the `test42` dataset. For detailed analysis, see [bench.md](docs/blog/bench.md).

### Environment

- **CPU**: AMD Ryzen 7 7840H (16) @ 4.97 GHz
- **RAM**: 32 GiB
- **OS**: Omarchy 3.4.2 (Kernel 6.19.6)

### Key Findings

1.  **Extreme Incremental Compression**: With a `window=2` configuration, the total storage overhead for 13 versions was only 9.15 MiB more than a single Zip backup. This means each incremental backup averages only **1.93%** of the original Zip size.
2.  **Ultimate Archival Potential**: After running `git gc --aggressive`, the total volume of the repository containing 13 historical versions (30.7 MiB) was actually **22% smaller** than a **single** version's Zip archive (39.54 MiB).
3.  **Performance Balance**:
    - Increasing the `window` parameter yields diminishing returns for compression while increasing computation time exponentially.
    - **Daily Backups**: Recommended `window <= 16`; backup time remains stable under 1 second.
    - **Long-term Archiving**: Recommended to perform `gc --aggressive` periodically.

## License

Since this project depends on the GPLv3-licensed Pumpkin project, it is also released under the GPLv3 license:

- [GNU General Public License v3.0](./LICENSE)
