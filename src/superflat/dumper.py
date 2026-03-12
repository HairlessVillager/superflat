from pathlib import Path
from typing import Callable, Iterable, Protocol

import zstandard as zstd
from structlog import get_logger
from superflat_pumpkin import seed_to_sections_batch

from .utils import Coords, exrtact_xz, get_full_chunks, write_bin

log = get_logger()
cctx = zstd.ZstdCompressor()
dctx = zstd.ZstdDecompressor()


class Dumper(Protocol):
    def batch_generate(self, coords: Coords): ...
    def get(self, chunk_x: int, chunk_z: int) -> bytes | None: ...
    @property
    def compressed(self) -> bool: ...
    def collect_full_chunks(
        self, base_dir: Path, pf: Callable[[Path], Iterable[Path]]
    ) -> Coords: ...


class SectionsDumper(Dumper):
    def __init__(self, seed: int, stroage_dir: Path):
        self.seed = seed
        self.stroage_dir = stroage_dir

    def stroage_filepath(self, chunk_x: int, chunk_z: int) -> Path:
        return self.stroage_dir / f"c.{chunk_x}.{chunk_z}.dump"

    def is_cached(self, chunk_x: int, chunk_z: int) -> bool:
        return self.stroage_filepath(chunk_x, chunk_z).exists()

    def collect_full_chunks(
        self, base_dir: Path, pf: Callable[[Path], Iterable[Path]]
    ) -> Coords:
        log.info("Collecting full chunks")
        coords = set()
        for dirpath, _dirnames, filenames in base_dir.walk():
            for filename in filenames:
                filepath = dirpath / filename
                rel_path = filepath.relative_to(base_dir)
                if filepath in pf(base_dir):
                    if region_xz := exrtact_xz(rel_path.name):
                        region_x, region_z = region_xz
                        coords |= get_full_chunks(filepath, region_x, region_z)
                    else:
                        log.warn(
                            f"Cannot exrtact x and z in {rel_path.name}",
                            filepath=filepath,
                        )
        log.info(f"Collected {len(coords)} full chunks", count=len(coords))
        return coords

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

    @property
    def compressed(self) -> bool:
        return True


class ZeroDumper(Dumper):
    DUMP_SIZE = 0x3062A
    ZERO_DUMP = bytes(DUMP_SIZE)

    def batch_generate(self, coords: Coords):
        pass

    def get(self, chunk_x: int, chunk_z: int) -> bytes | None:
        return self.ZERO_DUMP

    @property
    def compressed(self) -> bool:
        return False

    def collect_full_chunks(
        self, base_dir: Path, pf: Callable[[Path], Iterable[Path]]
    ) -> Coords:
        return set()
