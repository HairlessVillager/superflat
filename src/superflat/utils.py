import math
import re
import zlib
from pathlib import Path
from typing import TypedDict

import structlog
from structlog.contextvars import bound_contextvars

from pumpkin_py import is_chunk_status_full, normalize_nbt

log = structlog.get_logger()

SECTOR_SIZE = 4096

type Coords = set[tuple[int, int]]


class RegionFile(TypedDict):
    region_x: int
    region_z: int
    is_empty: bool
    timestamp_header: bytes
    chunkxz2nbt: dict[tuple[int, int], bytes]


def exrtact_xz(filename: str) -> tuple[int, int] | None:
    mc = re.search(r"\.(-?\d+)\.(-?\d+)\.", filename)
    if not mc:
        return None
    x = int(mc.group(1))
    z = int(mc.group(2))
    return (x, z)


def get_full_chunks(region_filepath: Path, region_x: int, region_z: int) -> Coords:
    region = read_region_file(region_filepath, region_x, region_z)
    return {
        (chunk_x, chunk_z)
        for (chunk_x, chunk_z), nbt in region["chunkxz2nbt"].items()
        if is_chunk_status_full(nbt)
    }


def read_region_file(region_filepath: Path, region_x: int, region_z: int) -> RegionFile:
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

    with bound_contextvars(region_filepath=region_filepath, x=region_x, z=region_z):
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
        write_bin(region_filepath, b"")
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
    write_bin(region_filepath, content)


def write_bin(filepath: Path, data: bytes | bytearray):
    filepath.parent.mkdir(parents=True, exist_ok=True)
    filepath.write_bytes(data)
