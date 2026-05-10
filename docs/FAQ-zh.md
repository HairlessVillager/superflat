# 常见问题（Frequently Asked Questions）

这里收集了一些用户经常问的问题，希望能解答你的疑问。

- [Git 能这样用吗？](#git-能这样用吗？)
- [跟其他备份工具的区别？除了高性能还有什么其他特性？](#跟其他备份工具的区别除了高性能还有什么其他特性)
- [有 Mod 吗？](#有-mod-吗)
- [支持定时备份功能吗？](#支持定时备份功能吗)
- [支持非原版游戏吗？加了 Mod 插件怎么办？](#支持非原版游戏吗加了-mod-插件怎么办)
- [适用于服务器吗？](#适用于服务器吗)
- [存档大小有没有限制？](#存档大小有没有限制)
- [可以把存档上传到 GitHub 吗？](#可以把存档上传到-github-吗)
- [其他问题？](#其他问题)

## Git 能这样用吗？

**可以**。虽然 Git 的一部分高级操作（如 `git-diff` `git-blame`）无法像对待代码那样对二进制文件进行文本行级别的操作，但它在存储层拥有非常强大的增量压缩机制。当你在存档中只修改了少量数据时，`git-pack-objects` 会使用一种基于字节的差分算法（基于 `xdelta`）。它能够高效地识别出两个二进制版本之间的相似之处，并只存储差异部分。

我们通过一套专门为 Git 差分算法调优的解压机制，将高熵的原始文件高效转换为差分友好的格式，从而使得 Git 能完美捕获并压缩文件之间的冗余部分。

另外，我们正在计划实现 `superflat diff` 等一系列功能，实现存档的方块级操作。

## 跟其他备份工具的区别？除了高性能还有什么其他特性？

首先，在对备份的实现上我们有很大的区别：

- 我们使用 Git 自带的差分算法对游戏文件做去重，每次备份仅需存档约 1% 的空间
- 大部分备份工具仍然使用 `.zip` 格式对每个备份做全量压缩，每次备份需要存档约 80% 的空间
- 一些增量备份工具（如 [QuickBackupM](https://github.com/QuickBackupMultiMod-Dev/QuickBackupM-Reforged) [PrimeBackup](https://github.com/TISUnion/PrimeBackup) [MineBackup](https://github.com/Leafuke/MineBackup)）虽然在不同层面上应用了增量压缩算法，但是都没有实现对高墒存档文件的解压，其单次增量备份的大小还是有提升空间。

> _TODO: 增加性能对比表格_

其次，我们对版本控制的最佳实践 Git 进行复用，低成本地与 Git 持久化逻辑兼容，这意味着你可以对存档的 Git 仓库应用几乎任何 Git 工具（如 VSCode 或者 GitHub），这是其他产品做不到的。

## 有 Mod 吗？

**目前没有**，原因有二：

1. 我们不希望在项目早期阶段加入过多非核心功能，我们希望将时间和精力投入到核心功能的开发中。
2. 我们没有 Mod 开发经验。

如果你是一般用户，请使用 [GUI 程序](../README_ZH.md#基于-GUI)。如果你是 Mod 开发者，欢迎发起 Pull Request！<3

## 支持定时备份功能吗？

GUI 不内置定时备份功能，你可以使用 CLI 程序配合定时任务工具（如 [crontab](https://linux.die.net/man/5/crontab) 或 [PowerShell ScheduledTasks](https://learn.microsoft.com/en-us/powershell/module/scheduledtasks/?view=windowsserver2025-ps)）来组装出这个功能。正如 Unix 系统上管道机制的发明者 Malcolm McIlroy 所言：

> 程序应该只关注一个目标，并尽可能把它做好。

## 支持非原版游戏吗？加了 Mod 插件怎么办？

这个问题的答案稍微有些复杂。

### 需要兼容的情况

如果你的 Mod 满足下面条件之一，那么你将很有可能不能正常地使用 Superflat：

- 修改了 `.dat` `.mca` `.mcc` 文件的存储逻辑
- 修改了 [方块状态](https://zh.minecraft.wiki/w/%E6%96%B9%E5%9D%97%E7%8A%B6%E6%80%81) 或 [生物群系](https://zh.minecraft.wiki/w/%E7%94%9F%E7%89%A9%E7%BE%A4%E7%B3%BB#%E6%95%B0%E6%8D%AE%E5%80%BC) 列表，包含新增项、删除项、替换项

如果你希望 Superflat 支持你喜欢的 Mod，请在 Issues 发一个 `[Feature Request]`，我们会积极地考虑兼容流行的 Mod。

### 可以正常使用的情况

如果你的 Mod 不仅没有修改 `.dat` `.mca` `.mcc` 文件的存储逻辑，也没有修改方块状态和生物群系，而且还满足下面条件之一，那么你可以正常使用 Superflat：

- 没有新增文件类型
- 新增了文件类型，但是不需要关心（比如你可能不需要关心 Distant Horizons 或者 Voxy 模组的 LoD 缓存）

大多数轻量级优化 Mod 都满足上面的条件，所以我们也兼容 Fabulously Optimized 整合包。

## 适用于服务器吗？

对于 Paper、Fabric 以及其他对原版存储机制没有修改的服务核心，**是的**；对于其他服务核心以及服务器插件，请参考之前的 FAQ。

如果你可以登录服务器的后台终端（注意不是游戏服务核心的控制台），你可以直接下载并运行 CLI 程序；如果你不能登录服务器的后台终端，但是可以进入游戏的控制台，你可以安装 [ConsoleMC](https://modrinth.com/mod/consolemc) 或类似的插件 / Mod，以便在控制台中运行宿主机的程序。后续我们会添加更多服务端支持。

## 存档大小有没有限制？

**没有限制**，不过你应当控制你的存档大小不要超过硬盘总空间的 25%，不然后期迁移到更大的硬盘或者删除 Git 历史会比较麻烦。

## 可以把存档上传到 GitHub 吗？

**理论上可以**，只要你的仓库满足 [GitHub 的要求](https://docs.github.com/zh/repositories/creating-and-managing-repositories/repository-limits)。

如果你的仓库不满足托管平台的要求，我们推荐你使用 Gitea 或者类似工具自行部署，Git 作为最佳实践会尽力避免不必要的磁盘空间和网络流量。

在项目的后期我们可能会考虑提供类似的服务。

## 其他问题？

你可以先在 Issues 搜索已有的问题，如果没有欢迎发 `[Question]`！你的每一个提问都是为社区做的贡献，每一个质疑都在推动项目进步。
