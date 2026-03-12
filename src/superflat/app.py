from pathlib import Path
from typing import Callable, Iterable

import structlog
from structlog.contextvars import bound_contextvars

from superflat.config import Config
from superflat.dumper import SectionsDumper, ZeroDumper
from superflat.executors import Executor
from superflat.paths import chunk_region_paths_flatten, chunk_region_paths_unflatten
from superflat.utils import Coords, exrtact_xz, get_full_chunks

log = structlog.get_logger()


class Applicatioin:
    def __init__(self, config: Config):
        self.save_dir = config["save_dir"]
        self.repo_dir = config["repo_dir"]
        if config["terrain"]:
            self.dumper = SectionsDumper(config["seed"], config["cache_dir"])
        else:
            self.dumper = ZeroDumper()

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

        full_chunks = self.collect_full_chunks(
            self.save_dir, chunk_region_paths_flatten
        )
        self.dumper.batch_generate(full_chunks)

        executors: list[Executor] = [
            RawFileFlattenExecutor(),
            GzipNbtFileFlattenExecutor(),
            ChunkRegionFileFlattenExecutor(self.dumper, full_chunks),
            # TODO: EntitiesRegionFile executors
            OtherRegionFileFlattenExecutor(),
        ]
        for e in executors:
            with bound_contextvars(executor=type(e).__name__):
                log.info("Collecting tasks")
                e.collect_task(self.save_dir, self.repo_dir)

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
                e.collect_task(self.save_dir, self.repo_dir)

        for e in executors:
            with bound_contextvars(executor=type(e).__name__):
                log.info("Unflattening files")
                e.batch_execute()

    def clear(self): ...

    def delete(self): ...
