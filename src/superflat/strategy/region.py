from functools import cached_property
from pathlib import Path
from typing import override

import structlog
from xdelta3_py import decode, encode

from superflat.paths import region_paths_flatten, region_paths_unflatten
from superflat.utils import exrtact_xz, read_region_file, write_region_file

from .base import Strategy

log = structlog.get_logger()


class RegionFileStrategy(Strategy):
    @cached_property
    @override
    def flatten_paths(self) -> set[Path]:
        return region_paths_flatten(self.save_dir)

    @cached_property
    @override
    def unflatten_paths(self) -> set[Path]:
        return region_paths_unflatten(self.git_dir)

    @override
    def flatten(self, rel_path: Path):
        region_xz = exrtact_xz(rel_path.name)
        if region_xz := exrtact_xz(rel_path.name):
            region_x, region_z = region_xz
        else:
            raise ValueError(f"Cannot exrtact x and z in {rel_path.name}")
        region = read_region_file(self.save_dir / rel_path, region_x, region_z)
        # (self.git_dir / rel_path).mkdir(parents=True, exist_ok=True)
        if region["is_empty"]:
            return
        (self.git_dir / rel_path / "timestamp-header").write_bytes(
            region["timestamp_header"]
        )
        for (chunk_x, chunk_z), nbt in region["chunkxz2nbt"].items():
            target = nbt
            base = self.sfnbt_manager.get(chunk_x, chunk_z)
            if not base:
                raise ValueError(f"Cannot get SFNBT on {chunk_x=}, {chunk_z=}")
            delta = encode(base, target)
            delta_filepath = self.git_dir / rel_path / f"c.{chunk_x}.{chunk_z}.sf.delta"
            delta_filepath.write_bytes(delta)

    @override
    def unflatten(self, rel_path: Path):
        # TODO
        region_xz = exrtact_xz(rel_path.name)
        if region_xz := exrtact_xz(rel_path.name):
            region_x, region_z = region_xz
        else:
            raise ValueError(f"Cannot exrtact x and z in {rel_path.name}")
        timestamp_header = None
        chunkxz2nbt = {}
        for dirpath, _dirnames, filenames in (self.git_dir / rel_path).walk():
            if dirpath != self.git_dir / rel_path:
                log.warn(f"Skipped unrecognized dir: {dirpath}", skip_dir=dirpath)
                continue
            for filename in filenames:
                filepath = dirpath / filename
                if filename == "timestamp-header":
                    timestamp_header = filepath.read_bytes()
                elif (
                    chunk_xz := exrtact_xz(rel_path.name)
                ) and rel_path.suffix == "delta":
                    chunk_x, chunk_z = chunk_xz
                    base = self.sfnbt_manager.get(chunk_x, chunk_z)
                    if not base:
                        raise ValueError(f"Cannot get SFNBT on {chunk_x=}, {chunk_z=}")
                    delta = filepath.read_bytes()
                    target = decode(base, delta)
                    chunkxz2nbt[(chunk_x, chunk_z)] = target
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
