# see https://minecraft.wiki/w/Java_Edition_level_format

import gzip
import json
import math
import re
import zlib
from pathlib import Path
from typing import TypedDict

from pumpkin_py import normalize_nbt
from structlog import get_logger

log = get_logger()

SECTOR_SIZE = 4096


class EmptyFile(Exception):
    pass


class RegionFileFlattenEntry(TypedDict):
    chunk_x: int
    chunk_z: int
    nbt: bytes


class RegionFileFlattenResult(TypedDict):
    region_x: int
    region_z: int
    timestamp_header: bytes
    chunks: list[RegionFileFlattenEntry]


class RegionFileUnflattenEntry(TypedDict):
    filename: str
    content: bytes


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


class Flattener:
    def __init__(self, input_dir: Path, output_dir: Path):
        self._unflatten_dir = input_dir
        self._flatten_dir = output_dir

    def gzip_nbt_flatten(self, input: bytes) -> bytes:
        original_nbt = gzip.decompress(input)
        return normalize_nbt(original_nbt)

    def gzip_nbt_unflatten(self, input: bytes) -> bytes:
        return gzip.compress(input)

    def extract_region_xz(self, filename: str) -> tuple[int, int]:
        match = re.match(r"r.(-?\d+).(-?\d+).mca", filename)
        if not match:
            raise ValueError("region_x and region_z is unknown")
        region_x = int(match.group(1))
        region_z = int(match.group(2))
        return region_x, region_z

    def region_file_flatten(self, file: Path) -> RegionFileFlattenResult:
        region_x, region_z = self.extract_region_xz(file.name)
        log.debug("parse region position", filename=file, x=region_x, z=region_z)
        with file.open("rb") as region:
            log.debug("parse header")
            chunks: list[Chunk] = []
            locations_raw = memoryview(region.read(0x1000))
            timestamps_raw = memoryview(region.read(0x1000))
            if len(locations_raw) == 0 and len(timestamps_raw) == 0:
                raise EmptyFile()
            elif len(locations_raw) != 0x1000 or len(timestamps_raw) != 0x1000:
                raise RuntimeError(
                    f"Region file {file} has truncated header: {len(locations_raw)} + {len(timestamps_raw)}"
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
                    # log.debug("chunk not exists", i=i, x=x, z=z)
                    continue
                if offset < 2:
                    raise RuntimeError(
                        f"Region file {file} has invalid sector at index: {i}; sector {offset} overlaps with header"
                    )
                if size == 0:
                    raise RuntimeError(
                        f"Region file {file} has an invalid sector at index: {i}; size has to be > 0"
                    )
                if offset < 2:
                    raise RuntimeError(
                        f"Region file {file} has invalid sector at index: {i}; sector {offset} overlaps with header"
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
                        "source": file,
                        "timestamp": ts,
                        "compression_type": None,
                        "data": None,
                        "chunk_x": chunk_x,
                        "chunk_z": chunk_z,
                    }
                )
            chunks.sort(key=lambda c: c["offset_sectors"])

            log.debug("extract chunk")
            for chunk in chunks:
                seek_offset = region.seek(chunk["offset_sectors"] * SECTOR_SIZE)
                if seek_offset != chunk["offset_sectors"] * SECTOR_SIZE:
                    raise RuntimeError(
                        f"Region file {file} has an invalid sector at index: {chunk['index']}; sector {chunk['size_sectors']} is out of bounds"
                    )
                raw = memoryview(region.read(chunk["size_sectors"] * SECTOR_SIZE))
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
                data = normalize_nbt(data)
                chunk["compression_type"] = compression_type
                chunk["data"] = data

            return {
                "region_x": region_x,
                "region_z": region_z,
                "timestamp_header": timestamps_raw.tobytes(),
                "chunks": [
                    (
                        {
                            "chunk_x": chunk["chunk_x"],
                            "chunk_z": chunk["chunk_z"],
                            "nbt": chunk["data"],
                        }
                    )
                    for chunk in chunks
                    if chunk["data"] is not None
                ],
            }

    def region_file_unflatten(
        self, flatten: RegionFileFlattenResult
    ) -> list[RegionFileUnflattenEntry]:
        region_x = flatten["region_x"]
        region_z = flatten["region_z"]

        locations = bytearray(4096)
        timestamps = bytearray(flatten["timestamp_header"])
        if len(timestamps) != 4096:
            raise ValueError(f"Invalid timestamp length: {len(timestamps)} != 4096")

        current_sector = 2
        chunk_data_buffer = bytearray()

        for chunk_entry in flatten["chunks"]:
            # basic parameters
            local_x = chunk_entry["chunk_x"] - (region_x * 32)
            local_z = chunk_entry["chunk_z"] - (region_z * 32)
            index = local_x + local_z * 32
            if not (0 <= local_x < 32 and 0 <= local_z < 32):
                raise ValueError(
                    f"Chunk outside region boundary: x={chunk_entry['chunk_x']}, z={chunk_entry['chunk_z']}"
                )

            # chunk datapack
            compression_type = 2
            compressed = zlib.compress(chunk_entry["nbt"])
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
        filename = f"r.{region_x}.{region_z}.mca"

        return [
            {
                "filename": filename,
                "content": bytes(content),
            }
        ]

    def flatten(self):
        get_id = lambda input_path: (  # noqa: E731
            str(input_path.relative_to(self._unflatten_dir))
            .replace("/", "-")
            .replace(".", "-")
            .replace("_", "-")
        )
        ri = lambda input_path: str(input_path.relative_to(self._unflatten_dir))  # noqa: E731
        ro = lambda output_path: str(output_path.relative_to(self._flatten_dir))  # noqa: E731
        dimensions_dirs = [
            "",
            "DIM1",
            "DIM-1",
            *self._unflatten_dir.glob("dimensions/*/*"),
        ]
        index_json = {}

        index_json["raw"] = []
        raw_files = [
            self._unflatten_dir / "icon.png",
            *self._unflatten_dir.glob("advancements/*.json"),
            *self._unflatten_dir.glob("stats/*.json"),
        ]
        for input_path in raw_files:
            log.debug("flattening", file=input_path)
            if not input_path.exists():
                log.warn("file not exists, skipped", file=input_path)
                continue
            region_id = get_id(input_path)
            output_path = self._flatten_dir / "raw" / region_id
            output_path.parent.mkdir(parents=True, exist_ok=True)
            if output_path.exists():
                log.warning("overwrite to an existing file", file=output_path)
            output_path.write_bytes(input_path.read_bytes())
            index_json["raw"].append(
                {
                    "id": region_id,
                    "input_path": ri(input_path),
                    "output_path": ro(output_path),
                }
            )

        index_json["gzip-nbt"] = []
        gzip_nbt_files = [
            # root
            self._unflatten_dir / "level.dat",
            self._unflatten_dir / "data/idcounts.dat",
            *self._unflatten_dir.glob("data/command_storage_*.dat"),
            *self._unflatten_dir.glob("data/map_*.dat"),
            self._unflatten_dir / "data/scoreboard.dat",
            self._unflatten_dir / "data/stopwatches.dat",
            *self._unflatten_dir.glob("generated/*/structures/*.nbt"),
            *self._unflatten_dir.glob("playerdata/*.dat"),
            # dimensions
            *(
                self._unflatten_dir / dimensions_dir / "data" / dimensions_gzip_nbt_file
                for dimensions_dir in dimensions_dirs
                for dimensions_gzip_nbt_file in [
                    "chunks.dat",
                    "raids.dat",
                    "raids_end.dat",
                    "random_sequences.dat",
                    "world_border.dat",
                ]
            ),
        ]
        for input_path in gzip_nbt_files:
            log.debug("flattening", file=input_path)
            if not input_path.exists():
                log.warn("file not exists, skipped", file=input_path)
                continue
            region_id = get_id(input_path)
            output_path = self._flatten_dir / "gzip-nbt" / region_id
            output_path.parent.mkdir(parents=True, exist_ok=True)
            if output_path.exists():
                log.warning("overwrite to an existing file", file=output_path)
            output_path.write_bytes(self.gzip_nbt_flatten(input_path.read_bytes()))
            index_json["gzip-nbt"].append(
                {
                    "id": region_id,
                    "input_path": ri(input_path),
                    "output_path": ro(output_path),
                }
            )

        index_json["region"] = []
        region_files = [
            file
            for dimensions_dir in dimensions_dirs
            for dimensions_region_file_parent in ["entities", "poi", "region"]
            for file in (
                self._unflatten_dir / dimensions_dir / dimensions_region_file_parent
            ).glob("r.*.*.mca")
        ]
        for input_path in region_files:
            log.debug("flattening", file=input_path)
            if not input_path.exists():
                log.warn("file not exists, skipped", file=input_path)
                continue
            try:
                result = self.region_file_flatten(input_path)
            except EmptyFile:
                log.info("empty file, skipped", file=input_path)
                continue
            output_paths = []

            region_id = f"x{result['region_x']}-z{result['region_z']}"
            output_files = [
                (
                    f"timestamp-header-{region_id}",
                    result["timestamp_header"],
                ),
                *(
                    (f"chunk-x{chunk['chunk_x']}-z{chunk['chunk_z']}", chunk["nbt"])
                    for chunk in result["chunks"]
                ),
            ]
            output_base = self._flatten_dir / "region" / get_id(input_path)
            for output_file_id, output_content in output_files:
                output_path = output_base / output_file_id
                output_path.parent.mkdir(parents=True, exist_ok=True)
                if output_path.exists():
                    log.warning("overwrite to an existing file", file=output_path)
                output_path.write_bytes(output_content)
                output_paths.append(ro(output_path))

            index_json["region"].append(
                {
                    "id": region_id,
                    "input_path": ri(input_path),
                    "output_paths": output_paths,
                }
            )

        with (self._flatten_dir / "index.json").open("w") as f:
            log.info("write index json")
            json.dump(index_json, f, indent=4)

        log.info("flatten process finished")

    def unflatten(self):
        index_path = self._flatten_dir / "index.json"
        if not index_path.exists():
            log.error("index.json not found, cannot unflatten", path=index_path)
            return

        with index_path.open("r") as f:
            index_json = json.load(f)

        for item in index_json.get("raw", []):
            source_path = self._flatten_dir / item["output_path"]
            target_path = self._unflatten_dir / item["input_path"]

            log.debug("restoring raw file", target=item["input_path"])
            target_path.parent.mkdir(parents=True, exist_ok=True)
            target_path.write_bytes(source_path.read_bytes())

        for item in index_json.get("gzip-nbt", []):
            source_path = self._flatten_dir / item["output_path"]
            target_path = self._unflatten_dir / item["input_path"]

            log.debug("restoring gzip-nbt file", target=item["input_path"])
            target_path.parent.mkdir(parents=True, exist_ok=True)
            nbt_data = source_path.read_bytes()
            target_path.write_bytes(self.gzip_nbt_unflatten(nbt_data))

        for item in index_json.get("region", []):
            m = re.match(r"x(-?\d+)-z(-?\d+)", item["id"])
            if not m:
                raise ValueError(f"Unknown id: {item['id']}")

            region_x, region_z = int(m.group(1)), int(m.group(2))
            log.debug("restoring region file", region_x=region_x, region_z=region_z)

            chunks_to_pack: list[RegionFileFlattenEntry] = []

            timestamp_header = None
            for rel_path in item["output_paths"]:
                p = self._flatten_dir / rel_path
                fname = p.name

                if "timestamp-header" in fname:
                    timestamp_header = p.read_bytes()
                elif "chunk" in fname:
                    cm = re.search(r"chunk-x(-?\d+)-z(-?\d+)", fname)
                    if cm:
                        chunks_to_pack.append(
                            {
                                "chunk_x": int(cm.group(1)),
                                "chunk_z": int(cm.group(2)),
                                "nbt": p.read_bytes(),
                            }
                        )
                else:
                    raise ValueError(f"Unknown file {str(rel_path)}")
            if not timestamp_header:
                raise RuntimeError(
                    f"Timestamp header file not found under region ({region_x}, {region_z})"
                )

            unflatten_entries = self.region_file_unflatten(
                {
                    "region_x": region_x,
                    "region_z": region_z,
                    "timestamp_header": timestamp_header,
                    "chunks": chunks_to_pack,
                }
            )
            for res in unflatten_entries:
                target_path = self._unflatten_dir / item["input_path"]
                target_path.parent.mkdir(parents=True, exist_ok=True)
                target_path.write_bytes(res["content"])

        log.info("unflatten process finished")


if __name__ == "__main__":
    input_path = "/home/hlsvillager/.config/hmcl/.minecraft/versions/Fabulously-Optimized-1.21.11/saves/test42"
    flatten_path = "/home/hlsvillager/Desktop/superflat/temp/flatten"
    unflatten_path = "/home/hlsvillager/Desktop/superflat/temp/unflatten"
    flattener = Flattener(Path(input_path), Path(flatten_path))
    flattener.flatten()
    flattener = Flattener(Path(unflatten_path), Path(flatten_path))
    flattener.unflatten()
