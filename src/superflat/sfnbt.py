from pathlib import Path

from structlog import get_logger

from pumpkin_py import sf_from_seed_batch

log = get_logger()


class SFNBTManager:
    def __init__(self, seed: int, stroage_dir: Path):
        self.seed = seed
        self.stroage_dir = stroage_dir

    def stroage_filepath(self, chunk_x: int, chunk_z: int) -> Path:
        return self.stroage_dir / str(self.seed) / f"c.{chunk_x}.{chunk_z}.sf.nbt"

    def is_cached(self, chunk_x: int, chunk_z: int) -> bool:
        return self.stroage_filepath(chunk_x, chunk_z).exists()

    def batch_generate(self, coords: list[tuple[int, int]]) -> int:
        coords = [coord for coord in coords if not self.is_cached(coord[0], coord[1])]
        sfnbt = sf_from_seed_batch(self.seed, coords)
        for (chunk_x, chunk_z), nbt in zip(coords, sfnbt, strict=True):
            path = self.stroage_filepath(chunk_x, chunk_z)
            # path.parent.mkdir(parents=True, exist_ok=True)
            path.write_bytes(nbt)
            # log.debug(f"write nbt data to {path}")
        return len(coords)

    def get(self, chunk_x: int, chunk_z: int) -> bytes | None:
        path = self.stroage_filepath(chunk_x, chunk_z)
        if path.exists():
            return path.read_bytes()
        else:
            return None
