import gzip
from pathlib import Path
from typing import Annotated

import structlog
import typer

from pumpkin_py import seed_from_level
from superflat.app import Applicatioin

APP_NAME = "superflat"
typer_app = typer.Typer(name=APP_NAME)
log = structlog.get_logger()


@typer_app.command()
def flatten(
    save_dir: Annotated[
        Path, typer.Option("--save-dir", "-s", help="Path to your save")
    ],
    repo_dir: Annotated[
        Path,
        typer.Option("--repo-dir", "-r", help="Path to the flatten Git repository"),
    ],
    cache_dir: Annotated[
        Path,
        typer.Option("--cache-dir", "-c", help="Path to cache the sections dumps"),
    ],
):
    save_dir = save_dir.resolve()
    repo_dir = repo_dir.resolve()
    cache_dir = cache_dir.resolve()

    level_dat_filepath = save_dir / "level.dat"
    if not level_dat_filepath.exists():
        raise ValueError(f"{save_dir / 'level.dat'} not exists")
    seed = seed_from_level(gzip.decompress(level_dat_filepath.read_bytes()))

    log.info(
        f"Flattening save from {save_dir} to {repo_dir}, cache on {cache_dir}",
        save_dir=save_dir,
        repo_dir=repo_dir,
        cache_dir=cache_dir,
    )
    app = Applicatioin(
        {
            "cache_dir": cache_dir,
            "repo_dir": repo_dir,
            "save_dir": save_dir,
            "seed": seed,
        }
    )
    app.flatten()


@typer_app.command()
def unflatten(
    save_dir: Annotated[
        Path, typer.Option("--save-dir", "-s", help="Path to your save")
    ],
    repo_dir: Annotated[
        Path,
        typer.Option("--repo-dir", "-r", help="Path to the flatten Git repository"),
    ],
    cache_dir: Annotated[
        Path,
        typer.Option("--cache-dir", "-c", help="Path to cache the sections dumps"),
    ],
):
    save_dir = save_dir.resolve()
    repo_dir = repo_dir.resolve()
    cache_dir = cache_dir.resolve()

    level_dat_filepath = repo_dir / "level.dat"
    seed = seed_from_level(level_dat_filepath.read_bytes())

    log.info(
        f"Unflattening save from {repo_dir} to {save_dir}, cache on {cache_dir}",
        save_dir=save_dir,
        repo_dir=repo_dir,
        cache_dir=cache_dir,
    )
    app = Applicatioin(
        {
            "cache_dir": cache_dir,
            "repo_dir": repo_dir,
            "save_dir": save_dir,
            "seed": seed,
        }
    )
    app.unflatten()
