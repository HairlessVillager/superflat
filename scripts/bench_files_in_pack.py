import os
import subprocess
import tempfile
from pathlib import Path

import typer
from inspect_git_pack import inspect_pack


def pack_files(file_paths: list[Path]):
    for p in file_paths:
        if not p.is_file():
            raise ValueError(f"{p} is not a file")
    valid_files = [p.resolve() for p in file_paths if p.is_file()]

    with tempfile.TemporaryDirectory(delete=False) as tmp_dir:
        tmp_path = Path(tmp_dir)
        git_path = tmp_path / "repo"
        git_path.mkdir()

        subprocess.run(
            ["git", "init", "--bare"], cwd=git_path, check=True, capture_output=True
        )

        env = os.environ.copy()
        env["GIT_OBJECT_DIRECTORY"] = str(git_path / "objects")

        git_sha1_path_mapping = {}
        for p in valid_files:
            result = subprocess.run(
                ["git", "hash-object", "-w", str(p)],
                env=env,
                cwd=git_path,
                check=True,
                capture_output=True,
                text=True,
            )
            git_sha1_path_mapping[result.stdout.strip()] = Path(p)

        pack_input = "\n".join(git_sha1_path_mapping)
        base_name = "bench-test"
        pack_result = subprocess.run(
            ["git", "pack-objects", base_name],
            input=pack_input,
            env=env,
            cwd=git_path,
            check=True,
            capture_output=True,
            text=True,
        )

        pack_prefix = pack_result.stdout.strip().split("\n")[-1]
        pack_idx_path = git_path / f"{base_name}-{pack_prefix}.idx"
        # print(f"Write pack file to {pack_idx_path}")
        inspect_pack(git_path, pack_idx_path, git_sha1_path_mapping, show_commit=False)


if __name__ == "__main__":
    typer.run(pack_files)
