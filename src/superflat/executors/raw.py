import shutil
from pathlib import Path

from superflat.executors.base import Executor, collect_valid_paths
from superflat.paths import raw_paths


class RawFileFlattenExecutor(Executor):
    def collect_task(self, save_dir: Path, repo_dir: Path):
        self.save_dir = save_dir
        self.repo_dir = repo_dir
        self.rel_paths = collect_valid_paths(save_dir, raw_paths)

    def batch_execute(self):
        for rel_path in self.rel_paths:
            dst = self.repo_dir / rel_path
            dst.parent.mkdir(parents=True, exist_ok=True)
            shutil.copy2(self.save_dir / rel_path, dst)


class RawFileUnflattenExecutor(Executor):
    def collect_task(self, save_dir: Path, repo_dir: Path):
        self.save_dir = save_dir
        self.repo_dir = repo_dir
        self.rel_paths = collect_valid_paths(repo_dir, raw_paths)

    def batch_execute(self):
        for rel_path in self.rel_paths:
            dst = self.save_dir / rel_path
            dst.parent.mkdir(parents=True, exist_ok=True)
            shutil.copy2(self.repo_dir / rel_path, dst)
