from pathlib import Path
from typing import Annotated, Literal

import structlog
import typer

from superflat.app import Applicatioin
from superflat.dumper import ZeroDumper

APP_NAME = "superflat"
log = structlog.get_logger()


def parse_kvs(kvs: list[str]) -> dict[str, str]:
    d = {}
    if kvs:
        for item in kvs:
            if "=" not in item:
                raise typer.BadParameter(
                    f"Failed to parse '{item}', expected KEY=VALUE"
                )
            key, val = item.split("=", 1)
            d[key.strip()] = val.strip()
    log.debug(f"parse keys: {d}")
    return d


OptionSaveDir = Annotated[
    Path, typer.Option("--save-dir", "-s", help="Path to your save")
]
OptionRepoDir = Annotated[
    Path,
    typer.Option("--repo-dir", "-r", help="Path to the flatten Git repository"),
]
OptionCacheDir = Annotated[
    Path | None,
    typer.Option("--cache-dir", "-c", help="Path to cache the sections dumps"),
]
OptionTerrain = Annotated[
    bool,
    typer.Option(
        "--terrain/--no-terrain",
        help="Use sections dumps from natural terrain (powered by Pumpkin-MC). Disable this when world has no natural terrain (eg. superflat world, UGC save)",
    ),
]
OptionBlockIdMappingList = Annotated[
    list[str] | None,
    typer.Option(
        "--block-id-mapping",
        "-b",
        help="Setting block id mapping. A workaround for difference block id between Minecraft version (format: KEY=VALUE)",
    ),
]


def cli(
    command: Literal["flatten", "unflatten"],
    save_dir: OptionSaveDir,
    repo_dir: OptionRepoDir,
    block_id_mapping_list: OptionBlockIdMappingList = None,
    cache_dir: OptionCacheDir = None,
    terrain: OptionTerrain = False,
    # NOTE on 20260312: set default to False because this option cannot save delta space for now.
    # Pumpkin-MC terrain generation is still work in progress, and there will be a day to use True as default.
):
    if terrain:
        raise NotImplementedError("terrain is temporarily not maintained")

    block_id_mapping = parse_kvs(block_id_mapping_list) if block_id_mapping_list else {}
    save_dir = save_dir.resolve()
    repo_dir = repo_dir.resolve()

    dumper = ZeroDumper()

    log.info(
        f"{command.capitalize()} save from {save_dir} to {repo_dir}, "
        + ("cache on {cache_dir}, " if cache_dir else "no cache, ")
        + ("using terrain" if terrain else "disable terrain"),
        save_dir=save_dir,
        repo_dir=repo_dir,
        cache_dir=cache_dir,
        terrain=terrain,
    )

    app = Applicatioin(save_dir, repo_dir, dumper, block_id_mapping)
    if command == "flatten":
        app.flatten()
    elif command == "unflatten":
        app.unflatten()


def typer_app():
    try:
        typer.run(cli)
    except Exception:
        log.exception(f"{APP_NAME} Failed")
        exit(-1)


if __name__ == "__main__":
    cli(
        "flatten",
        # save_dir=Path(
        #     "/home/hlsvillager/.config/hmcl/.minecraft/versions/Fabulously-Optimized-1.21.11/saves/lewis20260309 lewis的世界"
        # ),
        # repo_dir=Path("temp/repo"),
        save_dir=Path("temp/saves/2026-03-08_19-25-54_test42/test42"),
        repo_dir=Path("temp/repo2"),
        # block_id_mapping_list=[
        #     "minecraft:grass=minecraft:short_grass"  # 1.20.3
        #     "minecraft:chain=minecraft:iron_chain"  # 1.21.9
        # ],
    )
