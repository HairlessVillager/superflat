中文 | [English](README.md)

<div align="center">

# ⛏️ Superflat

**基于 Git 的 Minecraft Java 版存档版本控制工具**

[![License: Apache-2.0 OR MIT](https://img.shields.io/badge/License-Apache--2.0%20OR%20MIT-blue.svg)](#license)
[![Built with Rust](https://img.shields.io/badge/Built%20with-Rust-orange?logo=rust&logoColor=white)](https://www.rust-lang.org/)
[![Platform](https://img.shields.io/badge/Platform-Windows%20%7C%20Linux%20%7C%20MasOS-lightgrey?logo=github)](https://github.com/HairlessVillager/superflat/releases)
[![GitHub Release](https://img.shields.io/github/v/release/HairlessVillager/superflat?color=green)](https://github.com/HairlessVillager/superflat/releases)

</div>

---

Superflat 是一款 Minecraft 存档格式转换工具，旨在将 Minecraft Java 版存档转换为 **Git 友好** 的格式。通过利用 Git 成熟的版本控制与差分压缩能力，Superflat 实现了：

- 🗜️ **极高的空间效率**：每次增量备份平均仅占存档原始 Zip 体积的 **~2%**
- ⚡ **快速备份**：处理速度约 30 MiB/s，Git 写入速度约 20 MiB/s
- 🔄 **快速回滚**：还原速度约 45 MiB/s

## 🗺️ 路线图 (Roadmap)

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
- [ ] 扩展版本支持
    - [x] 方块与群系数据版本支持
    - [ ] 存档目录格式支持（26.1 及之后）
- [ ] 基于 Minecraft 原版地形生成算法的区块去冗余（仅存储修改量）
- [x] 将项目许可变更为 Rust 社区标准的 MIT/Apache 2.0 双授权
    - [x] 替换 `pumpkin-nbt` 依赖
    - [x] 重新实现子区块转储（Sections Dump）功能
    - [x] 从 Git `main` 分支移除 `src/utils/palette.rs` 文件并强制推送

## 🙏 致谢

特别感谢 [Pumpkin-MC 项目](https://github.com/Pumpkin-MC) 对本项目的启发（以及对本项目 [历史版本](https://github.com/HairlessVillager/superflat/tree/gplv3-legacy-main) 的支持）。

感谢 [`gitoxide` 项目](https://github.com/GitoxideLabs/gitoxide) （基于 MIT / Apache-2.0 双许可）提供了非常高效且现代的 Git 兼容实现。本项目依赖 `gitoxide` 实现高性能的对象读取与写入功能。

感谢 lewis 提供的共计 4.6 GiB 的存档。在早期开发阶段我们非常缺少大量真实的实验数据。

## 📦 下载与安装

请确保系统中已安装 [Git](https://git-scm.com/install/)，`sf commit` 和 `sf checkout` 依赖 Git 进程提供流式备份与还原。

获取 Superflat 的可执行文件有两种方式：

- **预编译版本**：从 [GitHub Release](https://github.com/HairlessVillager/superflat/releases) 页面下载预编译的可执行文件
- **从源码编译**：本地编译安装（见下文）

### 本地编译

请确保系统中已安装 [rustup](https://rustup.rs/)，然后执行：

```sh
git clone https://github.com/HairlessVillager/superflat.git
cd superflat
cargo install --path . --bin sf
```

## 🚀 快速开始

### 基于 GUI

我们为 Windows 用户准备了 GUI 版本的程序。在 [GitHub Release 页面](https://github.com/HairlessVillager/superflat/releases) 下载 `superflat-gui-x.x.x-x86_64-pc-windows-msvc.exe` 可执行文件。下载后双击运行 `.exe` 即可运行。

> [!TIP]
> 如果您信任我们的程序，请把 GUI 进程加入 Window Defender 白名单（[中文教程](docs/windows-defender-bypass-zh.md)），从而获得更好的性能。

点击 [此处](docs/gui-guide-zh.md) 查看 GUI 程序的使用指引。GUI 程序致力于给基础操作（`sf commit`、`sf checkout` 以及一部分 Git 命令）提供一个所见即所得的用户界面，对于高级操作请使用 CLI 程序。

### 基于 CLI

本节演示一个标准的工作流：

#### 步骤 1 — 准备

你需要明确以下两个路径：

1. **存档路径 (`$SAVE_DIR`)**：即 `.minecraft/saves/` 下的具体存档目录（包含 `level.dat`）。
2. **Git 仓库路径 (`$GIT_DIR`)**：最终存放备份数据的 Git 裸仓库。建议存放在可靠的存储介质上，预留空间建议为原存档的 3× 以上。

此外你需要记住你的游戏存档的版本号（`$MC_VERSION`），比如 1.21.11 的版本记为 `1.21.11`。

#### 步骤 2 — 初始化 Git 仓库

若是首次备份，请创建一个 Git 裸仓库：

```sh
git init --initial-branch main --bare $GIT_DIR
git --git-dir $GIT_DIR config gc.auto 0              # 禁用自动垃圾回收，以便后面实现更小的仓库体积
git --git-dir $GIT_DIR config core.logAllRefUpdates true  # 记录 reflog，以便用更简单的语法表示 commit
```

用下面的命令检查是否设置了 Git 的提交身份：

```sh
git config user.name
git config user.email
```

如果没有输出则需要设置，避免提交时报错：

```sh
git config --global user.name $YOUR_USER_NAME
git config --global user.email $YOUR_USER_EMAIL
```

#### 步骤 3 — 执行备份

```sh
sf commit $SAVE_DIR $GIT_DIR --mc-version $MC_VERSION --repack -b main --init -m "你的备份注释"
```

这行命令的意思是：读取 `$SAVE_DIR` 位置的存档，按照 `$MC_VERSION` 的游戏版本解析，作为初始提交提交到 `$GIT_DIR` 位置裸仓库的 `main` 分支上，并自动重打包。

<details>
<summary><code>sf commit --help</code></summary>

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

</details>

#### 步骤 4 — 恢复备份

> [!WARNING]
> 如果 `$SAVE_DIR` 非空，恢复前请务必手动备份（如使用 `.zip`）。

```sh
sf checkout $SAVE_DIR $GIT_DIR -c "main@{10 minutes ago}"
# 恢复到 main 分支 10 分钟前的最近 commit
```

## 🔬 实现原理

Superflat 的设计基于以下两个核心洞察：

- **空间维度**：Minecraft 存档的大部分体积集中在 `region/*.mca` 文件中。虽然游戏内存在大量重复的方块和生物群系，但 `.mca` 的压缩机制仅限于区块内部。
- **时间维度**：相邻备份间的差异极小。传统的 Zip 备份方式将每次备份视为孤岛，浪费了大量的时空冗余数据。

> **一句话总结：存档在时空维度上具有高度重复性。**

Git 作为成熟的版本控制工具，其对象排序和 **Delta 压缩算法** 能够精准识别并消除这些重复数据。Superflat 通过将复杂的 `.mca` 二进制格式"拍平"为 Git 易于识别的小文件，从而充分释放 Git 的压缩潜力。

## 📊 实验与基准测试

我们通过一个生存存档（Seed: 42）的 13 次连续备份（名为 `test42` 数据集）验证了工具的有效性。更详细的说明另见 [bench.md](docs/blog/bench.md)

### 实验环境

| 组件 | 详情                                   |
| ---- | -------------------------------------- |
| CPU  | AMD Ryzen 7 7840H (16 线程) @ 4.97 GHz |
| 内存 | 32 GiB                                 |
| 系统 | Omarchy 3.4.2 (Kernel 6.19.6)          |

### 核心结论

1. **极高的增量压缩比**：在 `window=2` 的配置下，13 个版本的总存储开销仅比单次 Zip 备份多出 9.15 MiB。这意味着平均每个增量备份仅占原始 Zip 大小的 **1.93%**。
2. **极致归档潜力**：使用 `git gc --aggressive` 后，包含 13 个历史版本的仓库总体积（30.7 MiB）甚至比 **单个** 版本的 Zip 压缩包（39.54 MiB）还要小 **22%**。
3. **性能平衡点**：
    - 增大 `window` 参数对压缩率的提升边际效应递减，但计算耗时呈指数级增长。
    - **日常备份**：建议 `window <= 16`，备份耗时可稳定在 1 秒以内。
    - **长期归档**：建议定期执行 `gc --aggressive`。

## 📄 开源许可

本项目采用双重授权许可：

- Apache License, Version 2.0 ([LICENSE-APACHE](./LICENSE-APACHE) or http://www.apache.org/licenses/LICENSE-2.0)
- MIT license ([LICENSE-MIT](./LICENSE-MIT) or http://opensource.org/licenses/MIT)

您可以根据需要选择其中之一。

> [!CAUTION]
> **历史版本说明：**
>
> 由于之前的版本依赖于采用 GPLv3 协议的项目 Pumpkin，因此 [gplv3-legacy-main 分支](https://github.com/HairlessVillager/superflat/tree/gplv3-legacy-main) 仍遵循 [GNU General Public License v3.0](./LICENSE)。当前主版本已移除相关依赖，协议已变更为更为宽松的 Apache/MIT 双授权。
