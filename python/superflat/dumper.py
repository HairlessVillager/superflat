from typing import Protocol

from structlog import get_logger

from .utils import Coords

log = get_logger()


class Dumper(Protocol):
    def batch_generate(self, coords: Coords): ...
    def get(self, chunk_x: int, chunk_z: int) -> bytes | None: ...
    @property
    def compressed(self) -> bool: ...


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
