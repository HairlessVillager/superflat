# Frequently Asked Questions (FAQ)

Here are some frequently asked questions that may help answer your queries.

- [Can Git be used this way?](#can-git-be-used-this-way)
- [How is it different from other backup tools? What other features does it have besides high performance?](#how-is-it-different-from-other-backup-tools-what-other-features-does-it-have-besides-high-performance)
- [Is there a Mod?](#is-there-a-mod)
- [Does it support scheduled backups?](#does-it-support-scheduled-backups)
- [Does it support non-vanilla games? What about Mods/Plugins?](#does-it-support-non-vanilla-games-what-about-modsplugins)
- [Is it suitable for servers?](#is-it-suitable-for-servers)
- [Is there a limit on save size?](#is-there-a-limit-on-save-size)
- [Can I upload saves to GitHub?](#can-i-upload-saves-to-github)
- [Other questions?](#other-questions)

## Can Git be used this way?

**Yes**. Although some advanced Git operations (like `git-diff` `git-blame`) cannot work on binary files the way they do on text files, Git has a very powerful incremental compression mechanism at the storage layer. When you only modify a small amount of data in your save, `git-pack-objects` uses a byte-based differential algorithm (based on `xdelta`). It can efficiently identify the similarities between two binary versions and only store the differences.

We use a specially tuned decompression mechanism to efficiently convert high-entropy raw files into a diff-friendly format, allowing Git to perfectly capture and compress redundant parts between files.

Additionally, we are planning to implement features like `superflat diff` to enable block-level operations on saves.

## How is it different from other backup tools? What other features does it have besides high performance?

First, we have significant differences in our backup implementation:

- We use Git's built-in differential algorithm for deduplication, with each backup taking only ~1% of the save space
- Most backup tools still use `.zip` format for full compression of each backup, requiring ~80% of the save space per backup
- Some incremental backup tools (like [QuickBackupM](https://github.com/QuickBackupMultiMod-Dev/QuickBackupM-Reforged) [PrimeBackup](https://github.com/TISUnion/PrimeBackup) [MineBackup](https://github.com/Leafuke/MineBackup)) apply incremental compression algorithms at different levels, but none implement decompression for high-entropy save files, so there's still room for improvement in single incremental backup sizes.

> _TODO: Add performance comparison table_

Secondly, we leverage Git's best practices for version control, which means you can apply almost any Git tool (like VSCode or GitHub) to your save's Git repository — something other products cannot do.

## Is there a Mod?

**Not currently**, for two reasons:

1. We don't want to add too many non-core features in the early stages of the project. We want to focus time and energy on core functionality development.
2. We don't have Mod development experience.

If you're a regular user, please use the [GUI program](../README.md#using-the-gui). If you're a Mod developer, feel free to submit a Pull Request! <3

## Does it support scheduled backups?

The GUI does not have built-in scheduled backup functionality. You can use the CLI program together with scheduled task tools (like [crontab](https://linux.die.net/man/5/crontab) or [PowerShell ScheduledTasks](https://learn.microsoft.com/en-us/powershell/module/scheduledtasks/?view=windowsserver2025-ps)) to achieve this functionality. As Malcolm McIlroy, inventor of the Unix pipe mechanism, said:

> Programs should do one thing and do it well.

## Does it support non-vanilla games? What about Mods/Plugins?

The answer to this question is slightly complex.

### Cases that need compatibility

If your Mod meets any of the following conditions, you may not be able to use Superflat properly:

- Modified the storage logic of `.dat`, `.mca`, or `.mcc` files
- Modified the [block state](https://minecraft.wiki/w/Block_states) or [biome](https://minecraft.wiki/w/Biome#Biome_IDs) list, including adding, removing, or replacing items

If you want Superflat to support your favorite Mod, please post a `[Feature Request]` in Issues — we will actively consider supporting popular Mods.

### Cases that work normally

If your Mod not only hasn't modified the storage logic of `.dat`, `.mca`, `.mcc` files and hasn't modified block states and biomes, but also meets any of the following conditions, then you can use Superflat normally:

- No new file types added
- New file types added, but you don't need to care about them (e.g., you may not need to care about Distant Horizons or Voxy mod's LoD cache)

Most lightweight optimization Mods meet the conditions above, so we are also compatible with the Fabulously Optimized modpack.

## Is it suitable for servers?

**Yes**, for Paper, Fabric, and other server cores that haven't modified the vanilla storage mechanism. For server plugins, please refer to the previous FAQ.

## Is there a limit on save size?

There is **no limit**, but you should keep your save size under 25% of your total hard drive space. Otherwise, migrating to a larger hard drive or deleting Git history can be troublesome later.

## Can I upload saves to GitHub?

**Theoretically yes**, as long as your repository meets [GitHub's requirements](https://docs.github.com/en/repositories/creating-and-managing-repositories/repository-limits).

If your repository doesn't meet the hosting platform's requirements, we recommend deploying Gitea or similar tools yourself. As a best practice, Git works to avoid unnecessary disk space and network traffic.

We may consider providing a similar service in later stages of the project.

## Other questions?

You can first search for existing issues in Issues. If you can't find your question, feel free to post a `[Question]`! Every question you ask is a contribution to the community, and every concern drives the project forward.
