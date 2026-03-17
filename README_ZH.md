中文 | [English](README.md)

# Superflat

> [!IMPORTANT]
> **开发预警**：本项目正处于激进开发阶段，命令行接口（CLI）和存储格式尚未稳定。

> [!IMPORTANT]
> **版本支持**：目前主要针对 **Minecraft 1.21.11 Java Edition**。其他版本的兼容性仍在评估中。

> [!IMPORTANT]
> **空间占用**：存档平坦化后，中间产物（平坦化目录）的体积会膨胀约 20 倍。
> _原因：当前版本主要为概念验证（PoC），尚未实现流式计算。我们计划在下一版本中通过流式处理修复此问题。_

Superflat 是一款 Minecraft 存档格式转换工具，旨在将 Minecraft Java 版存档转换为 **Git 友好** 的格式。通过利用 Git 成熟的版本控制与差分压缩能力，Superflat 实现了：

1.  **极高的空间效率**：存储一份快照的增量开销极小（典型值：单次快照仅占存档原始 Zip 体积的 **2%**）。
2.  **极速备份**：支持快速存储快照（Superflat 处理速度约 30MiB/s，Git 写入速度约 20MiB/s）。
3.  **快速回滚**：支持快照的毫秒级检出（Superflat 还原速度约 45MiB/s）。

## 路线图 (Roadmap)

- [x] `superflat flatten`: 存档平坦化（解构）
- [x] `superflat unflatten`: 存档还原（重构）
- [ ] Rust 完全重构 / 引入流式计算
- [ ] 深度性能分析与极致性能优化
- [ ] 完善用户文档
- [ ] `superflat merge`: 实现区块 / 游戏语义级合并
- [ ] 精简 Sections Dump 功能对 Pumpkin 的依赖
- [ ] 扩展历史版本支持 (1.21.11 之前)
- [ ] 基于 Pumpkin 地形生成算法的区块去冗余（仅存储修改量）

## 致谢

特别感谢 [Pumpkin-MC 项目](https://github.com/Pumpkin-MC) 对本项目的启发与支持。本项目依赖 [Pumpkin](https://github.com/Pumpkin-MC/Pumpkin)（基于 GPL-3.0 协议）实现了核心的子区块转储（Sections Dump）功能。

感谢 lewis 提供的 4.6GiB 真实测试存档。

## 安装

目前本项目需从源码编译。待 Rust 重构完成后，我们将提供各平台的预编译二进制文件。

### 本地编译

请确保系统中已安装 [Git](https://git-scm.com/install/) 和 [uv](https://docs.astral.sh/uv/getting-started/installation/)。

```sh
git clone https://github.com/HairlessVillager/superflat.git
cd superflat
# 注意：依赖项 pumpkin-data 和 pumpkin-world 编译较慢，约需 2-3 分钟
uv tool install .
```

## 快速开始

本节演示一个标准的工作流：

### 1. 路径准备

你需要明确以下三个路径：

1.  **存档路径 (`$SAVE_DIR`)**：即 `.minecraft/saves/` 下的具体存档目录（包含 `level.dat`）。
2.  **平坦化仓库路径 (`$REPO_DIR`)**：中间产物存放处。建议存放在固态硬盘（SSD）或 **tmpfs**（内存盘）中。注意：需预留原存档 20 倍的空间。
3.  **Git 仓库路径 (`$GIT_DIR`)**：最终存放备份数据的 Git 裸仓库。建议存放在可靠的存储介质上，预留空间建议为原存档的 3 倍以上。

### 2. 初始化 Git 仓库

若是首次备份，请创建一个 Git 裸仓库并禁用自动垃圾回收（以手动控制性能开销）：

```sh
git init --initial-branch main --bare $GIT_DIR
git --git-dir $GIT_DIR config gc.auto 0
```

### 3. 执行备份

首先，将存档转换为平坦化格式：

```sh
sf flatten -s $SAVE_DIR -r $REPO_DIR
```

接着，将数据提交至 Git 仓库：

```sh
git --git-dir $GIT_DIR --work-tree $REPO_DIR add .
git --git-dir $GIT_DIR --work-tree $REPO_DIR commit -m "你的备份注释"
```

_此时备份已完成。你可以删除 `$REPO_DIR` 以释放空间，但在下次还原或备份前需重新生成。_

### 4. 优化存储 (Repack)

建议每提交一次后都通过以下命令查看并压缩仓库体积：

```sh
# 查看当前状态
git --git-dir $GIT_DIR count-objects -vH

# 执行压缩
git --git-dir $GIT_DIR repack -a -d --depth 4095 --window 1

# 执行深度压缩（效果很好但非常耗时）
git --git-dir $GIT_DIR repack -a -d --depth 4095 --window 256 -f
```

### 5. 恢复备份

**注意：** 如果 `$REPO_DIR` 非空，恢复前请务必手动备份（如使用 `.zip`）。

1.  **查找历史版本**：
    ```sh
    git --git-dir $GIT_DIR log --oneline
    ```
2.  **切换到指定提交 (Commit ID)**：
    ```sh
    git --git-dir $GIT_DIR --work-tree $REPO_DIR reset --hard <commit-id>
    ```
3.  **还原存档**：
    ```sh
    sf unflatten -s $SAVE_DIR -r $REPO_DIR
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

本项目采用双重许可协议：

- [Apache License, Version 2.0](./LICENSE-APACHE)
- [MIT License](./LICENSE-MIT)

你可以根据偏好选择其中之一。
