import gzip
from pathlib import Path

from structlog import get_logger
from superflat.superflat_pumpkin import normalize_nbt

from superflat.crafters.base import Crafter, collect_valid_paths
from superflat.paths import gzip_nbt_paths
from superflat.utils import write_bin

log = get_logger()


class GzipNbtFileFlattenCrafter(Crafter):
    def __init__(self, save_dir: Path, repo_dir: Path):
        self.save_dir = save_dir
        self.repo_dir = repo_dir

    def __call__(self) -> list[Path]:
        rel_paths = collect_valid_paths(self.save_dir, gzip_nbt_paths)
        for rel_path in rel_paths:
            content = (self.save_dir / rel_path).read_bytes()
            content = gzip.decompress(content)
            content = normalize_nbt(content)
            write_bin(self.repo_dir / rel_path, content)
        return rel_paths


class GzipNbtFileUnflattenCrafter(Crafter):
    def __init__(self, save_dir: Path, repo_dir: Path):
        self.save_dir = save_dir
        self.repo_dir = repo_dir

    def __call__(self) -> list[Path]:
        rel_paths = collect_valid_paths(self.repo_dir, gzip_nbt_paths)
        for rel_path in rel_paths:
            content = (self.repo_dir / rel_path).read_bytes()
            content = gzip.compress(content)
            write_bin(self.save_dir / rel_path, content)
        return rel_paths
