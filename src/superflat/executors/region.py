from pathlib import Path

import structlog

from pumpkin_py import (
    chunk_to_sections_other,
    sections_other_to_chunk,
)
from superflat.dumper import SectionsDumper
from superflat.executors.base import Executor, collect_valid_paths
from superflat.paths import (
    chunk_region_paths_flatten,
    chunk_region_paths_unflatten,
    other_region_paths_flatten,
    other_region_paths_unflatten,
)
from superflat.utils import (
    Coords,
    exrtact_xz,
    read_region_file,
    write_bin,
    write_region_file,
)

log = structlog.get_logger()


class ChunkRegionFileFlattenExecutor(Executor):
    def __init__(self, dumper: SectionsDumper, full_chunks: Coords):
        self.dumper = dumper
        self.full_chunks = full_chunks

    def collect_task(self, save_dir: Path, git_dir: Path):
        self.save_dir = save_dir
        self.git_dir = git_dir
        self.rel_paths = collect_valid_paths(save_dir, chunk_region_paths_flatten)

    def batch_execute(self):
        for rel_path in self.rel_paths:
            self.flatten(rel_path)

    def flatten(self, rel_path: Path):
        region_xz = exrtact_xz(rel_path.name)
        if region_xz := exrtact_xz(rel_path.name):
            region_x, region_z = region_xz
        else:
            raise ValueError(f"Cannot exrtact x and z in {rel_path.name}")
        region = read_region_file(self.save_dir / rel_path, region_x, region_z)
        if region["is_empty"]:
            return
        write_bin(
            self.git_dir / rel_path / "timestamp-header", region["timestamp_header"]
        )

        log.debug("Writing deltas")
        for (chunk_x, chunk_z), nbt in region["chunkxz2nbt"].items():
            if (chunk_x, chunk_z) not in self.full_chunks:
                continue

            sections, other = chunk_to_sections_other(nbt)
            target = sections
            base = self.dumper.get(chunk_x, chunk_z)
            if not base:
                raise ValueError(f"Cannot get SFNBT on {chunk_x=}, {chunk_z=}")
            delta = bytes([a ^ b for a, b in zip(base, target)])

            other_filepath = (
                self.git_dir / rel_path / "other" / f"c.{chunk_x}.{chunk_z}.nbt"
            )
            write_bin(other_filepath, other)
            delta_filepath = (
                self.git_dir / rel_path / "sections" / f"c.{chunk_x}.{chunk_z}.delta"
            )
            write_bin(delta_filepath, delta)

        log.debug("Deltas written")


class ChunkRegionFileUnflattenExecutor(Executor):
    def __init__(self, dumper: SectionsDumper, full_chunks: Coords):
        self.dumper = dumper
        self.full_chunks = full_chunks

    def collect_task(self, save_dir: Path, git_dir: Path):
        self.save_dir = save_dir
        self.git_dir = git_dir
        self.rel_paths = collect_valid_paths(git_dir, chunk_region_paths_unflatten)

    def batch_execute(self):
        for rel_path in self.rel_paths:
            self.unflatten(rel_path)

    def unflatten(self, rel_path: Path):
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
        chunkxz2sections: dict[tuple[int, int], bytes] = {}
        chunkxz2other: dict[tuple[int, int], bytes] = {}
        for dirpath, _dirnames, filenames in (self.git_dir / rel_path).walk():
            for filename in filenames:
                filepath = dirpath / filename
                if filename == "timestamp-header":
                    timestamp_header = filepath.read_bytes()
                elif chunk_xz := exrtact_xz(filepath.name):
                    chunk_x, chunk_z = chunk_xz
                    if (
                        filepath.parent.name == "sections"
                        and filepath.suffix == ".delta"
                    ):
                        base = self.dumper.get(chunk_x, chunk_z)
                        if not base:
                            raise ValueError(
                                f"Cannot get SFNBT on {chunk_x=}, {chunk_z=}"
                            )
                        delta = filepath.read_bytes()
                        target = bytes([a ^ b for a, b in zip(base, delta)])
                        chunkxz2sections[(chunk_x, chunk_z)] = target
                    elif filepath.parent.name == "other" and filepath.suffix == ".nbt":
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

        if not timestamp_header:
            raise RuntimeError(f"Timestamp header file not found for {rel_path}")

        chunkxz2nbt = {}
        for key in set((*chunkxz2sections, *chunkxz2other)):
            sections = chunkxz2sections[key]
            other = chunkxz2other[key]
            nbt = sections_other_to_chunk(sections, other)
            chunkxz2nbt[key] = nbt

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


class OtherRegionFileFlattenExecutor(Executor):
    def collect_task(self, save_dir: Path, git_dir: Path):
        self.save_dir = save_dir
        self.git_dir = git_dir
        self.rel_paths = collect_valid_paths(save_dir, other_region_paths_flatten)

    def batch_execute(self):
        for rel_path in self.rel_paths:
            self.flatten(rel_path)

    def flatten(self, rel_path: Path):
        region_xz = exrtact_xz(rel_path.name)
        if region_xz := exrtact_xz(rel_path.name):
            region_x, region_z = region_xz
        else:
            raise ValueError(f"Cannot exrtact x and z in {rel_path.name}")
        region = read_region_file(self.save_dir / rel_path, region_x, region_z)
        if region["is_empty"]:
            return
        write_bin(
            self.git_dir / rel_path / "timestamp-header", region["timestamp_header"]
        )
        for (chunk_x, chunk_z), nbt in region["chunkxz2nbt"].items():
            nbt_filepath = self.git_dir / rel_path / f"c.{chunk_x}.{chunk_z}.nbt"
            write_bin(nbt_filepath, nbt)


class OtherRegionFileUnlattenExecutor(Executor):
    def collect_task(self, save_dir: Path, git_dir: Path):
        self.save_dir = save_dir
        self.git_dir = git_dir
        self.rel_paths = collect_valid_paths(git_dir, other_region_paths_unflatten)

    def batch_execute(self):
        for rel_path in self.rel_paths:
            self.unflatten(rel_path)

    def unflatten(self, rel_path: Path):
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
        for dirpath, _dirnames, filenames in (self.git_dir / rel_path).walk():
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
