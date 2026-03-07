from pathlib import Path
from typing import Self

import structlog
import typer
from platformdirs import user_cache_path, user_config_path
from structlog.contextvars import bound_contextvars

from superflat.config import Config
from superflat.paths import region_paths_flatten, region_paths_unflatten
from superflat.sfnbt import SFNBTManager
from superflat.strategy import GzipNbtFileStrategy, RawFileStrategy
from superflat.strategy.region import RegionFileStrategy
from superflat.utils import exrtact_xz, get_full_chunks

APP_NAME = "superflat"
log = structlog.get_logger()


def main(log_level: str = "info"):
    structlog.configure(
        wrapper_class=structlog.make_filtering_bound_logger(log_level),
    )
    log.info("Hello from superflat!")


def cli():
    typer.run(main)


class Superflat:
    def __init__(self, config: Config):
        self.save_dir = config["save_dir"]
        self.git_dir = config["git_dir"]
        self.cache_dir = config["cache_dir"]
        self.strategies = [t(config) for t in config["strategy_classes"]]
        self.sfnbt_manager = config["sfnbt_manager"]

        # simple validation
        if not (self.save_dir / "level.dat").exists():
            raise ValueError(
                f"{self.save_dir / 'level.dat'} not exists, check save_dir"
            )

    @classmethod
    def from_name(cls, save_dir: Path, name: str, version: str, seed: int) -> Self:
        cache_dir = user_cache_path(APP_NAME) / version / str(seed)
        return cls(
            {
                "strategy_classes": [
                    RawFileStrategy,
                    GzipNbtFileStrategy,
                    RegionFileStrategy,
                ],
                "save_dir": save_dir,
                "git_dir": user_config_path(APP_NAME) / name,
                "cache_dir": cache_dir,
                "sfnbt_manager": SFNBTManager(seed, cache_dir),
            }
        )

    def flatten(self):
        base_dir = self.save_dir

        log.info("Collecting full chunks")
        coords = []
        for dirpath, _dirnames, filenames in base_dir.walk():
            for filename in filenames:
                filepath = dirpath / filename
                rel_path = filepath.relative_to(base_dir)
                if filepath in region_paths_flatten(base_dir):
                    region_xz = exrtact_xz(rel_path.name)
                    if region_xz := exrtact_xz(rel_path.name):
                        region_x, region_z = region_xz
                        coords.extend(get_full_chunks(filepath, region_x, region_z))
                    else:
                        log.warn(
                            f"Cannot exrtact x and z in {rel_path.name}",
                            filepath=filepath,
                        )
        log.info(f"Collected {len(coords)} full chunks", count=len(coords))

        log.info("Generating SFNBTs")
        sfnbt_count = self.sfnbt_manager.batch_generate(coords)
        log.info(f"Generated {sfnbt_count} SFNBTs", count=sfnbt_count)

        log.info("Flattening files")
        for dirpath, _dirnames, filenames in base_dir.walk():
            for filename in filenames:
                filepath = dirpath / filename
                rel_path = filepath.relative_to(base_dir)

                with bound_contextvars(filepath=filepath, rel_path=rel_path):
                    log.info(f"Flattening file {rel_path}")
                    for s in self.strategies:
                        if filepath in s.flatten_paths:
                            strategy_name = type(s).__name__
                            with bound_contextvars(strategy_name=strategy_name):
                                log.debug(f"Using {strategy_name} strategy")
                                s.flatten(rel_path)
                    else:
                        log.warn(
                            f"Skipped unrecognized file: {rel_path} (full path: {filepath})"
                        )

    def unflatten(self):
        base_dir = self.git_dir

        log.info("Collecting full chunks")
        coords = []
        for dirpath, _dirnames, filenames in base_dir.walk():
            for filename in filenames:
                filepath = dirpath / filename
                rel_path = filepath.relative_to(base_dir)
                if filepath in region_paths_unflatten(base_dir):
                    region_xz = exrtact_xz(rel_path.name)
                    if region_xz := exrtact_xz(rel_path.name):
                        region_x, region_z = region_xz
                        coords.extend(get_full_chunks(filepath, region_x, region_z))
                    else:
                        log.warn(
                            f"Cannot exrtact x and z in {rel_path.name}",
                            filepath=filepath,
                        )
        log.info(f"Collected {len(coords)} full chunks", count=len(coords))

        log.info("Generating SFNBTs")
        sfnbt_count = self.sfnbt_manager.batch_generate(coords)
        log.info(f"Generated {sfnbt_count} SFNBTs", count=sfnbt_count)

        for dirpath, _dirnames, filenames in base_dir.walk():
            for filename in filenames:
                filepath = dirpath / filename
                rel_path = filepath.relative_to(base_dir)

                with bound_contextvars(filepath=filepath, rel_path=rel_path):
                    log.info(f"Unflattening file {rel_path}")
                    for s in self.strategies:
                        if filepath in s.flatten_paths:
                            strategy_name = type(s).__name__
                            with bound_contextvars(strategy_name=strategy_name):
                                log.debug(f"Using {strategy_name} strategy")
                                s.flatten(rel_path)
                    else:
                        log.warn(
                            f"Skipped unrecognized file: {rel_path} (full path: {filepath})"
                        )

    def clear(self): ...

    def delete(self): ...


if __name__ == "__main__":
    main()
