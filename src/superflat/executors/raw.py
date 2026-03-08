import shutil
from pathlib import Path

from superflat.executors.base import Executor, collect_valid_paths
from superflat.paths import raw_paths


class RawFileFlattenExecutor(Executor):
    def collect_task(self, save_dir: Path, git_dir: Path):
        self.save_dir = save_dir
        self.git_dir = git_dir
        self.rel_paths = collect_valid_paths(save_dir, raw_paths)

    def batch_execute(self):
        for rel_path in self.rel_paths:
            dst = self.git_dir / rel_path
            dst.parent.mkdir(parents=True, exist_ok=True)
            shutil.copy2(self.save_dir / rel_path, dst)


class RawFileUnflattenExecutor(Executor):
    def collect_task(self, save_dir: Path, git_dir: Path):
        self.save_dir = save_dir
        self.git_dir = git_dir
        self.rel_paths = collect_valid_paths(git_dir, raw_paths)

    def batch_execute(self):
        for rel_path in self.rel_paths:
            dst = self.save_dir / rel_path
            dst.parent.mkdir(parents=True, exist_ok=True)
            shutil.copy2(self.git_dir / rel_path, dst)
