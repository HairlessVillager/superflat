from abc import ABC, abstractmethod
from functools import cached_property
from pathlib import Path


class Strategy(ABC):
    def __init__(self, save_dir: Path, git_dir: Path, cache_dir: Path):
        self.save_dir = save_dir
        self.git_dir = git_dir
        self.cache_dir = cache_dir

    @cached_property
    def dimensions_dirs(self) -> set[Path]:
        return {
            self.save_dir,
            self.save_dir / "DIM1",
            self.save_dir / "DIM-1",
            *self.save_dir.glob("dimensions/*/*"),
        }

    @cached_property
    @abstractmethod
    def paths(self) -> set[Path]: ...

    @abstractmethod
    def flatten(self, rel_path: Path): ...

    @abstractmethod
    def unflatten(self, rel_path: Path): ...
