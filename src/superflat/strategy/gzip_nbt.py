import gzip
from functools import cached_property
from pathlib import Path
from typing import override

from pumpkin_py import normalize_nbt
from superflat.paths import gzip_nbt_paths

from .base import Strategy


class GzipNbtFileStrategy(Strategy):
    @cached_property
    @override
    def flatten_paths(self) -> set[Path]:
        return gzip_nbt_paths(self.save_dir)

    @cached_property
    @override
    def unflatten_paths(self) -> set[Path]:
        return gzip_nbt_paths(self.git_dir)

    @override
    def flatten(self, rel_path: Path):
        content = (self.save_dir / rel_path).read_bytes()
        content = gzip.decompress(content)
        content = normalize_nbt(content)
        (self.git_dir / rel_path).write_bytes(content)

    @override
    def unflatten(self, rel_path: Path):
        content = (self.git_dir / rel_path).read_bytes()
        content = gzip.decompress(content)
        (self.save_dir / rel_path).write_bytes(content)
