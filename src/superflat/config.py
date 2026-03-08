from pathlib import Path
from typing import TypedDict


class Config(TypedDict):
    save_dir: Path
    git_dir: Path
    cache_dir: Path
    seed: int
