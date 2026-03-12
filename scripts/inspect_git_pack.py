import re
import subprocess
import zlib
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


def id_to_raw_size(git_dir: Path, obj_id: str):
    raw_size = int(
        subprocess.check_output(
            [
                "git",
                f"--git-dir={str(git_dir)}",
                "cat-file",
                "-s",
                obj_id,
            ],
            universal_newlines=True,
            stderr=subprocess.DEVNULL,
        ).strip()
    )
    return raw_size


def pref_str(*nums: int):
    s = ""
    last = None
    for n in nums:
        if last:
            s += f"x{n / last * 100:.2f}%=> "
        s += f"{n:,} "
        last = n

    return s.strip()


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

    total_raw_size = 0
    total_object_size = 0
    total_pack_size = 0
    # print(git_sha1_path_mapping)
    for line in result.splitlines():
        # print(line)
        if mc := re.search(
            r"([0-9a-f]{40}) blob +(\d+) (\d+) \d+ (\d+) ([0-9a-f]{40})", line
        ):
            blob: str = mc[1]
            pack_size = int(mc[3])
            depth = int(mc[4])
            base_blob = mc[5]
            blob_path = sha1_mapping[blob]
            base_blob_path = sha1_mapping[base_blob]
            raw_size = id_to_raw_size(git_path, blob)
            raw_content = subprocess.check_output(
                [
                    "git",
                    f"--git-dir={str(git_path)}",
                    "cat-file",
                    "blob",
                    blob,
                ],
                stderr=subprocess.DEVNULL,
            ).strip()
            object_size = len(zlib.compress(raw_content))

            total_raw_size += raw_size
            total_object_size += object_size
            total_pack_size += pack_size

            if show_commit:
                print(
                    f"DELTA @{depth} {blob_to_commit(git_path, blob)[:8]}:{blob_path} -> {blob_to_commit(git_path, base_blob)[:8]}:{base_blob_path}"
                )
            else:
                print(f"DELTA @{depth} {blob_path} -> {base_blob_path}")
            print(f"\t{pref_str(raw_size, object_size, pack_size)}")
        elif mc := re.search(r"([0-9a-f]{40}) blob +(\d+) (\d+) \d+", line):
            blob = mc[1]
            # object_size = int(mc[2])
            pack_size = int(mc[3])
            blob_path = sha1_mapping[blob]
            raw_size = id_to_raw_size(git_path, blob)

            raw_content = subprocess.check_output(
                [
                    "git",
                    f"--git-dir={str(git_path)}",
                    "cat-file",
                    "blob",
                    blob,
                ],
                stderr=subprocess.DEVNULL,
            ).strip()
            object_size = len(zlib.compress(raw_content))

            total_raw_size += raw_size
            total_object_size += object_size
            total_pack_size += pack_size

            if show_commit:
                print(f"BASE {blob_to_commit(git_path, blob)[:8]}:{blob_path}")
            else:
                print(f"BASE {blob_path}")
            print(f"\t{pref_str(raw_size, object_size, pack_size)}")

    print(
        f"Pack performance: {pref_str(total_raw_size, total_object_size, total_pack_size)}"
    )


def main(git_dir: Path, pack_idx_path: Path):
    inspect_pack(
        git_dir, pack_idx_path, git_sha1_path_mapping(git_dir), show_commit=True
    )


if __name__ == "__main__":
    typer.run(main)
