from pathlib import Path

import structlog
from structlog.contextvars import bound_contextvars

from superflat.dumper import Dumper
from superflat.executors import Executor
from superflat.paths import chunk_region_paths_flatten, chunk_region_paths_unflatten

log = structlog.get_logger()


class Applicatioin:
    def __init__(self, save_dir: Path, repo_dir: Path, dumper: Dumper):
        self.save_dir = save_dir
        self.repo_dir = repo_dir
        self.dumper = dumper

    def flatten(self):
        from superflat.executors import (
            ChunkRegionFileFlattenExecutor,
            GzipNbtFileFlattenExecutor,
            OtherRegionFileFlattenExecutor,
            RawFileFlattenExecutor,
        )

        full_chunks = self.dumper.collect_full_chunks(
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
        flatten_paths = set()
        for e in executors:
            with bound_contextvars(executor=type(e).__name__):
                log.info("Collecting tasks")
                paths = e.collect_task(self.save_dir, self.repo_dir)
                for p in paths:
                    flatten_paths.add(p)

        for dirname, _, filenames in self.save_dir.walk():
            for filename in filenames:
                rel_path = (dirname / filename).relative_to(self.save_dir)
                if rel_path not in flatten_paths:
                    log.warn(f"Skipped {rel_path}")

        # exit(1)
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

        full_chunks = self.dumper.collect_full_chunks(
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
