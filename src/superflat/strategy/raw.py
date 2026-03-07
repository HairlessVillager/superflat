import shutil
from functools import cached_property
from pathlib import Path
from typing import override

from superflat.paths import raw_paths

from .base import Strategy


class RawFileStrategy(Strategy):
    @cached_property
    @override
    def flatten_paths(self) -> set[Path]:
        return raw_paths(self.save_dir)

    @cached_property
    @override
    def unflatten_paths(self) -> set[Path]:
        return raw_paths(self.git_dir)

    @override
    def flatten(self, rel_path: Path):
        shutil.copy2(self.save_dir / rel_path, self.git_dir / rel_path)

    @override
    def unflatten(self, rel_path: Path):
        shutil.copy2(self.git_dir / rel_path, self.save_dir / rel_path)
