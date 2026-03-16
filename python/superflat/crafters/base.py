from abc import ABC, abstractmethod
from pathlib import Path
from typing import Callable, Iterable, TypedDict

from structlog import get_logger

log = get_logger()


class CrafterEntry(TypedDict):
    input_paths: Iterable[Path]
    output_path_bins: Iterable[tuple[Path, bytes]]


class Crafter(ABC):
    @abstractmethod
    def flatten(self) -> Iterable[CrafterEntry]: ...

    @abstractmethod
    def unflatten(self) -> Iterable[CrafterEntry]: ...


def collect_valid_paths(base: Path, pf: Callable[[Path], Iterable[Path]]) -> list[Path]:
    rel_paths = []
    for p in pf(base):
        if p.exists():
            rel_paths.append(p.relative_to(base))
        else:
            log.warn(f"{p} not exists, skipped")
    return rel_paths
