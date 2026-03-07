import gzip
import math
import re
import shutil
import zlib
from abc import ABC, abstractmethod
from functools import cached_property
from pathlib import Path
from typing import Self, TypedDict, override

import structlog
import typer
from platformdirs import user_cache_path, user_config_path
from structlog.contextvars import bound_contextvars

from pumpkin_py import normalize_nbt

APP_NAME = "superflat"
log = structlog.get_logger()


def main(log_level: str = "info"):
    structlog.configure(
        wrapper_class=structlog.make_filtering_bound_logger(log_level),
    )
    log.info("Hello from superflat!")


def cli():
    typer.run(main)


SECTOR_SIZE = 4096


class Strategy(ABC):
    def __init__(self, save_dir: Path, git_dir: Path, cache_dir: Path):
        self.save_dir = save_dir
        self.git_dir = git_dir
        self.cache_dir = cache_dir

    @cached_property
    def dimensions_dirs(self) -> set[Path]:
        return {
            self.save_dir,
            self.save_dir / "DIM1",
            self.save_dir / "DIM-1",
            *self.save_dir.glob("dimensions/*/*"),
        }

    @cached_property
    @abstractmethod
    def paths(self) -> set[Path]: ...

    @abstractmethod
    def flatten(self, rel_path: Path): ...

    @abstractmethod
    def unflatten(self, rel_path: Path): ...


class RawFileStrategy(Strategy):
    @cached_property
    @override
    def paths(self) -> set[Path]:
        return {
            self.save_dir / "icon.png",
            *self.save_dir.glob("advancements/*.json"),
            *self.save_dir.glob("stats/*.json"),
        }

    @override
    def flatten(self, rel_path: Path):
        shutil.copy2(self.save_dir / rel_path, self.git_dir / rel_path)

    @override
    def unflatten(self, rel_path: Path):
        shutil.copy2(self.git_dir / rel_path, self.save_dir / rel_path)


class GzipNbtFileStrategy(Strategy):
    @cached_property
    @override
    def paths(self) -> set[Path]:
        return {
            # root
            self.save_dir / "level.dat",
            self.save_dir / "data/idcounts.dat",
            *self.save_dir.glob("data/command_storage_*.dat"),
            *self.save_dir.glob("data/map_*.dat"),
            self.save_dir / "data/scoreboard.dat",
            self.save_dir / "data/stopwatches.dat",
            *self.save_dir.glob("generated/*/structures/*.nbt"),
            *self.save_dir.glob("playerdata/*.dat"),
            # dimensions
            *(
                self.save_dir / dimensions_dir / "data" / dimensions_gzip_nbt_file
                for dimensions_dir in self.dimensions_dirs
                for dimensions_gzip_nbt_file in [
                    "chunks.dat",
                    "raids.dat",
                    "raids_end.dat",
                    "random_sequences.dat",
                    "world_border.dat",
                ]
            ),
        }

    @override
    def flatten(self, rel_path: Path):
        content = (self.save_dir / rel_path).read_bytes()
        content = gzip.decompress(content)
        content = normalize_nbt(content)
        (self.git_dir / rel_path).write_bytes(content)

    @override
    def unflatten(self, rel_path: Path):
        content = (self.git_dir / rel_path).read_bytes()
        content = gzip.decompress(content)
        (self.save_dir / rel_path).write_bytes(content)


class RegionFile(TypedDict):
    region_x: int
    region_z: int
    is_empty: bool
    timestamp_header: bytes
    chunkxz2nbt: dict[tuple[int, int], bytes]


def exrtact_xz(filename: str) -> tuple[int, int] | None:
    mc = re.match(r"[cr]\.(-?\d+)\.(-?\d+)\.[a-zA-Z]+", filename)
    if not mc:
        return None
    x = int(mc.group(1))
    z = int(mc.group(2))
    return (x, z)


