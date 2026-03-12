from abc import ABC, abstractmethod
from pathlib import Path
from typing import Callable, Iterable

from structlog import get_logger

log = get_logger()


class Crafter(ABC):
    @abstractmethod
    def __call__(self) -> Iterable[Path]: ...


def collect_valid_paths(base: Path, pf: Callable[[Path], Iterable[Path]]) -> list[Path]:
    rel_paths = []
    for p in pf(base):
        if p.exists():
            rel_paths.append(p.relative_to(base))
        else:
            log.warn(f"{p} not exists, skipped")
    return rel_paths
