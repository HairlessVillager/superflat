from abc import ABC, abstractmethod
from functools import cached_property
from pathlib import Path

from superflat.config import Config
from superflat.utils import Coords


class Strategy(ABC):
    def __init__(self, config: Config, full_chunks: Coords):
        self.save_dir = config["save_dir"]
        self.git_dir = config["git_dir"]
        self.sfnbt_manager = config["sfnbt_manager"]
        self.full_chunks = full_chunks

    @cached_property
    @abstractmethod
    def flatten_paths(self) -> set[Path]: ...

    @abstractmethod
    def flatten(self, rel_path: Path): ...

    @cached_property
    @abstractmethod
    def unflatten_paths(self) -> set[Path]: ...

    @abstractmethod
    def unflatten(self, rel_path: Path): ...
