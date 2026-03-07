import shutil
from functools import cached_property
from pathlib import Path
from typing import override

from superflat.paths import raw_paths
from superflat.strategy.base import Strategy


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
        dst = self.git_dir / rel_path
        dst.parent.mkdir(parents=True, exist_ok=True)
        shutil.copy2(self.save_dir / rel_path, dst)

    @override
    def unflatten(self, rel_path: Path):
        dst = self.save_dir / rel_path
        dst.parent.mkdir(parents=True, exist_ok=True)
        shutil.copy2(self.git_dir / rel_path, self.save_dir / rel_path)
