# Whitelisting Superflat GUI in Windows Defender

If you trust the GUI executable downloaded from this project's Release page, you can add it to Microsoft Defender's exclusions to reduce the overhead from real-time scanning.

This is optional and purely a performance optimization. Only do this if you are confident the executable comes from a trusted source.

## Why Bother

Superflat reads and writes a large number of small files rapidly during backup and restore operations. Windows Defender's real-time scanning may repeatedly inspect these file accesses, leading to:

- Slower backup or restore speeds in the GUI
- Console output has started, but disk usage remains persistently high
- More noticeable lag on first run or with large saves

After adding the GUI process to the exclusions, Defender will no longer scan file operations triggered by that process at the same intensity, which typically improves performance.

## Steps

The screenshots below are from a Windows 11 interface. Windows 10 may use slightly different names, but the paths are essentially the same.

### 1. Open Windows Security

Open `Windows Security` and go to the home page:

![Windows Security home](images/screenshot-2026-04-03-182603.png)

### 2. Go to Virus & Threat Protection

Click `Virus & threat protection` in the left sidebar:

![Virus & threat protection](images/screenshot-2026-04-03-182613.png)

Scroll down to find `Virus & threat protection settings` and click `Manage settings`.

### 3. Open Exclusions Settings

Continue scrolling to the `Exclusions` section and click `Add or remove exclusions`:

![Exclusions entry](images/screenshot-2026-04-03-182629.png)

### 4. Add the GUI Process

On the exclusions page, click `Add an exclusion`, select `Process`, then enter the executable name or full path of `superflat-gui`.

For example, if you run the file downloaded directly from the Release page, you can enter a path like:

`C:\Users\<your-username>\Downloads\superflat-gui-v0.5.0-x86_64-pc-windows-msvc.exe`

Example interface:

![Add process exclusion](images/screenshot-2026-04-03-182725.png)

Click `Add` when done.
