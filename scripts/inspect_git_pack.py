import re
import subprocess
from functools import cache
from pathlib import Path

import typer


@cache
def get_commit_to_blob_mapping(git_path: Path):
    commits = subprocess.check_output(
        [
            "git",
            "--git-dir",
            str(git_path),
            "rev-list",
            "--all",
        ],
        universal_newlines=True,
        stderr=subprocess.DEVNULL,
    )
    mapping = {}
    for commit in commits.splitlines():
        result = subprocess.check_output(
            ["git", "--git-dir", str(git_path), "ls-tree", "-r", commit],
            universal_newlines=True,
            stderr=subprocess.DEVNULL,
        )
        blobs = set()
        for line in result.splitlines():
            if mc := re.search(r"\d+ blob ([0-9a-f]{40})", line):
                blob = mc[1]
                blobs.add(blob)
        mapping[commit] = blobs
    # breakpoint()
    return mapping


@cache
def git_sha1_path_mapping(git_dir: Path):
    objects = {}
    result = subprocess.check_output(
        ["git", f"--git-dir={str(git_dir)}", "rev-list", "--objects", "--all"],
        universal_newlines=True,
        stderr=subprocess.DEVNULL,
    )

    for line in result.strip().split("\n"):
        parts = line.split(maxsplit=1)
        if not parts:
            continue

        sha1 = parts[0]
        path = parts[1] if len(parts) > 1 else ""

        if path:
            objects[sha1] = path

    return objects


def blob_to_commit(git_path: Path, blob: str):
    for commit, blobs in get_commit_to_blob_mapping(git_path).items():
        if blob in blobs:
            return commit
    else:
        raise ValueError(f"Cannot find blob {blob} in all commits")


def inspect_pack(
    git_path: Path, pack_idx_path: Path, sha1_mapping: dict[str, str], show_commit: bool
):
    result = subprocess.check_output(
        [
            "git",
            f"--git-dir={str(git_path)}",
            "verify-pack",
            "-v",
            str(pack_idx_path),
        ],
        universal_newlines=True,
        stderr=subprocess.DEVNULL,
    )

    total_original_size = 0
    total_compressed_size = 0
    # print(git_sha1_path_mapping)
    for line in result.splitlines():
        # print(line)
        if mc := re.search(
            r"([0-9a-f]{40}) blob +(\d+) (\d+) \d+ (\d+) ([0-9a-f]{40})", line
        ):
            blob = mc[1]
            compressed_size = int(mc[2])
            # size_in_packfile = int(mc[3])
            depth = int(mc[4])
            base_blob = mc[5]
            blob_path = sha1_mapping[blob]
            base_blob_path = sha1_mapping[base_blob]
            uncompressed_size = int(
                subprocess.check_output(
                    [
                        "git",
                        f"--git-dir={str(git_path)}",
                        "cat-file",
                        "-s",
                        blob,
                    ],
                    universal_newlines=True,
                    stderr=subprocess.DEVNULL,
                ).strip()
            )
            pcent = compressed_size / uncompressed_size * 100 - 100
            if show_commit:
                print(
                    f"DELTA {blob_to_commit(git_path, blob)[:8]}:{blob_path} -> {blob_to_commit(git_path, base_blob)[:8]}:{base_blob_path} @{depth} {uncompressed_size:,} {pcent:+.2f}%"
                )
            else:
                print(
                    f"DELTA {blob} -> {blob_path} @{depth} {uncompressed_size:,} {pcent:+.2f}%"
                )
            total_original_size += uncompressed_size
            total_compressed_size += compressed_size
        elif mc := re.search(r"([0-9a-f]{40}) blob +(\d+) (\d+) \d+", line):
            blob = mc[1]
            compressed_size = int(mc[2])
            # size_in_packfile = int(mc[3])
            blob_path = sha1_mapping[blob]
            if show_commit:
                print(
                    f"BASE {blob_to_commit(git_path, blob)[:8]}:{blob_path} {compressed_size:,}"
                )
            else:
                print(f"BASE {blob_path} {compressed_size:,}")

            total_original_size += compressed_size
            total_compressed_size += compressed_size

    print(
        f"Pack performance: {total_compressed_size / total_original_size * 100 - 100:+.2f}% ({total_original_size:,}B -> {total_compressed_size:,}B)"
    )


def main(git_dir: Path, pack_idx_path: Path):
    inspect_pack(
        git_dir, pack_idx_path, git_sha1_path_mapping(git_dir), show_commit=True
    )


if __name__ == "__main__":
    typer.run(main)
