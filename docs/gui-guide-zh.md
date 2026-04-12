# GUI 程序使用指引

<p align="center">
  <img src="images/screenshot-2026-04-12_19-48-22.png" alt="image" width="854">
</p>

我们为 GUI 程序设计了几条动线：

- [本地备份与恢复](#本地备份与恢复)
- [本地存档同步到远程仓库](#本地存档同步到远程仓库)

## 本地备份与恢复

点击左上方的 `☰` 按钮打开档案菜单。

<p align="center">
  <img src="images/screenshot-2026-04-12_19-50-15.png" alt="image" width="854">
</p>

点左边的 `Track Local Save` 跟踪本地存档：

<p align="center">
  <img src="images/screenshot-2026-04-12_19-50-20.png" alt="image" width="854">
</p>

填写必要信息：

- `Save directory`： 存档文件夹，包含 `level.dat` 文件
- `Branch`：Git 分支名称，默认为 `main`，推荐使用默认值
- `MC Version`：Minecraft Java 版游戏版本号，例如 `1.18`、`1.21.11`、 `26.1`

<p align="center">
  <img src="images/screenshot-2026-04-12_19-50-45.png" alt="image" width="854">
</p>

点击 `Track` 保存档案：

<p align="center">
  <img src="images/screenshot-2026-04-12_19-48-45.png" alt="image" width="854">
</p>

选择这个档案：

<p align="center">
  <img src="images/screenshot-2026-04-12_20-09-47.png" alt="image" width="854">
</p>

点击 `Commit` 按钮，填写必要的信息

<p align="center">
  <img src="images/screenshot-2026-04-12_20-55-06.png" alt="image" width="854">
</p>

尽管这里是英文，你也可以用中文的提交消息。这里提供 Minecraft 的修改类型，可以参考：

- arch：成就突破
- build：建筑 / 装饰
- farm：生电机器 / 自动化
- explore：探索 / 坐标发现 / 维度旅行
- daily：维护 / 物资积累 / 仓库整理
- system：Mod / 版本 / 游戏规则

提交完成之后可以看到提交历史：

<p align="center">
  <img src="images/screenshot-2026-04-12_21-00-58.png" alt="image" width="854">
</p>

点击 `Checkout` 按钮即可恢复这个提交（原存档会被移动到 `.minecraft/saves/<save-name>.bak`）：

<p align="center">
  <img src="images/screenshot-2026-04-12_21-02-42.png" alt="image" width="854">
</p>

## 本地存档同步到远程仓库

打开一个档案，点击 `Set Remote` 按钮，在 `Remote URL` 一栏填写你的远程仓库地址（推荐使用 SSH 协议）：

<p align="center">
  <img src="images/screenshot-2026-04-12_21-07-11.png" alt="image" width="854">
</p>

点击 OK 之后就能看到 `Pull`（拉取远程到本地）和 `Push`（推送本地到远程）。

<p align="center">
  <img src="images/screenshot-2026-04-12_21-10-52.png" alt="image" width="854">
</p>

如果你刚刚新建了一个提交，可以点击 `Push` 推送到远程仓库；反之，如果你要获取远程仓库的最新存档，可以点击 `Pull` 拉取远程仓库，在点对应提交的 `Checkout` 恢复存档。
