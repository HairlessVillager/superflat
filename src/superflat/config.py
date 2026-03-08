from pathlib import Path
from typing import TYPE_CHECKING, TypedDict

from superflat.dumper import SectionsDumper

if TYPE_CHECKING:
    from superflat.strategy import Strategy


class Config(TypedDict):
    strategy_classes: list[type["Strategy"]]
    save_dir: Path
    git_dir: Path
    cache_dir: Path
    dumper: SectionsDumper
