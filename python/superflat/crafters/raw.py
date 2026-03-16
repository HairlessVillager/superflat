from pathlib import Path
from typing import Iterable

from superflat.crafters.base import Crafter, CrafterEntry, collect_valid_paths
from superflat.paths import raw_paths


class RawFileCrafter(Crafter):
    def __init__(self, save_dir: Path, repo_dir: Path):
        self.save_dir = save_dir
        self.repo_dir = repo_dir

    def flatten(self) -> Iterable[CrafterEntry]:
        def f(rel_path: Path):
            return (self.save_dir / rel_path).read_bytes()

        return (
            {"input_paths": [rp], "output_path_bins": [(rp, f(rp))]}
            for rp in collect_valid_paths(self.save_dir, raw_paths)
        )

    def unflatten(self) -> Iterable[CrafterEntry]:

        def f(rel_path: Path):
            return (self.repo_dir / rel_path).read_bytes()

        return (
            {"input_paths": [rp], "output_path_bins": [(rp, f(rp))]}
            for rp in collect_valid_paths(self.repo_dir, raw_paths)
        )
