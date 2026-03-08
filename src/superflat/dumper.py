from pathlib import Path

import zstandard as zstd
from pumpkin_py import seed_to_sections
from structlog import get_logger

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
    def batch_generate(self, coords: Coords) -> int:
        filtered_coords = [
            coord for coord in coords if not self.is_cached(coord[0], coord[1])
        ]
        # sfnbt = sf_from_seed_batch(self.seed, filtered_coords)
        # for (chunk_x, chunk_z), nbt in zip(filtered_coords, sfnbt, strict=True):
        #     path = self.stroage_filepath(chunk_x, chunk_z)
        #     data = cctx.compress(nbt)
        #     write_bin(path, data)
        # return len(filtered_coords)
        for chunk_x, chunk_z in filtered_coords:
            nbt = seed_to_sections(self.seed, chunk_x, chunk_z)
            path = self.stroage_filepath(chunk_x, chunk_z)
            data = cctx.compress(nbt)
            write_bin(path, data)
        return len(filtered_coords)

    def get(self, chunk_x: int, chunk_z: int) -> bytes | None:
        path = self.stroage_filepath(chunk_x, chunk_z)
        if path.exists():
            data = path.read_bytes()
            # bytes = dctx.stream_reader(bytes).read()
            data = dctx.decompress(data)
            return data
        else:
            return None
