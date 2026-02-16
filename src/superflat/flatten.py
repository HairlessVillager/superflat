import re
import zlib
from pathlib import Path
from typing import TypedDict

import structlog
from fastnbt_btree import normalize  # pyright: ignore[reportAttributeAccessIssue]

SECTOR_SIZE = 4096

log = structlog.get_logger()


class Chunk(TypedDict):
    data: bytes | None

    local_x: int
    local_z: int
    region_x: int
    region_z: int
    timestamp: int
    index: int
    source: Path
    offset_sectors: int
    size_sectors: int
    compression_type: int | None


def flatten(
    src: Path, dst: Path, region_x: int | None = None, region_z: int | None = None
):
    if not src.is_file():
        raise ValueError("src must be a file path")
    if dst.exists():
        if not dst.is_dir():
            raise ValueError("dst must be a dir path")
    else:
        dst.mkdir(parents=True)

    if not region_x or not region_z:
        match = re.match(r"r.(-?\d+).(-?\d+).mca", src.name)
        if not match:
            raise ValueError("region_x and region_z is unknown")
        region_x = int(match.group(1))
        region_z = int(match.group(2))
        log.debug("parse region position", filename=src, x=region_x, z=region_z)

    with src.open("rb") as mca_in:
        # parse header
        chunks: list[Chunk] = []
        locations_raw = memoryview(mca_in.read(0x1000))
        timestamps_raw = memoryview(mca_in.read(0x1000))
        if len(locations_raw) != 0x1000 or len(timestamps_raw) != 0x1000:
            raise RuntimeError(
                f"Region file {src} has truncated header: {len(locations_raw)} + {len(timestamps_raw)}"
            )
        for i in range(1024):
            x = i % 32
            z = i // 32
            loc = locations_raw[i * 4 : (i + 1) * 4]
            offset = int.from_bytes(loc[:3], byteorder="big")
            size = int.from_bytes(loc[3:], byteorder="big")
            ts = int.from_bytes(timestamps_raw[i * 4 : (i + 1) * 4], byteorder="big")
            log.debug("meta info", i=i, x=x, z=z, offset=offset, size=size, ts=ts)
            if offset == 0 and size == 0:
                log.info("chunk not exists", i=i, x=x, z=z)
                continue
            if offset < 2:
                raise RuntimeError(
                    f"Region file {src} has invalid sector at index: {i}; sector {offset} overlaps with header"
                )
            if size == 0:
                raise RuntimeError(
                    f"Region file {src} has an invalid sector at index: {i}; size has to be > 0"
                )
            if offset < 2:
                raise RuntimeError(
                    f"Region file {src} has invalid sector at index: {i}; sector {offset} overlaps with header"
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
                    "source": src,
                    "timestamp": ts,
                    "compression_type": None,
                    "data": None,
                }
            )
        chunks.sort(key=lambda c: c["offset_sectors"])

        # exrtact chunks
        for chunk in chunks:
            log.debug("extract chunk", i=chunk["index"])
            seek_offset = mca_in.seek(chunk["offset_sectors"] * SECTOR_SIZE)
            if seek_offset != chunk["offset_sectors"] * SECTOR_SIZE:
                raise RuntimeError(
                    f"Region file {src} has an invalid sector at index: {chunk['index']}; sector {chunk['size_sectors']} is out of bounds"
                )
            raw = memoryview(mca_in.read(chunk["size_sectors"] * SECTOR_SIZE))
            data_length = int.from_bytes(raw[:4], byteorder="big")
            compression_type = int.from_bytes(raw[4:5], byteorder="big")
            compressed_data = raw[5 : 5 + data_length]
            match compression_type:
                case 2:
                    data = zlib.decompress(compressed_data)
                case 129:
                    raise NotImplementedError("mcc file is not supported")
                case _:
                    raise NotImplementedError(
                        f"not supportd compression_type: {compression_type}"
                    )
            data = normalize(data)
            chunk["compression_type"] = compression_type
            chunk["data"] = data

        # write mcc files
        for chunk in chunks:
            chunk_x = region_x * 32 + chunk["local_x"]
            chunk_z = region_z * 32 + chunk["local_z"]
            mcc_filepath = dst / f"c.{chunk_x}.{chunk_z}.mcc"
            with mcc_filepath.open("wb") as mcc_out:
                mcc_out.write(bytes([0x00, 0x00, 0x00, 0x00, 0x00]))
                if not chunk["data"]:
                    raise RuntimeError(f"Chunk {chunk['index']} has no data")
                mcc_out.write(chunk["data"])

        # write mca file
        mca_filepath = dst / f"r.{region_x}.{region_z}.mca"
        with mca_filepath.open("wb") as mca_out:
            for i, chunk in enumerate(chunks):
                mca_out.seek(chunk["index"] * 4)
                mca_out.write((i + 1).to_bytes(length=3, byteorder="big"))
                mca_out.write((1).to_bytes(length=1, byteorder="big"))

                mca_out.seek(0x1000 + chunk["index"] * 4)
                mca_out.write(chunk["timestamp"].to_bytes(length=4, byteorder="big"))

                mca_out.seek(0x2000 + i * SECTOR_SIZE)
                if not chunk["data"]:
                    raise RuntimeError(f"Chunk {chunk['index']} has no data")
                mca_out.write(len(chunk["data"]).to_bytes(length=4, byteorder="big"))
                NO_COMPRESS = 3
                mca_out.write(NO_COMPRESS.to_bytes(length=1, byteorder="big"))
