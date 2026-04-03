中文 | [English](README.md)

# Superflat

Superflat 是一款 Minecraft 存档格式转换工具，旨在将 Minecraft Java 版存档转换为 **Git 友好** 的格式。通过利用 Git 成熟的版本控制与差分压缩能力，Superflat 实现了：

1.  **极高的空间效率**：存储一份快照的增量开销极小（典型值：单次快照仅占存档原始 Zip 体积的 **2%**）。
2.  **快速备份**：支持快速存储快照（Superflat 处理速度约 30MiB/s，Git 写入速度约 20MiB/s）。
3.  **快速回滚**：支持快速检出快照（Superflat 还原速度约 45MiB/s）。

## 路线图 (Roadmap)

- [x] `superflat flatten`: 存档平坦化（解构）
- [x] `superflat unflatten`: 存档还原（重构）
- [x] Rust 完全重构
- [x] 基本的并行计算
- [x] `superflat commit`：流式平坦化并提交到 Git
- [x] `superflat checkout`：从 Git 检出并流式还原存档
- [ ] 深度性能分析与极致性能优化
    - [x] `ChunkRegionCrafter` 并行化
    - [x] `LocalGitOdb` 并行化
    - [ ] 更多的性能优化
- [ ] `superflat merge`: 实现区块 / 游戏语义级合并
- [x] 精简 Sections Dump 功能对 Pumpkin 的依赖
- [x] 构建自动编译 GitHub 工作流
- [x] 扩展历史版本支持 (1.21.11 之前)
- [ ] 基于 Minecraft 原版地形生成算法的区块去冗余（仅存储修改量）
- [ ] 将项目许可变更为 Rust 社区标准的 MIT/Apache 2.0 双授权，这是为了更好地融入 Rust 生态系统，并让更多的开发者和组织能够无障碍地使用和贡献
    - [ ] 替换 `pumpkin-nbt` 依赖
    - [ ] 重新实现子区块转储（Sections Dump）功能
    - [ ] 从 Git 历史中彻底移除 `src/utils/palette.rs` 文件并强制推送

## 致谢

