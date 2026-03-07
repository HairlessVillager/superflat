from pathlib import Path
from typing import TypedDict

from superflat.sfnbt import SFNBTManager
from superflat.strategy import Strategy


class Config(TypedDict):
    strategy_classes: list[type[Strategy]]
    save_dir: Path
    git_dir: Path
    cache_dir: Path
    sfnbt_manager: SFNBTManager
