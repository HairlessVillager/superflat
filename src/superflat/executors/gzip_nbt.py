import gzip
from pathlib import Path

from pumpkin_py import normalize_nbt
from structlog import get_logger

from superflat.executors.base import Executor, collect_valid_paths
from superflat.paths import gzip_nbt_paths
from superflat.utils import write_bin

log = get_logger()


class GzipNbtFileFlattenExecutor(Executor):
    def collect_task(self, save_dir: Path, repo_dir: Path):
        self.save_dir = save_dir
        self.repo_dir = repo_dir
        self.rel_paths = collect_valid_paths(save_dir, gzip_nbt_paths)

    def batch_execute(self):
        for rel_path in self.rel_paths:
            content = (self.save_dir / rel_path).read_bytes()
            content = gzip.decompress(content)
            content = normalize_nbt(content)
            write_bin(self.repo_dir / rel_path, content)


class GzipNbtFileUnflattenExecutor(Executor):
    def collect_task(self, save_dir: Path, repo_dir: Path):
        self.save_dir = save_dir
        self.repo_dir = repo_dir
        self.rel_paths = collect_valid_paths(repo_dir, gzip_nbt_paths)

    def batch_execute(self):
        for rel_path in self.rel_paths:
            content = (self.repo_dir / rel_path).read_bytes()
            content = gzip.compress(content)
            write_bin(self.save_dir / rel_path, content)
