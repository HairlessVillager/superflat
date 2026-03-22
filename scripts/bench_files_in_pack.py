import os
import random
import subprocess
import tempfile
from pathlib import Path
from typing import Annotated

import typer
from inspect_git_pack import inspect_pack

# Import generation helpers from sibling script
from gen_test_files import _apply_edits, _make_base_document

app = typer.Typer(help="Benchmark git pack-objects compression on a set of files.")

# ---------------------------------------------------------------------------
# Shared internals
# ---------------------------------------------------------------------------

def _git_init_bare(path: Path) -> None:
    subprocess.run(
        ["git", "init", "--bare"], cwd=path, check=True, capture_output=True
    )


def _pack_objects(
    git_path: Path,
    file_paths: list[Path],
    window: int,
    depth: int,
) -> None:
    """Write blobs, pack them, then print verify-pack stats."""
    env = os.environ.copy()
    env["GIT_OBJECT_DIRECTORY"] = str(git_path / "objects")

    sha1_path: dict[str, Path] = {}
    for p in file_paths:
        result = subprocess.run(
            ["git", "hash-object", "-w", str(p)],
            env=env,
            cwd=git_path,
            check=True,
            capture_output=True,
            text=True,
        )
        sha1_path[result.stdout.strip()] = p

    pack_result = subprocess.run(
        [
            "git", "pack-objects",
            f"--window={window}",
            f"--depth={depth}",
            "bench-pack",
        ],
        input="\n".join(sha1_path),
        env=env,
        cwd=git_path,
        check=True,
        capture_output=True,
        text=True,
    )

    pack_hash = pack_result.stdout.strip().split("\n")[-1]
    idx_path = git_path / f"bench-pack-{pack_hash}.idx"
    inspect_pack(git_path, idx_path, sha1_path, show_commit=False)


# ---------------------------------------------------------------------------
# Sub-commands
# ---------------------------------------------------------------------------

@app.command("pack")
def pack_files(
    file_paths: Annotated[list[Path], typer.Argument(help="Files to pack.")],
    window: Annotated[int, typer.Option("--window", "-w", help="Delta search window.")] = 10,
    depth: Annotated[int, typer.Option("--depth", "-d", help="Maximum delta chain depth.")] = 50,
) -> None:
    """Pack explicit files and report delta-compression statistics."""
    valid = [p.resolve() for p in file_paths if p.is_file()]
    if not valid:
        typer.echo("No valid files provided.", err=True)
        raise typer.Exit(1)

    with tempfile.TemporaryDirectory(delete=False) as tmp:
        git_path = Path(tmp) / "repo"
        git_path.mkdir()
        _git_init_bare(git_path)
        _pack_objects(git_path, valid, window, depth)


@app.command("gen")
def gen_and_bench(
    num_files: Annotated[int, typer.Option("--num-files", "-n", min=1)] = 100,
    seed: Annotated[int, typer.Option("--seed", "-s")] = 42,
    base_lines: Annotated[int, typer.Option("--base-lines", "-l", min=5)] = 80,
    edits_per_step: Annotated[int, typer.Option("--edits", "-e", min=1)] = 6,
    window: Annotated[int, typer.Option("--window", "-w")] = 100,
    depth: Annotated[int, typer.Option("--depth", "-d")] = 4095,
    verbose: Annotated[bool, typer.Option("--verbose", "-v")] = False,
) -> None:
    """Generate a synthetic editing history then benchmark pack compression.

    Files are produced by gen_test_files._make_base_document / _apply_edits,
    written to a temp directory, and fed directly into git pack-objects.
    """
    typer.echo(
        f"Generating {num_files} files  seed={seed}  base_lines={base_lines}"
        f"  edits/step={edits_per_step}  window={window}  depth={depth}"
    )

    rng = random.Random(seed)
    lines = _make_base_document(rng, base_lines)

    with tempfile.TemporaryDirectory(delete=False) as tmp:
        tmp_path = Path(tmp)
        git_path = tmp_path / "repo"
        docs_path = tmp_path / "docs"
        git_path.mkdir()
        docs_path.mkdir()
        _git_init_bare(git_path)

        width = len(str(num_files - 1))
        file_paths: list[Path] = []

        for i in range(num_files):
            if i > 0:
                lines = _apply_edits(lines, rng, edits_per_step)
            p = docs_path / f"doc_{i:0{width}d}.txt"
            p.write_text("\n".join(lines) + "\n", encoding="utf-8")
            file_paths.append(p)
            if verbose:
                typer.echo(f"  {p.name}  lines={len(lines):4d}  bytes={p.stat().st_size:6d}")

        typer.echo(f"\nPacking {len(file_paths)} objects  window={window}  depth={depth}")
        _pack_objects(git_path, file_paths, window, depth)


if __name__ == "__main__":
    app()
