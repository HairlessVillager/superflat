from pathlib import Path
from typing import Callable, Iterable, Protocol

from structlog import get_logger

log = get_logger()


class Executor(Protocol):
    def collect_task(self, save_dir: Path, repo_dir: Path) -> Iterable[Path]: ...
    def batch_execute(self): ...


def collect_valid_paths(base: Path, pf: Callable[[Path], Iterable[Path]]) -> list[Path]:
    rel_paths = []
    for p in pf(base):
        if p.exists():
            rel_paths.append(p.relative_to(base))
        else:
            log.warn(f"{p} not exists, skipped")
    return rel_paths
