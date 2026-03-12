import gzip
from pathlib import Path
from typing import Annotated

import structlog
import typer
from superflat_pumpkin import seed_from_level

from superflat.app import Applicatioin

APP_NAME = "superflat"
typer_app = typer.Typer(name=APP_NAME)
log = structlog.get_logger()

OptionSaveDir = Annotated[
    Path, typer.Option("--save-dir", "-s", help="Path to your save")
]
OptionRepoDir = Annotated[
    Path,
    typer.Option("--repo-dir", "-r", help="Path to the flatten Git repository"),
]
OptionCacheDir = Annotated[
    Path,
    typer.Option("--cache-dir", "-c", help="Path to cache the sections dumps"),
]
OptionTerrain = Annotated[
    bool,
    typer.Option(
        "--terrain/--no-terrain",
        help="Use sections dumps from natural terrain (powered by Pumpkin-MC). Disable this when world has no natural terrain (eg. superflat world, UGC save)",
    ),
]


@typer_app.command()
def flatten(
    save_dir: OptionSaveDir,
    repo_dir: OptionRepoDir,
    cache_dir: OptionCacheDir,
    terrain: OptionTerrain = False,
    # NOTE on 20260312: set default to False because this option cannot save delta space for now.
    # Pumpkin-MC terrain generation is still work in progress, and there will be a day to use True as default.
):
    save_dir = save_dir.resolve()
    repo_dir = repo_dir.resolve()
    cache_dir = cache_dir.resolve()

    level_dat_filepath = save_dir / "level.dat"
    if not level_dat_filepath.exists():
        raise ValueError(f"{save_dir / 'level.dat'} not exists")
    seed = seed_from_level(gzip.decompress(level_dat_filepath.read_bytes()))

    log.info(
        f"Flattening save from {save_dir} to {repo_dir}, cache on {cache_dir}, "
        + ("using terrain" if terrain else "disable terrain"),
        save_dir=save_dir,
        repo_dir=repo_dir,
        cache_dir=cache_dir,
        terrain=terrain,
    )
    app = Applicatioin(
        {
            "cache_dir": cache_dir,
            "repo_dir": repo_dir,
            "save_dir": save_dir,
            "seed": seed,
            "terrain": terrain,
        }
    )
    app.flatten()


@typer_app.command()
def unflatten(
    save_dir: OptionSaveDir,
    repo_dir: OptionRepoDir,
    cache_dir: OptionCacheDir,
    terrain: OptionTerrain = False,
    # NOTE on 20260312: set default to False because this option cannot save delta space for now.
    # Pumpkin-MC terrain generation is still work in progress, and there will be a day to use True as default.
):
    save_dir = save_dir.resolve()
    repo_dir = repo_dir.resolve()
    cache_dir = cache_dir.resolve()

    level_dat_filepath = repo_dir / "level.dat"
    seed = seed_from_level(level_dat_filepath.read_bytes())

    log.info(
        f"Unflattening save from {repo_dir} to {save_dir}, cache on {cache_dir}"
        + ("using terrain" if terrain else "disable terrain"),
        save_dir=save_dir,
        repo_dir=repo_dir,
        cache_dir=cache_dir,
        terrain=terrain,
    )
    app = Applicatioin(
        {
            "cache_dir": cache_dir,
            "repo_dir": repo_dir,
            "save_dir": save_dir,
            "seed": seed,
            "terrain": terrain,
        }
    )
    app.unflatten()
