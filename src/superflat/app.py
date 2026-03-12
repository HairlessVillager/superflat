from pathlib import Path
from typing import Callable, Iterable

import structlog
from structlog.contextvars import bound_contextvars

from superflat import crafters as c
from superflat.dumper import Dumper
from superflat.paths import chunk_region_paths_unflatten
from superflat.utils import Coords, exrtact_xz, get_full_chunks

log = structlog.get_logger()


class Applicatioin:
    def __init__(self, save_dir: Path, repo_dir: Path, dumper: Dumper):
        self.save_dir = save_dir
        self.repo_dir = repo_dir
        self.dumper = dumper

    def collect_full_chunks(
        self, base_dir: Path, pf: Callable[[Path], Iterable[Path]]
    ) -> Coords:
        log.info("Collecting full chunks")
        raise NotImplementedError()
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
        # full_chunks = self.collect_full_chunks(
        #     self.save_dir, chunk_region_paths_flatten
        # )
        # self.dumper.batch_generate(full_chunks)

        crafters: list[c.Crafter] = [
            c.RawFileFlattenCrafter(self.save_dir, self.repo_dir),
            c.GzipNbtFileFlattenCrafter(self.save_dir, self.repo_dir),
            # c.ChunkRegionFileFlattenCrafter(self.save_dir, self.repo_dir, self.dumper),
            c.ChunkRegionFileFlattenCrafterRust(
                self.save_dir, self.repo_dir, self.dumper
            ),
            c.OtherRegionFileFlattenCrafter(self.save_dir, self.repo_dir),
        ]
        flatten_paths = set()
        for crafter in crafters:
            with bound_contextvars(executor=crafter.__class__.__name__):
                log.info("Flattening files")
                paths = crafter()
                for p in paths:
                    flatten_paths.add(p)

        for dirname, _, filenames in self.save_dir.walk():
            for filename in filenames:
                rel_path = (dirname / filename).relative_to(self.save_dir)
                if rel_path not in flatten_paths:
                    log.warn(f"Skipped {rel_path}")

    def unflatten(self):
        full_chunks = self.collect_full_chunks(
            self.repo_dir, chunk_region_paths_unflatten
        )
        self.dumper.batch_generate(full_chunks)

        crafters: list[c.Crafter] = [
            c.RawFileUnflattenCrafter(self.save_dir, self.repo_dir),
            c.GzipNbtFileUnflattenCrafter(self.save_dir, self.repo_dir),
            c.ChunkRegionFileUnflattenCrafter(
                self.save_dir, self.repo_dir, self.dumper
            ),
            c.OtherRegionFileUnflattenCrafter(self.save_dir, self.repo_dir),
        ]
        flatten_paths = set()
        for crafter in crafters:
            with bound_contextvars(executor=crafter.__class__.__name__):
                log.info("Unflattening files")
                paths = crafter()
                for p in paths:
                    flatten_paths.add(p)

        for dirname, _, filenames in self.save_dir.walk():
            for filename in filenames:
                rel_path = (dirname / filename).relative_to(self.save_dir)
                if rel_path not in flatten_paths:
                    log.warn(f"Skipped {rel_path}")
