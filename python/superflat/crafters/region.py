from pathlib import Path
from typing import TypedDict

import structlog

from superflat import _superflat
from superflat.crafters.base import Crafter, collect_valid_paths
from superflat.dumper import Dumper
from superflat.paths import (
    chunk_region_paths_unflatten,
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


class ChunkRegionFileUnflattenCrafter(Crafter):
    def __init__(self, save_dir: Path, repo_dir: Path, dumper: Dumper):
        self.save_dir = save_dir
        self.repo_dir = repo_dir
        self.dumper = dumper

    def __call__(self) -> list[Path]:
        class Task(TypedDict):
            other: bytes
            sections_delta: bytes
            sections_dump: bytes
            region_xz: tuple[int, int]
            chunk_xz: tuple[int, int]

        class TaskResult(TypedDict):
            chunk_nbt: bytes

        rel_paths = collect_valid_paths(self.repo_dir, chunk_region_paths_unflatten)

        tasks: list[Task] = []
        regionxz2header: dict[tuple[int, int], bytes] = {}
        regionxz2relpath: dict[tuple[int, int], Path] = {}
        regionxz2taskresults: dict[tuple[int, int], list[tuple[Task, TaskResult]]] = {}

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

            chunkxz2delta: dict[tuple[int, int], bytes] = {}
            chunkxz2other: dict[tuple[int, int], bytes] = {}
            for dirpath, _dirnames, filenames in (self.repo_dir / rel_path).walk():
                for filename in filenames:
                    filepath = dirpath / filename
                    if filename == "timestamp-header":
                        timestamp_header = filepath.read_bytes()
                        regionxz2header[region_xz] = timestamp_header
                    elif chunk_xz := exrtact_xz(filepath.name):
                        chunk_x, chunk_z = chunk_xz
                        if (
                            filepath.parent.name == "sections"
                            and filepath.suffix == ".delta"
                        ):
                            delta = filepath.read_bytes()
                            chunkxz2delta[(chunk_x, chunk_z)] = delta
                        elif (
                            filepath.parent.name == "other"
                            and filepath.suffix == ".nbt"
                        ):
                            other = filepath.read_bytes()
                            chunkxz2other[(chunk_x, chunk_z)] = other
                        else:
                            log.warn(
                                f"Skipped unrecognized file: {rel_path} (full path: {filepath})"
                            )
                    else:
                        log.warn(
                            f"Skipped unrecognized file: {rel_path} (full path: {filepath})"
                        )

            regionxz2relpath[region_xz] = rel_path
            for chunk_xz in set((*chunkxz2delta, *chunkxz2other)):
                chunk_x, chunk_z = chunk_xz
                sections_dump = self.dumper.get(chunk_x, chunk_z)
                if not sections_dump:
                    raise ValueError(f"Cannot get SFNBT on {chunk_x=}, {chunk_z=}")
                tasks.append(
                    {
                        "region_xz": region_xz,
                        "chunk_xz": chunk_xz,
                        "other": chunkxz2other[chunk_xz],
                        "sections_delta": chunkxz2delta[chunk_xz],
                        "sections_dump": sections_dump,
                    }
                )

        log.info(f"Collected {len(tasks)} tasks, running", count=len(tasks))
        results: list[TaskResult] = [
            {"chunk_nbt": e}
            for e in _superflat.pumpkin.chunk_region_decode_batch(
                [t["other"] for t in tasks],
                [t["sections_delta"] for t in tasks],
                [t["sections_dump"] for t in tasks],
                self.dumper.compressed,
            )
        ]

        log.info("Write files")
        for task, result in zip(tasks, results, strict=True):
            region_xz = task["region_xz"]
            lst = regionxz2taskresults.get(region_xz, list())
            lst.append((task, result))
            regionxz2taskresults[region_xz] = lst
        for region_xz, rel_path in regionxz2relpath.items():
            region_x, region_z = region_xz
            task_results = regionxz2taskresults[region_xz]
            timestamp_header = regionxz2header[region_xz]
            write_region_file(
                {
                    "region_x": region_x,
                    "region_z": region_z,
                    "is_empty": False,
                    "timestamp_header": timestamp_header,
                    "chunkxz2nbt": {
                        task["chunk_xz"]: result["chunk_nbt"]
                        for task, result in task_results
                    },
                },
                self.save_dir / rel_path,
            )
        return rel_paths


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
                    elif (
                        chunk_xz := exrtact_xz(filepath.name)
                    ) and filepath.suffix == ".nbt":
                        chunk_x, chunk_z = chunk_xz
                        nbt = filepath.read_bytes()
                        chunkxz2nbt[(chunk_x, chunk_z)] = nbt
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
        return rel_paths
