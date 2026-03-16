import gzip
from pathlib import Path
from typing import Iterable

from structlog import get_logger

from superflat import _superflat
from superflat.crafters.base import Crafter, CrafterEntry, collect_valid_paths
from superflat.paths import gzip_nbt_paths

log = get_logger()


class GzipNbtFileCrafter(Crafter):
    def __init__(self, save_dir: Path, repo_dir: Path):
        self.save_dir = save_dir
        self.repo_dir = repo_dir

    def flatten(self) -> Iterable[CrafterEntry]:
        def f(rel_path: Path):
            content = (self.save_dir / rel_path).read_bytes()
            content = gzip.decompress(content)
            content = _superflat.pumpkin.normalize_nbt(content)
            return content

        return (
            {"input_paths": [rp], "output_path_bins": [(rp, f(rp))]}
            for rp in collect_valid_paths(self.save_dir, gzip_nbt_paths)
        )

    def unflatten(self) -> Iterable[CrafterEntry]:
        def f(rel_path: Path):
            content = (self.repo_dir / rel_path).read_bytes()
            content = gzip.compress(content)
            return content

        return (
            {"input_paths": [rp], "output_path_bins": [(rp, f(rp))]}
            for rp in collect_valid_paths(self.repo_dir, gzip_nbt_paths)
        )
