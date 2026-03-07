from pathlib import Path
from typing import Self

import structlog
import typer
from platformdirs import user_cache_path, user_config_path
from structlog.contextvars import bound_contextvars

from superflat.strategy import GzipNbtFileStrategy, RawFileStrategy, Strategy
from superflat.strategy.region import RegionFileStrategy

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
    def __init__(
        self,
        strategy_classes: list[type[Strategy]],
        save_dir: Path,
        git_dir: Path,
        cache_dir: Path,
    ):
        self.save_dir = save_dir
        self.git_dir = git_dir
        self.cache_dir = cache_dir
        self.strategies = [t(save_dir, git_dir, cache_dir) for t in strategy_classes]

        # simple validation
        if not (self.save_dir / "level.dat").exists():
            raise ValueError(
                f"{self.save_dir / 'level.dat'} not exists, check save_dir"
            )

    @classmethod
    def from_name(cls, save_dir: Path, name: str, version: str, seed: int) -> Self:
        return cls(
            strategy_classes=[RawFileStrategy, GzipNbtFileStrategy, RegionFileStrategy],
            save_dir=save_dir,
            git_dir=user_config_path(APP_NAME) / name,
            cache_dir=user_cache_path(APP_NAME) / version / str(seed),
        )

    def flatten(self):
        for dirpath, _dirnames, filenames in self.save_dir.walk():
            for filename in filenames:
                filepath = dirpath / filename
                rel_path = filepath.relative_to(self.save_dir)

                with bound_contextvars(filepath=filepath, rel_path=rel_path):
                    log.info(f"Processing file {rel_path}")
                    for s in self.strategies:
                        if filepath in s.paths:
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
