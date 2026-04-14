import csv
import re
import subprocess
from datetime import datetime
from pathlib import Path

TRAIL_ID = datetime.now().isoformat()

SAVE_DIRS = [
    # "/home/hlsvillager/Desktop/test-saves/test42v2/2026-03-22_15-37-16_test42/test42",
    # "/home/hlsvillager/Desktop/test-saves/test42v2/2026-03-22_15-47-58_test42/test42",
    # "/home/hlsvillager/Desktop/test-saves/test42v2/2026-03-22_16-05-26_test42/test42",
    # "/home/hlsvillager/Desktop/test-saves/test42v2/2026-03-22_16-16-07_test42/test42",
    # "/home/hlsvillager/Desktop/test-saves/test42v2/2026-03-22_16-26-49_test42/test42",
    # "/home/hlsvillager/Desktop/test-saves/test42v2/2026-03-22_16-37-30_test42/test42",
    # "/home/hlsvillager/Desktop/test-saves/test42v2/2026-03-22_16-48-05_test42/test42",
    # "/home/hlsvillager/Desktop/test-saves/test42v2/2026-03-22_16-58-56_test42/test42",
    # "/home/hlsvillager/Desktop/test-saves/test42v2/2026-03-22_17-09-31_test42/test42",
    # "/home/hlsvillager/Desktop/test-saves/test42v2/2026-03-22_17-20-21_test42/test42",
    "/home/hlsvillager/Desktop/test-saves/test42v2e/2026-03-22_15-37-16_test42/test42",
    "/home/hlsvillager/Desktop/test-saves/test42v2e/2026-03-22_15-47-58_test42/test42",
    "/home/hlsvillager/Desktop/test-saves/test42v2e/2026-03-22_22-24-25_test42/test42",
    "/home/hlsvillager/Desktop/test-saves/test42v2e/2026-03-22_23-05-04_test42/test42",
    "/home/hlsvillager/Desktop/test-saves/test42v2e/2026-03-23_10-23-10_test42/test42",
]

GIT_REPO = "/tmp/sf.git"

CSV_FILEPATH = Path("trails.csv")


def main():
    subprocess.run(["rm", "-rf", GIT_REPO], check=False)
    subprocess.run(["git", "init", "--bare", GIT_REPO], check=True)

    for index, save_dir in enumerate(SAVE_DIRS):
        print(f"#{index}: {save_dir}")

        print("commit")
        cmd = [
            "./target/release/sf",
            "commit",
            save_dir,
            GIT_REPO,
            "-b",
            "main",
            "-m",
            str(index),
            "--mc-version",
            "1.21.11",
        ]

        if index == 0:
            cmd.append("--init")

        start = datetime.now()
        subprocess.run(cmd, check=True)
        end = datetime.now()
        commit_duration = end - start

        print("repack")
        # cmd = [
        #     "git",
        #     "--git-dir",
        #     GIT_REPO,
        #     "repack",
        #     "--depth",
        #     "4095",
        #     "--window",
        #     "2",
        #     "-a",
        #     "-d",
        # ]
        cmd = [
            "git",
            "--git-dir",
            GIT_REPO,
            "repack",
            "--geometric=8",
            "-d",
            "--write-midx",
        ]

        start = datetime.now()
        subprocess.run(cmd, check=True)
        end = datetime.now()
        repack_duration = end - start

        print("count objects")
        cmd = [
            "git",
            "--git-dir",
            GIT_REPO,
            "count-objects",
            "-vH",
        ]
        pack_result = subprocess.check_output(cmd, universal_newlines=True)
        m = re.search(r"size-pack:\s+([\d.]+)\s+MiB", pack_result)
        size_pack = float(m.group(1)) if m else float("nan")

        first_write = not CSV_FILEPATH.exists()
        with CSV_FILEPATH.open("a", newline="") as f:
            writer = csv.DictWriter(f, ["ID", "index", "commit", "repack", "size"])
            if first_write:
                writer.writeheader()
            writer.writerow(
                {
                    "ID": TRAIL_ID,
                    "index": index,
                    "commit": commit_duration.total_seconds(),
                    "repack": repack_duration.total_seconds(),
                    "size": size_pack,
                }
            )


if __name__ == "__main__":
    main()
