import subprocess
from pathlib import Path


def get_commit_topo(git_dir: Path, commit: str) -> dict[str, list[str]]:
    lines = subprocess.check_output(
        [
            "git",
            "--git-dir",
            str(git_dir),
            "rev-list",
            "--parents",
            "--topo-order",
            commit,
        ],
        universal_newlines=True,
        stderr=subprocess.DEVNULL,
    )
    child2parents = {}
    for line in lines.splitlines():
        if not line:
            continue
        child, *parents = line.split()
        child2parents[child] = parents
    return child2parents


def get_commit_tree(git_dir: Path, commit: str) -> dict[Path, str]:
    lines = subprocess.check_output(
        [
            "git",
            "--git-dir",
            str(git_dir),
            "ls-tree",
            "--format=%(objectname) %(path)",
            "-r",
            commit,
        ],
        universal_newlines=True,
        stderr=subprocess.DEVNULL,
    )
    path2blob = {}
    for line in lines:
        blob_id, path = line.split()
        path2blob[path] = blob_id
    return path2blob


def repack(git_dir: Path, commit: str):
    commit_child2parents = get_commit_topo(git_dir, commit)
    commit2blobs = {c: get_commit_tree(git_dir, c) for c in commit_child2parents.keys()}
    paths = set(
        path for path2blob in commit2blobs.values() for path in path2blob.keys()
    )
    # use midx command to avoid run time disk space peak
    subprocess.check_call(
        ["git", "--git-dir", str(git_dir), "pack-objects", "superflat"]
    )
