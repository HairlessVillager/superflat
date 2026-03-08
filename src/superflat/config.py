from pathlib import Path
from typing import TypedDict


class Config(TypedDict):
    save_dir: Path
    git_dir: Path | None
    cache_dir: Path | None
    name: str
    version: str
    seed: int
