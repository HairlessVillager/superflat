from pathlib import Path

import structlog

from superflat import _superflat
from superflat.crafters.base import Crafter, collect_valid_paths
from superflat.dumper import Dumper
from superflat.paths import (
    other_region_paths_flatten,
    other_region_paths_unflatten,
)
from superflat.utils import (
    exrtact_xz,
    read_region_file,
    write_bin,
    write_region_file,
)

log = structlog.get_logger()


class ChunkRegionFileFlattenCrafterRust(Crafter):
    """Rust implementation of ChunkRegionFileFlattenCrafter"""

    def __init__(
        self,
        save_dir: Path,
        repo_dir: Path,
        dumper: Dumper,
        block_id_mapping: dict[str, str] | None = None,
    ):
        self.save_dir = save_dir
        self.repo_dir = repo_dir
        self.dumper = dumper
        self.block_id_mapping = block_id_mapping or {}

    def __call__(self) -> list[Path]:
        processed = _superflat.pumpkin.chunk_region_flatten(
            self.save_dir,
            self.repo_dir,
            self.block_id_mapping,
        )
        return [Path(p) for p in processed]


class ChunkRegionFileUnflattenCrafterRust(Crafter):
    """Rust implementation of chunk region unflatten"""

    def __init__(self, save_dir: Path, repo_dir: Path):
        self.save_dir = save_dir
        self.repo_dir = repo_dir

    def __call__(self) -> list[Path]:
        processed = _superflat.pumpkin.chunk_region_unflatten(
            self.save_dir,
            self.repo_dir,
        )
        return [Path(p) for p in processed]


class OtherRegionFileFlattenCrafter(Crafter):
    def __init__(self, save_dir: Path, repo_dir: Path):
        self.save_dir = save_dir
        self.repo_dir = repo_dir

    def __call__(self) -> list[Path]:
        rel_paths = collect_valid_paths(self.save_dir, other_region_paths_flatten)
        for rel_path in rel_paths:
            region_xz = exrtact_xz(rel_path.name)
            if region_xz := exrtact_xz(rel_path.name):
                region_x, region_z = region_xz
            else:
                raise ValueError(f"Cannot exrtact x and z in {rel_path.name}")
            region = read_region_file(self.save_dir / rel_path, region_x, region_z)
            if region["is_empty"]:
                continue
            write_bin(
                self.repo_dir / rel_path / "timestamp-header",
                region["timestamp_header"],
            )
            for (chunk_x, chunk_z), nbt in region["chunkxz2nbt"].items():
                nbt_filepath = self.repo_dir / rel_path / f"c.{chunk_x}.{chunk_z}.nbt"
                write_bin(nbt_filepath, nbt)
        return rel_paths


class OtherRegionFileUnflattenCrafter(Crafter):
    def __init__(self, save_dir: Path, repo_dir: Path):
        self.save_dir = save_dir
        self.repo_dir = repo_dir

    def __call__(self) -> list[Path]:
        rel_paths = collect_valid_paths(self.repo_dir, other_region_paths_unflatten)
        processed_rel_paths = []
        for rel_path in rel_paths:
            # simple check
            if rel_path.name != "timestamp-header":
                raise ValueError(f"Invalid rel_path: {rel_path}")

            rel_path = rel_path.parent

            region_xz = exrtact_xz(rel_path.name)
            if region_xz := exrtact_xz(rel_path.name):
                region_x, region_z = region_xz
            else:
                raise ValueError(f"Cannot exrtact x and z in {rel_path.name}")

            timestamp_header = None
            chunkxz2nbt = {}
            for dirpath, _dirnames, filenames in (self.repo_dir / rel_path).walk():
                for filename in filenames:
                    filepath = dirpath / filename
                    if filename == "timestamp-header":
                        timestamp_header = filepath.read_bytes()
                        processed_rel_paths.append(filepath.relative_to(self.repo_dir))
                    elif (
                        chunk_xz := exrtact_xz(filepath.name)
                    ) and filepath.suffix == ".nbt":
                        chunk_x, chunk_z = chunk_xz
                        nbt = filepath.read_bytes()
                        chunkxz2nbt[(chunk_x, chunk_z)] = nbt
                        processed_rel_paths.append(filepath.relative_to(self.repo_dir))
                    else:
                        log.warn(
                            f"Skipped unrecognized file: {rel_path} (full path: {filepath})"
                        )

            if not timestamp_header:
                raise RuntimeError(f"Timestamp header file not found for {rel_path}")

            write_region_file(
                {
                    "region_x": region_x,
                    "region_z": region_z,
                    "is_empty": False,
                    "timestamp_header": timestamp_header,
                    "chunkxz2nbt": chunkxz2nbt,
                },
                self.save_dir / rel_path,
            )
        return processed_rel_paths
