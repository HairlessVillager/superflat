import re
import subprocess
from functools import cache
from pathlib import Path

import typer
from inspect_git_pack import get_commit_to_blob_mapping, git_sha1_path_mapping


@cache
def get_id_pack_size_mapping(git_dir: Path, idx_path: Path):
    result = subprocess.check_output(
        [
            "git",
            "--git-dir",
            str(git_dir),
            "verify-pack",
            "-v",
            str(idx_path),
        ],
        universal_newlines=True,
        stderr=subprocess.DEVNULL,
    )
    mapping = {}
    for line in result.splitlines():
        # print(line)
        if mc := re.search(
            r"([0-9a-f]{40}) blob +(\d+) (\d+) \d+ (\d+) ([0-9a-f]{40})", line
        ):
            blob = mc[1]
            size_in_packfile = int(mc[3])
            mapping[blob] = size_in_packfile
        elif mc := re.search(r"([0-9a-f]{40}) blob +(\d+) (\d+) \d+", line):
            blob = mc[1]
            size_in_packfile = int(mc[3])
            mapping[blob] = size_in_packfile
    return mapping


def bench_git_pack(
    git_dir: Path,
    idx_path: Path,
    commit: str,
    prefix_path: str | None = None,
):
    commit_blob_mapping = get_commit_to_blob_mapping(git_dir)
    id_size_mapping = get_id_pack_size_mapping(git_dir, idx_path)
    id_size_mapping = {
        k: v for k, v in id_size_mapping.items() if k in commit_blob_mapping[commit]
    }
    if not prefix_path:
        prefix_path = ""

    id_path_mapping = git_sha1_path_mapping(git_dir)
    path_size_mapping = {
        Path(id_path_mapping[id]): size for id, size in id_size_mapping.items()
    }
    prefix = Path(prefix_path)
    prefix_size_mapping = {}
    for path, size in path_size_mapping.items():
        if not path.is_relative_to(prefix):
            continue
        suffix = path.relative_to(prefix)
        key = suffix.parts[0]
        count = prefix_size_mapping.get(key, 0)
        count += size
        prefix_size_mapping[key] = count

    total = sum(prefix_size_mapping.values())
    for prefix, size in sorted(
        prefix_size_mapping.items(), key=lambda x: x[1], reverse=True
    ):
        print(f"{prefix}: {size:,} {size / total * 100:.2f}%")
    print()
    print(f"Total: {total:,} 100.00%")


if __name__ == "__main__":
    typer.run(bench_git_pack)
