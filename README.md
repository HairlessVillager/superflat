[中文](README_ZH.md) | English

# Superflat

> [!IMPORTANT]
> **Development Warning**: This project is currently in an aggressive development phase. The Command Line Interface (CLI) and storage formats are not yet stable.

> [!IMPORTANT]
> **Version Support**: Currently targets **Minecraft 1.21.11 Java Edition**. Compatibility with other versions is still being evaluated.

Superflat is a Minecraft save format conversion tool designed to convert Minecraft Java Edition saves into a **Git-friendly** format. By leveraging Git’s mature version control and delta compression capabilities, Superflat achieves:

1.  **Extreme Space Efficiency**: The incremental overhead of storing a snapshot is minimal (typically only **2%** of the original Zip volume of the save).
2.  **Ultra-fast Backup**: Supports rapid snapshotting (Superflat processing speed ~30MiB/s, Git write speed ~20MiB/s).
3.  **Rapid Rollback**: Supports millisecond-level snapshot checkouts (Superflat restoration speed ~45MiB/s).

## Roadmap

- [x] `superflat flatten`: Deconstruct save files into a flattened format.
- [x] `superflat unflatten`: Reconstruct save files from the flattened format.
- [x] Complete Rust refactor
- [x] Basic parallel computing
- [x] `superflat commit`: Stream-flatten and commit to Git.
- [x] `superflat checkout`: Checkout from Git and stream-restore the save.
- [ ] In-depth performance profiling and extreme optimization.
- [ ] Comprehensive user documentation.
- [ ] `superflat merge`: Implement chunk-level and game-semantic level merging.
- [ ] Reduce dependency on Pumpkin for the Sections Dump feature.
- [ ] Expand support for legacy versions (pre-1.21.11).
- [ ] Chunk de-duplication based on Pumpkin terrain generation algorithms (storing only modifications).

## Credits

Special thanks to the [Pumpkin-MC Project](https://github.com/Pumpkin-MC) for inspiration and support. This project relies on [Pumpkin](https://github.com/Pumpkin-MC/Pumpkin) (licensed under GPL-3.0) for its core Sub-chunk Dump (Sections Dump) functionality.

Thanks to Lewis for providing the 4.6GiB real-world test save.

## Installation

Currently, this project must be compiled from source. Pre-compiled binaries for various platforms will be provided once the Rust refactor is complete.

### Local Compilation

Ensure you have [Git](https://git-scm.com/install/) and [rustup](https://rustup.rs/).

```sh
git clone https://github.com/HairlessVillager/superflat.git
cd superflat
# Note: Compiling dependencies pumpkin-data and pumpkin-world is slow (approx. 2-3 minutes)
cargo install --path . --bin sf
```

## Quick Start

This section demonstrates a standard workflow:

### 1. Prepare Paths

You need to define the following three paths:

1.  **Save Path (`$SAVE_DIR`)**: The specific world directory under `.minecraft/saves/` (containing `level.dat`).
2.  **Flattened Repo Path (`$REPO_DIR`)**: The location for intermediate products. We recommend using an SSD or **tmpfs** (RAM disk). Note: Reserve 20x the space of the original save.
3.  **Git Repo Path (`$GIT_DIR`)**: A bare Git repository to store the final backup data. Recommended for reliable storage media; reserve at least 3x the space of the original save.

### 2. Initialize Git Repository

For the first backup, create a bare Git repository and disable auto-GC to manually control performance overhead:

```sh
git init --initial-branch main --bare $GIT_DIR
git --git-dir $GIT_DIR config gc.auto 0
```

### 3. Execute Backup

If this is not your first backup, use `git rev-parse refs/heads/main` to get the commit ID `$COMMIT` that the branch points to:

```sh
sf commit $SAVE_DIR $GIT_DIR -f $COMMIT -m "Your backup note"
```

For the first backup, you can omit the `-f` parameter.

### 4. Optimize Storage (Repack)

It is recommended to check and compress the repository volume after commits:

```sh
# Check current status
git --git-dir $GIT_DIR count-objects -vH

# Perform compression
git --git-dir $GIT_DIR repack -a -d --depth 4095 --window 1

# Perform deep compression (excellent results but time-consuming)
git --git-dir $GIT_DIR repack -a -d --depth 4095 --window 256 -f
```

### 5. Restore Backup

**Note:** If `$REPO_DIR` is not empty, please back up its contents manually (e.g., as a `.zip`) before restoring.

1.  **Find History**:
    ```sh
    git --git-dir $GIT_DIR log --oneline
    ```
2.  **Restore the Save**:
    ```sh
    sf checkout $SAVE_DIR $REPO_DIR -c $COMMIT
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
