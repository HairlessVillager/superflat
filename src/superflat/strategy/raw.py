import shutil
from functools import cached_property
from pathlib import Path
from typing import override

from .base import Strategy


class RawFileStrategy(Strategy):
    @cached_property
    @override
    def paths(self) -> set[Path]:
        return {
            self.save_dir / "icon.png",
            *self.save_dir.glob("advancements/*.json"),
            *self.save_dir.glob("stats/*.json"),
        }

    @override
    def flatten(self, rel_path: Path):
        shutil.copy2(self.save_dir / rel_path, self.git_dir / rel_path)

    @override
    def unflatten(self, rel_path: Path):
        shutil.copy2(self.git_dir / rel_path, self.save_dir / rel_path)
