from pathlib import Path

import zstandard as zstd
from structlog import get_logger

from pumpkin_py import seed_to_sections

from .utils import Coords, write_bin

log = get_logger()
cctx = zstd.ZstdCompressor()
dctx = zstd.ZstdDecompressor()


class SectionsDumper:
    def __init__(self, seed: int, stroage_dir: Path):
        self.seed = seed
        self.stroage_dir = stroage_dir

    def stroage_filepath(self, chunk_x: int, chunk_z: int) -> Path:
        return self.stroage_dir / f"c.{chunk_x}.{chunk_z}.dump"

    def is_cached(self, chunk_x: int, chunk_z: int) -> bool:
        return self.stroage_filepath(chunk_x, chunk_z).exists()

    # TODO: dim
    def batch_generate(self, coords: Coords):
        log.info("Generating sections dumps")
        filtered_coords = [
            coord for coord in coords if not self.is_cached(coord[0], coord[1])
        ]
        for chunk_x, chunk_z in filtered_coords:
            nbt = seed_to_sections(self.seed, chunk_x, chunk_z)
            path = self.stroage_filepath(chunk_x, chunk_z)
            data = cctx.compress(nbt)
            write_bin(path, data)
        count = len(filtered_coords)
        log.info(f"Generated {count} sections dumps", count=count)

    def get(self, chunk_x: int, chunk_z: int) -> bytes | None:
        path = self.stroage_filepath(chunk_x, chunk_z)
        if path.exists():
            data = path.read_bytes()
            data = dctx.decompress(data)
            return data
        else:
            return None