def parse_region_file(
    region_filepath: Path, region_x: int, region_z: int
) -> RegionFile:
    class Chunk(TypedDict):
        data: bytes | None

        local_x: int
        local_z: int
        region_x: int
        region_z: int
        chunk_x: int
        chunk_z: int
        timestamp: int
        index: int
        source: Path
        offset_sectors: int
        size_sectors: int
        compression_type: int | None

    with bound_contextvars(filename=region_filepath, x=region_x, z=region_z):
        with region_filepath.open("rb") as region_reader:
            log.debug("Parsing header")
            chunks: list[Chunk] = []
            locations_raw = memoryview(region_reader.read(0x1000))
            timestamps_raw = memoryview(region_reader.read(0x1000))
            if len(locations_raw) == 0 and len(timestamps_raw) == 0:
                return {
                    "region_x": region_x,
                    "region_z": region_z,
                    "is_empty": True,
                    "timestamp_header": b"",
                    "chunkxz2nbt": {},
                }
            elif len(locations_raw) != 0x1000 or len(timestamps_raw) != 0x1000:
                raise RuntimeError(
                    f"Region file {region_filepath} has truncated header: {len(locations_raw)} + {len(timestamps_raw)}"
                )
            for i in range(1024):
                x = i % 32
                z = i // 32
                chunk_x = region_x * 32 + x
                chunk_z = region_z * 32 + z

                loc = locations_raw[i * 4 : (i + 1) * 4]
                offset = int.from_bytes(loc[:3], byteorder="big")
                size = int.from_bytes(loc[3:], byteorder="big")
                ts = int.from_bytes(
                    timestamps_raw[i * 4 : (i + 1) * 4], byteorder="big"
                )
                if offset == 0 and size == 0:
                    continue
                if offset < 2:
                    raise RuntimeError(
                        f"Region file {region_filepath} has invalid sector at index: {i}; sector {offset} overlaps with header"
                    )
                if size == 0:
                    raise RuntimeError(
                        f"Region file {region_filepath} has an invalid sector at index: {i}; size has to be > 0"
                    )
                if offset < 2:
                    raise RuntimeError(
                        f"Region file {region_filepath} has invalid sector at index: {i}; sector {offset} overlaps with header"
                    )
                chunks.append(
                    {
                        "index": i,
                        "region_x": region_x,
                        "region_z": region_z,
                        "local_x": x,
                        "local_z": z,
                        "offset_sectors": offset,
                        "size_sectors": size,
                        "source": region_filepath,
                        "timestamp": ts,
                        "compression_type": None,
                        "data": None,
                        "chunk_x": chunk_x,
                        "chunk_z": chunk_z,
                    }
                )
            chunks.sort(key=lambda c: c["offset_sectors"])

            log.debug("Extracting chunks")
            for chunk in chunks:
                seek_offset = region_reader.seek(chunk["offset_sectors"] * SECTOR_SIZE)
                if seek_offset != chunk["offset_sectors"] * SECTOR_SIZE:
                    raise RuntimeError(
                        f"Region file {region_filepath} has an invalid sector at index: {chunk['index']}; sector {chunk['size_sectors']} is out of bounds"
                    )
                raw = memoryview(
                    region_reader.read(chunk["size_sectors"] * SECTOR_SIZE)
                )
                data_length = int.from_bytes(raw[:4], byteorder="big")
                compression_type = int.from_bytes(raw[4:5], byteorder="big")
                compressed_data = raw[5 : 5 + data_length]
                if compression_type == 2:
                    data = zlib.decompress(compressed_data)
                elif compression_type == 129:
                    raise NotImplementedError("mcc file is not supported")
                else:
                    raise NotImplementedError(
                        f"Unsupportd compression_type: {compression_type}"
                    )
                data = normalize_nbt(data)
                chunk["compression_type"] = compression_type
                chunk["data"] = data

            return {
                "region_x": region_x,
                "region_z": region_z,
                "is_empty": False,
                "timestamp_header": timestamps_raw.tobytes(),
                "chunkxz2nbt": {
                    (chunk["chunk_x"], chunk["chunk_z"]): chunk["data"]
                    for chunk in chunks
                    if chunk["data"] is not None
                },
            }


def write_region_file(region: RegionFile, region_filepath: Path):
    if region["is_empty"]:
        region_filepath.write_bytes(b"")
        return

    region_x = region["region_x"]
    region_z = region["region_z"]

    locations = bytearray(4096)
    timestamps = bytearray(region["timestamp_header"])
    if len(timestamps) != 4096:
        raise ValueError(f"Invalid timestamp length: {len(timestamps)} != 4096")

    current_sector = 2
    chunk_data_buffer = bytearray()

    for (chunk_x, chunk_z), nbt in region["chunkxz2nbt"].items():
        # basic parameters
        local_x = chunk_x - (region_x * 32)
        local_z = chunk_z - (region_z * 32)
        index = local_x + local_z * 32
        if not (0 <= local_x < 32 and 0 <= local_z < 32):
            raise ValueError(
                f"Chunk outside region boundary: chunk_x={chunk_x}, chunk_z={chunk_z},"
            )

        # chunk datapack
        compression_type = 2
        compressed = zlib.compress(nbt)
        content_length = len(compressed) + 1
        chunk_payload = (
            content_length.to_bytes(4, "big")
            + compression_type.to_bytes(1, "big")
            + compressed
        )

        # count sectors
        total_size = len(chunk_payload)
        sectors_needed = math.ceil(total_size / SECTOR_SIZE)
        if sectors_needed >= 256:
            raise NotImplementedError(
                f"Chunk too large for standard mca format: size = {total_size}, {sectors_needed} >= 256",
            )

        # update location header
        loc_offset = index * 4
        locations[loc_offset : loc_offset + 3] = current_sector.to_bytes(3, "big")
        locations[loc_offset + 3] = sectors_needed

        # write chunk datapack and align to sectors
        padding_size = (sectors_needed * SECTOR_SIZE) - total_size
        chunk_data_buffer.extend(chunk_payload)
        chunk_data_buffer.extend(b"\x00" * padding_size)

        current_sector += sectors_needed

    content = locations + timestamps + chunk_data_buffer
    region_filepath.write_bytes(content)


class RegionFileStrategy(Strategy):
    @cached_property
    @override
    def paths(self) -> set[Path]:
        return {
            file
            for dimensions_dir in self.dimensions_dirs
            for dimensions_region_file_parent in ["entities", "poi", "region"]
            for file in (
                self.save_dir / dimensions_dir / dimensions_region_file_parent
            ).glob("r.*.*.mca")
        }

    @override
    def flatten(self, rel_path: Path):
        # TODO
        region_xz = exrtact_xz(rel_path.name)
        if region_xz := exrtact_xz(rel_path.name):
            region_x, region_z = region_xz
        else:
            raise ValueError(f"Cannot exrtact x and z in {rel_path.name}")
        region = parse_region_file(self.save_dir / rel_path, region_x, region_z)
        # (self.git_dir / rel_path).mkdir(parents=True, exist_ok=True)
        if region["is_empty"]:
            return
        (self.git_dir / rel_path / "timestamp-header").write_bytes(
            region["timestamp_header"]
        )
        for (chunk_x, chunk_z), nbt in region["chunkxz2nbt"].items():
            (self.git_dir / rel_path / f"c.{chunk_x}.{chunk_z}.nbt").write_bytes(nbt)

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
                elif chunk_xz := exrtact_xz(rel_path.name):
                    chunk_x, chunk_z = chunk_xz
                    chunkxz2nbt[(chunk_x, chunk_z)] = filepath.read_bytes()
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
