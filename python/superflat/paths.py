from functools import cache
from pathlib import Path


@cache
def dimensions_dirs(base_dir: Path) -> set[Path]:
    return {
        base_dir,
        base_dir / "DIM1",
        base_dir / "DIM-1",
        *base_dir.glob("dimensions/*/*"),
    }


@cache
def gzip_nbt_paths(base_dir) -> set[Path]:
    return {
        # root
        base_dir / "level.dat",
        base_dir / "data/idcounts.dat",
        *base_dir.glob("data/command_storage_*.dat"),
        *base_dir.glob("data/map_*.dat"),
        base_dir / "data/scoreboard.dat",
        base_dir / "data/stopwatches.dat",
        *base_dir.glob("generated/*/structures/*.nbt"),
        *base_dir.glob("playerdata/*.dat"),
        # dimensions
        *(
            base_dir / dimensions_dir / "data" / dimensions_gzip_nbt_file
            for dimensions_dir in dimensions_dirs(base_dir)
            for dimensions_gzip_nbt_file in [
                "chunks.dat",
                "raids.dat",
                "raids_end.dat",
                "random_sequences.dat",
                "world_border.dat",
            ]
        ),
    }


@cache
def raw_paths(base_dir) -> set[Path]:
    return {
        base_dir / "icon.png",
        *base_dir.glob("advancements/*.json"),
        *base_dir.glob("stats/*.json"),
    }


@cache
def other_region_paths_flatten(base_dir) -> set[Path]:
    return {
        file
        for dimensions_dir in dimensions_dirs(base_dir)
        for dimensions_region_file_parent in ["entities", "poi"]
        for file in (base_dir / dimensions_dir / dimensions_region_file_parent).glob(
            "r.*.*.mca"
        )
    }


@cache
def other_region_paths_unflatten(base_dir) -> set[Path]:
    return {
        file
        for dimensions_dir in dimensions_dirs(base_dir)
        for dimensions_region_file_parent in ["entities", "poi"]
        for file in (base_dir / dimensions_dir / dimensions_region_file_parent).glob(
            "r.*.*.mca/timestamp-header"
        )
    }


@cache
def chunk_region_paths_flatten(base_dir) -> set[Path]:
    return {
        file
        for dimensions_dir in dimensions_dirs(base_dir)
        for dimensions_region_file_parent in ["region"]
        for file in (base_dir / dimensions_dir / dimensions_region_file_parent).glob(
            "r.*.*.mca"
        )
    }


@cache
def chunk_region_paths_unflatten(base_dir) -> set[Path]:
    return {
        file
        for dimensions_dir in dimensions_dirs(base_dir)
        for dimensions_region_file_parent in ["region"]
        for file in (base_dir / dimensions_dir / dimensions_region_file_parent).glob(
            "r.*.*.mca/timestamp-header"
        )
    }
