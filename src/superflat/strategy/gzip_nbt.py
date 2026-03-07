import gzip
from functools import cached_property
from pathlib import Path
from typing import override

from pumpkin_py import normalize_nbt

from .base import Strategy


class GzipNbtFileStrategy(Strategy):
    @cached_property
    @override
    def paths(self) -> set[Path]:
        return {
            # root
            self.save_dir / "level.dat",
            self.save_dir / "data/idcounts.dat",
            *self.save_dir.glob("data/command_storage_*.dat"),
            *self.save_dir.glob("data/map_*.dat"),
            self.save_dir / "data/scoreboard.dat",
            self.save_dir / "data/stopwatches.dat",
            *self.save_dir.glob("generated/*/structures/*.nbt"),
            *self.save_dir.glob("playerdata/*.dat"),
            # dimensions
            *(
                self.save_dir / dimensions_dir / "data" / dimensions_gzip_nbt_file
                for dimensions_dir in self.dimensions_dirs
                for dimensions_gzip_nbt_file in [
                    "chunks.dat",
                    "raids.dat",
                    "raids_end.dat",
                    "random_sequences.dat",
                    "world_border.dat",
                ]
            ),
        }

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
