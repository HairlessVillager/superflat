from pathlib import Path
from typing import Callable, Iterable

import structlog
import typer
from platformdirs import user_cache_path, user_config_path
from structlog.contextvars import bound_contextvars

from superflat.config import Config
from superflat.dumper import SectionsDumper
from superflat.executors import Executor
from superflat.paths import chunk_region_paths_flatten, chunk_region_paths_unflatten
from superflat.utils import Coords, exrtact_xz, get_full_chunks

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
        self.save_dir = config["save_dir"]
        self.git_dir = config["git_dir"] or user_config_path(APP_NAME) / config["name"]
        cache_dir = config["cache_dir"] or user_cache_path(APP_NAME) / config[
            "version"
        ] / str(config["seed"])
        self.dumper = SectionsDumper(config["seed"], cache_dir)

    def collect_full_chunks(
        self, base_dir: Path, pf: Callable[[Path], Iterable[Path]]
    ) -> Coords:
        log.info("Collecting full chunks")
        coords = set()
        for dirpath, _dirnames, filenames in base_dir.walk():
            for filename in filenames:
                filepath = dirpath / filename
                rel_path = filepath.relative_to(base_dir)
                if filepath in pf(base_dir):
                    if region_xz := exrtact_xz(rel_path.name):
                        region_x, region_z = region_xz
                        coords |= get_full_chunks(filepath, region_x, region_z)
                    else:
                        log.warn(
                            f"Cannot exrtact x and z in {rel_path.name}",
                            filepath=filepath,
                        )
        log.info(f"Collected {len(coords)} full chunks", count=len(coords))
        return coords

    def flatten(self):
        from superflat.executors import (
            ChunkRegionFileFlattenExecutor,
            GzipNbtFileFlattenExecutor,
            OtherRegionFileFlattenExecutor,
            RawFileFlattenExecutor,
        )

        log.info("Validating")
        if not (self.save_dir / "level.dat").exists():
            raise ValueError(
                f"{self.save_dir / 'level.dat'} not exists, check save_dir"
            )

        full_chunks = self.collect_full_chunks(
            self.save_dir, chunk_region_paths_flatten
        )
        self.dumper.batch_generate(full_chunks)

        executors: list[Executor] = [
            RawFileFlattenExecutor(),
            GzipNbtFileFlattenExecutor(),
            ChunkRegionFileFlattenExecutor(self.dumper, full_chunks),
            OtherRegionFileFlattenExecutor(),
        ]
        for e in executors:
            with bound_contextvars(executor=type(e).__name__):
                log.info("Collecting tasks")
                e.collect_task(self.save_dir, self.git_dir)

        for e in executors:
            with bound_contextvars(executor=type(e).__name__):
                log.info("Flattening files")
                e.batch_execute()

    def unflatten(self):
        from superflat.executors import (
            ChunkRegionFileUnflattenExecutor,
            GzipNbtFileUnflattenExecutor,
            OtherRegionFileUnlattenExecutor,
            RawFileUnflattenExecutor,
        )

        full_chunks = self.collect_full_chunks(
            self.save_dir, chunk_region_paths_unflatten
        )
        self.dumper.batch_generate(full_chunks)

        executors: list[Executor] = [
            RawFileUnflattenExecutor(),
            GzipNbtFileUnflattenExecutor(),
            ChunkRegionFileUnflattenExecutor(self.dumper, full_chunks),
            OtherRegionFileUnlattenExecutor(),
        ]
        for e in executors:
            with bound_contextvars(executor=type(e).__name__):
                log.info("Collecting tasks")
                e.collect_task(self.save_dir, self.git_dir)

        for e in executors:
            with bound_contextvars(executor=type(e).__name__):
                log.info("Unflattening files")
                e.batch_execute()

    def clear(self): ...

    def delete(self): ...


if __name__ == "__main__":
    wd = Path("/home/hlsvillager/Desktop/superflat")
    sf = Superflat(
        {
            "cache_dir": Path(wd / "./temp/cache"),
            "git_dir": Path(wd / "./temp/git"),
            "name": "test42",
            # "save_dir": Path(
            #     wd / "./temp/saves/2026-03-08_19-25-54_test42/test42"
            # ),  # t0
            "save_dir": Path(
                wd / "./temp/saves/2026-03-08_19-27-12_test42/test42"
            ),  # t1
            "seed": 42,
            "version": "1.21.11",
        }
    )
    log.info("Flattening")
    sf.flatten()
    sf = Superflat(
        {
            "cache_dir": Path(wd / "./temp/cache"),
            "git_dir": Path(wd / "./temp/git"),
            "name": "test42",
            "save_dir": Path(wd / "./temp/restore"),
            "seed": 42,
            "version": "1.21.11",
        }
    )
    log.info("Unflattening")
    sf.unflatten()
