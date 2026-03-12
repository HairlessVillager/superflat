import shutil
from pathlib import Path

from superflat.crafters.base import Crafter, collect_valid_paths
from superflat.paths import raw_paths


class RawFileFlattenCrafter(Crafter):
    def __init__(self, save_dir: Path, repo_dir: Path):
        self.save_dir = save_dir
        self.repo_dir = repo_dir

    def __call__(self) -> list[Path]:
        rel_paths = collect_valid_paths(self.save_dir, raw_paths)
        for rel_path in rel_paths:
            dst = self.repo_dir / rel_path
            dst.parent.mkdir(parents=True, exist_ok=True)
            shutil.copy2(self.save_dir / rel_path, dst)
        return rel_paths


class RawFileUnflattenCrafter(Crafter):
    def __init__(self, save_dir: Path, repo_dir: Path):
        self.save_dir = save_dir
        self.repo_dir = repo_dir

    def __call__(self) -> list[Path]:
        rel_paths = collect_valid_paths(self.repo_dir, raw_paths)
        for rel_path in rel_paths:
            dst = self.save_dir / rel_path
            dst.parent.mkdir(parents=True, exist_ok=True)
            shutil.copy2(self.repo_dir / rel_path, dst)
        return rel_paths