特别感谢 [Pumpkin-MC 项目](https://github.com/Pumpkin-MC) 对本项目的启发与支持。截至目前，本项目依赖 [Pumpkin](https://github.com/Pumpkin-MC/Pumpkin)（基于 GPL-3.0 协议）实现了核心的子区块转储（Sections Dump）功能。

感谢 [`gitoxide` 项目](https://github.com/GitoxideLabs/gitoxide) （基于 MIT / Apache-2.0 双许可）提供了非常高效且现代的 Git 兼容实现。本项目依赖 `gitoxide` 实现高性能的对象读取与写入功能。

感谢 lewis 提供的共计 4.6GiB 的存档。在早期开发阶段我们非常缺少大量真实的实验数据。

## 安装

请确保系统中已安装 [Git](https://git-scm.com/install/)，`sf commit` 和 `sf checkout` 依赖 Git 进程提供流式备份与还原。

获取 Superflat 的可执行文件有两种方式：

- 从 [GitHub Release](https://github.com/HairlessVillager/superflat/releases) 页面下载预编译的可执行文件
- 或者本地编译安装。

### 本地编译

请确保系统中已安装 [rustup](https://rustup.rs/)。

```sh
git clone https://github.com/HairlessVillager/superflat.git
cd superflat
cargo install --path . --bin sf
```

## 快速开始

本节演示一个标准的工作流：

### 1. 准备

你需要明确以下两个路径：

1.  **存档路径 (`$SAVE_DIR`)**：即 `.minecraft/saves/` 下的具体存档目录（包含 `level.dat`）。
2.  **Git 仓库路径 (`$GIT_DIR`)**：最终存放备份数据的 Git 裸仓库。建议存放在可靠的存储介质上，预留空间建议为原存档的 3 倍以上。

此外你需要记住你的游戏存档的版本号（`$MC_VERSION`），比如 1.21.11 的版本记为 `1.21.11`。

### 2. 初始化 Git 仓库

若是首次备份，请创建一个 Git 裸仓库：

```sh
git init --initial-branch main --bare $GIT_DIR
git --git-dir $GIT_DIR config gc.auto 0 # 禁用自动垃圾回收，以便后面实现更小的仓库体积
git --git-dir $GIT_DIR config core.logAllRefUpdates true # 记录 reflog，以便用更简单的语法表示 commit
```

用下面的命令检查是否设置了 Git 的提交身份：

```sh
git config user.name
git config user.email
```

如果没有输出则需要设置，避免提交时报错，下面的命令设置了全局的 Git 提交身份：

```sh
git config --global user.name $YOUR_USER_NAME
git config --global user.email $YOUR_USER_EMAIL
```

### 3. 执行备份

使用下面的命令备份并创建一个 Commit：

```sh
sf commit $SAVE_DIR $GIT_DIR --mc-version $MC_VERSION --repack -b main --init -m "你的备份注释"
```

这行命令的意思是：读取 `$SAVE_DIR` 位置的存档，按照 `$MC_VERSION` 的游戏版本解析，作为初始提交提交到 `$GIT_DIR` 位置裸仓库的 `main` 分支上，并自动重打包。

`sf commit --help` 命令行帮助文档：

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

### 4. 恢复备份

**注意：** 如果 `$SAVE_DIR` 非空，恢复前请务必手动备份（如使用 `.zip`）。

```sh
sf checkout $SAVE_DIR $GIT_DIR -c "main@{10 minutes ago}" # 恢复到 main 分支 10 分钟前的最近 commit
```

## 实现原理

Superflat 的设计基于以下核心洞察：

- **空间维度**：Minecraft 存档的大部分体积集中在 `region/*.mca` 文件中。虽然游戏内存在大量重复的方块和生物群系，但 `.mca` 的压缩机制仅限于区块内部。
- **时间维度**：相邻备份间的差异极小。传统的 Zip 备份方式将每次备份视为孤岛，浪费了大量的时空冗余数据。

**一句话总结：存档在时空维度上具有高度重复性。**

Git 作为成熟的版本控制工具，其对象排序和 **Delta 压缩算法** 能够精准识别并消除这些重复数据。Superflat 通过将复杂的 `.mca` 二进制格式“拍平”为 Git 易于识别的小文件，从而充分释放 Git 的压缩潜力。

## 实验与基准测试

我们通过一个生存存档（Seed: 42）的 13 次连续备份（名为 `test42` 数据集）验证了工具的有效性。更详细的说明另见 [bench.md](docs/blog/bench.md)

### 实验环境

- **CPU**: AMD Ryzen 7 7840H (16) @ 4.97 GHz
- **内存**: 32 GiB
- **系统**: Omarchy 3.4.2 (Kernel 6.19.6)

### 核心结论

1.  **极高的增量压缩比**：在 `window=2` 的配置下，13 个版本的总存储开销仅比单次 Zip 备份多出 9.15 MiB。这意味着平均每个增量备份仅占原始 Zip 大小的 **1.93%**。
2.  **极致归档潜力**：使用 `git gc --aggressive` 后，包含 13 个历史版本的仓库总体积（30.7 MiB）甚至比 **单个** 版本的 Zip 压缩包（39.54 MiB）还要小 **22%**。
3.  **性能平衡点**：
    - 增大 `window` 参数对压缩率的提升边际效应递减，但计算耗时呈指数级增长。
    - **日常备份**：建议 `window <= 16`，备份耗时可稳定在 1 秒以内。
    - **长期归档**：建议定期执行 `gc --aggressive`。

## 开源许可

由于依赖了 GPLv3 开源的 Pumpkin 项目，本项目采用 GPLv3 许可协议开源：

- [GNU General Public License v3.0](./LICENSE)
