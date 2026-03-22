# rm -rf temp/git
# git init --bare temp/git

# git --git-dir temp/git --work-tree temp/repo add . >/dev/null
# git --git-dir temp/git --work-tree temp/repo commit -m "t0" >/dev/null
# time git --git-dir temp/git repack -d
# git --git-dir temp/git count-objects -vH

# git --git-dir temp/git --work-tree temp/repo2 add . >/dev/null
# git --git-dir temp/git --work-tree temp/repo2 commit -m "t1" >/dev/null
# time git --git-dir temp/git repack -d
# git --git-dir temp/git count-objects -vH

# git --git-dir temp/git --no-pager log --pretty=oneline

import csv
import re
import subprocess
import tempfile
import time
from pathlib import Path

import typer

app = typer.Typer(name="bench")


def command(*cmd: str) -> str:
    try:
        return subprocess.check_output(
            cmd,
            universal_newlines=True,
        )
    except subprocess.CalledProcessError as e:
        print("STDOUT:")
        print(e.stdout)
        print()
        print("STDERR:")
        print(e.stderr)
        print()
        raise


SAVE_DIRS = [
    "/home/hlsvillager/Desktop/test-saves/test42/2026-03-15_15-55-44_test42/test42",
    "/home/hlsvillager/Desktop/test-saves/test42/2026-03-15_16-09-57_test42/test42",
    "/home/hlsvillager/Desktop/test-saves/test42/2026-03-15_16-20-00_test42/test42",
    "/home/hlsvillager/Desktop/test-saves/test42/2026-03-15_16-30-10_test42/test42",
    "/home/hlsvillager/Desktop/test-saves/test42/2026-03-15_16-40-20_test42/test42",
    "/home/hlsvillager/Desktop/test-saves/test42/2026-03-15_16-50-29_test42/test42",
    "/home/hlsvillager/Desktop/test-saves/test42/2026-03-15_17-00-40_test42/test42",
    "/home/hlsvillager/Desktop/test-saves/test42/2026-03-15_17-10-49_test42/test42",
    "/home/hlsvillager/Desktop/test-saves/test42/2026-03-15_18-39-46_test42/test42",
    "/home/hlsvillager/Desktop/test-saves/test42/2026-03-15_18-49-57_test42/test42",
    "/home/hlsvillager/Desktop/test-saves/test42/2026-03-15_19-14-15_test42/test42",
    "/home/hlsvillager/Desktop/test-saves/test42/2026-03-15_19-24-26_test42/test42",
    "/home/hlsvillager/Desktop/test-saves/test42/2026-03-15_19-29-51_test42/test42",
]


@app.command()
def flatten(repo_base_dir: Path) -> list[Path]:
    repo_dirs = []
    for idx, save_dir in enumerate(SAVE_DIRS):
        repo_dir = repo_base_dir / str(idx)
        print(idx, str(save_dir), str(repo_dir))
        start = time.perf_counter()
        command(
            "uv",
            "run",
            "sf",
            "flatten",
            "-s",
            str(save_dir),
            "-r",
            str(repo_dir),
            "-b",
            "minecraft:grass=minecraft:short_grass",  # 1.20.3
            "-b",
            "minecraft:chain=minecraft:iron_chain",  # 1.21.9
        )
        end = time.perf_counter()
        duration = end - start
        print(f"#{idx}, Executed in {duration:.4f}s")
        repo_dirs.append(repo_dir)
    return repo_dirs


@app.command()
def commit_repack(
    repo_base_dir: Path,
    window: int,
    csv_output: bool = typer.Option(False, "--csv"),
):
    csv_path = Path("bench-results.csv")
    write_header = csv_output and not csv_path.exists()

    with (
        tempfile.TemporaryDirectory(delete=True) as git_dir,
    ):
        command("git", "init", "--bare", str(git_dir))
        command("git", "--git-dir", str(git_dir), "config", "gc.auto", "0")
        size = len(list(repo_base_dir.iterdir()))
        for idx in range(size):
            repo_dir = repo_base_dir / str(idx)
            print(idx, repo_dir)
            command(
                "git",
                "--git-dir",
                str(git_dir),
                "--work-tree",
                str(repo_dir),
                "add",
                ".",
            )
            command(
                "git",
                "--git-dir",
                str(git_dir),
                "--work-tree",
                str(repo_dir),
                "commit",
                "-m",
                f"time-{idx}",
            )
            start = time.perf_counter()
            if window == 0:
                command("git", "--git-dir", str(git_dir), "gc", "--aggressive")
            else:
                command(
                    "git",
                    "--git-dir",
                    str(git_dir),
                    "repack",
                    "-a",
                    "-d",
                    "--depth",
                    "4095",
                    "--window",
                    str(window),
                )

            end = time.perf_counter()
            pack_result = command(
                "git",
                "--git-dir",
                str(git_dir),
                "count-objects",
                "-vH",
            )
            print(f"#{idx}")
            print()
            print(pack_result)
            print()
            duration = end - start
            print(f"Executed in {duration:.4f}s")
            print()
            print("-" * 40)
            print()

            if csv_output:
                m = re.search(r"size-pack:\s+([\d.]+)\s+MiB", pack_result)
                size_pack = float(m.group(1)) if m else float("nan")
                with open(csv_path, "a", newline="") as f:
                    writer = csv.writer(f)
                    if write_header:
                        writer.writerow(
                            ["window", "round", "size_pack_mib", "time_cost_s"]
                        )
                        write_header = False
                    writer.writerow([window, idx, size_pack, f"{duration:.4f}"])


@app.command()
def plot(
    csv_path: Path = Path("bench-results.csv"), output: Path = Path("bench-results.png")
):
    import csv as csv_mod
    from collections import defaultdict

    import matplotlib.cm as cm  # pyright: ignore[reportMissingImports]
    import matplotlib.pyplot as plt  # pyright: ignore[reportMissingImports]
    import numpy as np  # pyright: ignore[reportMissingImports]

    data = defaultdict(lambda: {"round": [], "size": [], "time": []})
    with open(csv_path) as f:
        for row in csv_mod.DictReader(f):
            w = int(row["window"])
            data[w]["round"].append(int(row["round"]) - 1)  # 0-indexed
            data[w]["size"].append(float(row["size_pack_mib"]))
            data[w]["time"].append(float(row["time_cost_s"]))

    windows = sorted(data.keys())
    colors = cm.Blues(np.linspace(0.25, 0.95, len(windows)))

    fig, (ax1, ax2) = plt.subplots(2, 1, figsize=(10, 8), sharex=True)

    for i, w in enumerate(windows):
        d = data[w]
        label = f"window={w}"
        ax1.plot(
            d["round"],
            d["size"],
            color=colors[i],
            label=label,
            marker="o",
            markersize=3,
        )
        ax2.plot(
            d["round"],
            d["time"],
            color=colors[i],
            label=label,
            marker="o",
            markersize=3,
        )

    ax1.set_ylabel("size-pack (MiB)")
    ax1.set_title("git repack --depth 4095 --window N  (test42, no terrain)")
    ax1.legend(loc="upper left", fontsize=8)
    ax1.grid(True, alpha=0.3)

    ax2.set_ylabel("time cost (s)")
    ax2.set_xlabel("round")
    ax2.legend(loc="upper left", fontsize=8)
    ax2.grid(True, alpha=0.3)

    plt.tight_layout()
    plt.savefig(output, dpi=150)
    print(f"Saved {output}")


if __name__ == "__main__":
    app()
