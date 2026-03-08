from pathlib import Path

import zstandard as zstd
from structlog import get_logger

from pumpkin_py import seed_to_sections_batch

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
        dumps = seed_to_sections_batch(self.seed, filtered_coords)
        for (chunk_x, chunk_z), dump in zip(filtered_coords, dumps):
            path = self.stroage_filepath(chunk_x, chunk_z)
            write_bin(path, dump)
        count = len(filtered_coords)
        log.info(f"Generated {count} sections dumps", count=count)

    def get(self, chunk_x: int, chunk_z: int) -> bytes | None:
        path = self.stroage_filepath(chunk_x, chunk_z)
        if path.exists():
            data = path.read_bytes()
            return data
        else:
            return None
