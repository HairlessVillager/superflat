from pathlib import Path

import structlog
from structlog.contextvars import bound_contextvars

from superflat import crafters as c
from superflat.dumper import Dumper
from superflat.paths import chunk_region_paths_unflatten

log = structlog.get_logger()


class Applicatioin:
    def __init__(
        self,
        save_dir: Path,
        repo_dir: Path,
        dumper: Dumper,
        block_id_mapping: dict[str, str],
    ):
        self.save_dir = save_dir
        self.repo_dir = repo_dir
        self.dumper = dumper
        self.block_id_mapping = block_id_mapping

    def flatten(self):
        crafters: list[c.Crafter] = [
            c.RawFileFlattenCrafter(self.save_dir, self.repo_dir),
            c.GzipNbtFileFlattenCrafter(self.save_dir, self.repo_dir),
            # c.ChunkRegionFileFlattenCrafter(self.save_dir, self.repo_dir, self.dumper),
            c.ChunkRegionFileFlattenCrafterRust(
                self.save_dir, self.repo_dir, self.dumper, self.block_id_mapping
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
