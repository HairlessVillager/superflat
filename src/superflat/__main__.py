from pathlib import Path
from typing import Self

import structlog
import typer
from platformdirs import user_cache_path, user_config_path
from structlog.contextvars import bound_contextvars

from superflat.config import Config
from superflat.dumper import SectionsDumper
from superflat.paths import chunk_region_paths_flatten, chunk_region_paths_unflatten
from superflat.strategy import GzipNbtFileStrategy, RawFileStrategy
from superflat.strategy.region import ChunkRegionFileStrategy, OtherRegionFileStrategy
from superflat.utils import exrtact_xz, get_full_chunks

APP_NAME = "superflat"
app = typer.Typer(name=APP_NAME)
log = structlog.get_logger()


@app.command()
def commit(): ...  # TODO


@app.command()
def restore(): ...  # TODO


@app.command()
def push(): ...  # TODO


@app.command()
def pull(): ...  # TODO


class Superflat:
    def __init__(self, config: Config):
        self.config = config
        self.save_dir = config["save_dir"]
        self.git_dir = config["git_dir"]
        self.cache_dir = config["cache_dir"]
        self.strategy_classes = config["strategy_classes"]
        self.dumper = config["dumper"]

    @classmethod
    def from_name(cls, save_dir: Path, name: str, version: str, seed: int) -> Self:
        cache_dir = user_cache_path(APP_NAME) / version / str(seed)
        return cls(
            {
                "strategy_classes": [
                    RawFileStrategy,
                    GzipNbtFileStrategy,
                    ChunkRegionFileStrategy,
                    OtherRegionFileStrategy,
                ],
                "save_dir": save_dir,
                "git_dir": user_config_path(APP_NAME) / name,
                "cache_dir": cache_dir,
                "dumper": SectionsDumper(seed, cache_dir),
            }
        )

    def flatten(self):
        log.info("Validating")
        if not (self.save_dir / "level.dat").exists():
            raise ValueError(
                f"{self.save_dir / 'level.dat'} not exists, check save_dir"
            )

        base_dir = self.save_dir

        log.info("Collecting full chunks")
        coords = set()
        for dirpath, _dirnames, filenames in base_dir.walk():
            for filename in filenames:
                filepath = dirpath / filename
                rel_path = filepath.relative_to(base_dir)
                if filepath in chunk_region_paths_flatten(base_dir):
                    if region_xz := exrtact_xz(rel_path.name):
                        region_x, region_z = region_xz
                        coords |= get_full_chunks(filepath, region_x, region_z)
                    else:
                        log.warn(
                            f"Cannot exrtact x and z in {rel_path.name}",
                            filepath=filepath,
                        )
        log.info(f"Collected {len(coords)} full chunks", count=len(coords))

        log.info("Generating SFNBTs")
        sfnbt_count = self.dumper.batch_generate(coords)
        log.info(f"Generated {sfnbt_count} SFNBTs", count=sfnbt_count)

        log.info("Flattening files")
        strategies = [t(self.config, coords) for t in self.strategy_classes]
        for dirpath, _dirnames, filenames in base_dir.walk():
            for filename in filenames:
                filepath = dirpath / filename
                rel_path = filepath.relative_to(base_dir)

                with bound_contextvars(filepath=filepath, rel_path=rel_path):
                    log.info(f"Flattening file {rel_path}")
                    for s in strategies:
                        if filepath in s.flatten_paths:
                            strategy_name = type(s).__name__
                            with bound_contextvars(strategy_name=strategy_name):
                                log.debug(f"Using {strategy_name} strategy")
                                s.flatten(rel_path)
                            break
                    else:
                        log.warn(
                            f"Skipped unrecognized file: {rel_path} (full path: {filepath})"
                        )

    def unflatten(self):
        base_dir = self.git_dir

        log.info("Collecting full chunks")
        coords = set()
        for dirpath, _dirnames, filenames in base_dir.walk():
            for filename in filenames:
                filepath = dirpath / filename
                rel_path = filepath.relative_to(base_dir)
                if filepath in chunk_region_paths_unflatten(base_dir):
                    region_xz = exrtact_xz(rel_path.name)
                    if region_xz := exrtact_xz(rel_path.name):
                        region_x, region_z = region_xz
                        coords |= get_full_chunks(filepath, region_x, region_z)
                    else:
                        log.warn(
                            f"Cannot exrtact x and z in {rel_path.name}",
                            filepath=filepath,
                        )
        log.info(f"Collected {len(coords)} full chunks", count=len(coords))

        log.info("Generating SFNBTs")
        sfnbt_count = self.dumper.batch_generate(coords)
        log.info(f"Generated {sfnbt_count} SFNBTs", count=sfnbt_count)

        log.info("Unflattening files")
        strategies = [t(self.config, coords) for t in self.strategy_classes]
        for dirpath, dirnames, filenames in base_dir.walk():
            for filename in filenames:
                filepath = dirpath / filename
                rel_path = filepath.relative_to(base_dir)

                with bound_contextvars(filepath=filepath, rel_path=rel_path):
                    log.info(f"Unflattening file {rel_path}")
                    for s in strategies:
                        if filepath in s.unflatten_paths:
                            strategy_name = type(s).__name__
                            with bound_contextvars(strategy_name=strategy_name):
                                log.debug(f"Using {strategy_name} strategy")
                                s.unflatten(rel_path)
                            break
                    else:
                        log.warn(
                            f"Skipped unrecognized file: {rel_path} (full path: {filepath})"
                        )

    def clear(self): ...

    def delete(self): ...


if __name__ == "__main__":
    save_path = "/home/hlsvillager/.config/hmcl/.minecraft/versions/Fabulously-Optimized-1.21.11/saves/test42"
    restore_path = save_path + "_restored"
    sf = Superflat.from_name(Path(save_path), "test42", "1.21.11", 42)
    sf.flatten()
    sf = Superflat.from_name(Path(restore_path), "test42", "1.21.11", 42)
    sf.unflatten()
