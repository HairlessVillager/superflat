import json
import shutil
from datetime import datetime
from pathlib import Path
from typing import Annotated, TypedDict

import structlog
import typer
from git import Repo
from platformdirs import user_cache_path, user_config_path

from superflat.app import Applicatioin

APP_NAME = "superflat"
typer_app = typer.Typer(name=APP_NAME)
log = structlog.get_logger()


class Save(TypedDict):
    name: str
    seed: int
    version: str
    save_dir: str


def get_git_dir(save: Save):
    return user_config_path(APP_NAME) / save["name"]


def get_cache_dir(save: Save):
    return user_cache_path(APP_NAME) / save["version"] / str(save["seed"])


def load_save(name: str) -> Save:
    saves_filepath = user_config_path(APP_NAME) / "saves.json"
    if saves_filepath.exists():
        with saves_filepath.open("r") as f:
            saves: list[Save] = json.load(f)
    else:
        raise ValueError("saves.json not exists")

    for s in saves:
        if s["name"] == name:
            return s
    else:
        raise ValueError(f"Save named {name} not found in saves.json")


@typer_app.command()
def init(
    name: str,
    save_dir: Path,
    seed: Annotated[int, typer.Option()],
    version: Annotated[str, typer.Option()],
):
    save: Save = {
        "name": name,
        "seed": seed,
        "version": version,
        "save_dir": str(save_dir),
    }
    log.info("Reading saves json")
    saves_filepath = user_config_path(APP_NAME) / "saves.json"
    if saves_filepath.exists():
        with saves_filepath.open("r") as f:
            saves: list[Save] = json.load(f)
    else:
        saves_filepath.parent.mkdir(parents=True, exist_ok=True)
        saves = []

    log.info("Checking independent")
    for s in saves:
        if s["name"] == name:
            raise ValueError(f"Save named {name} already exists")

    log.info("Checking level.dat file")
    if not (save_dir / "level.dat").exists():
        raise ValueError(f"{save_dir / 'level.dat'} not exists")

    log.info("Initializing Git repo")
    git_dir = get_git_dir(save)
    git_dir.mkdir()
    repo = Repo.init(git_dir)

    log.info("Commiting with README.md")
    readme_filepath = git_dir / "README.md"
    readme_filepath.write_text(f"""\
# {name}

- Version: Minecraft Java Edition {version}
- Seed: {seed}

*Powered by Superflat*
""")
    repo.index.add(readme_filepath)
    repo.index.commit("Superflat Initialization")

    log.info("Writing saves json")
    saves.append(save)
    with saves_filepath.open("w") as f:
        json.dump(saves, f)


@typer_app.command()
def commit(name: str):
    log.info("Loading save")
    save = load_save(name)
    git_dir = get_git_dir(save)
    save_dir = Path(save["save_dir"])

    log.info("Removing old files in repo")
    reserved_paths = {".git", "README.md"}
    for p in git_dir.iterdir():
        if p.name not in reserved_paths:
            if p.is_file():
                p.unlink()
            elif p.is_dir():
                shutil.rmtree(p)
            else:
                log.warn(f"Cannot remove {p}, skipped")

    log.info(
        f"Flattening save from {save_dir} to {git_dir}",
        save_dir=save_dir,
        git_dir=git_dir,
    )
    app = Applicatioin(
        {
            "cache_dir": get_cache_dir(save),
            "git_dir": git_dir,
            "save_dir": save_dir,
            "seed": save["seed"],
        }
    )
    app.flatten()

    log.info("Commiting")
    repo = Repo(git_dir)
    repo.index.add(".")
    repo.index.commit(f"Update at {datetime.now().isoformat(' ')}")


@typer_app.command()
def restore(name: str):
    log.info("Loading save")
    save = load_save(name)
    git_dir = get_git_dir(save)
    save_dir = Path(save["save_dir"])

    log.info("Backuping old save")
    backup_path = save_dir.parent / f"{save_dir}.{datetime.now().isoformat()}.backup"
    save_dir.replace(backup_path)
    log.info(f"Backuped old save to {backup_path}")

    log.info(
        f"Unflattening save from {save_dir} to {git_dir}",
        save_dir=save["save_dir"],
        git_dir=git_dir,
    )
    app = Applicatioin(
        {
            "cache_dir": get_cache_dir(save),
            "git_dir": git_dir,
            "save_dir": Path(save["save_dir"]),
            "seed": save["seed"],
        }
    )
    app.unflatten()
