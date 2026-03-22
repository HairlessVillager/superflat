from pathlib import Path
from sys import stdout

import typer

from superflat.utils import exrtact_xz, read_region_file


def export_chunk(region_path: Path, chunk_x: int, chunk_z: int):
    if region_xz := exrtact_xz(region_path.name):
        region_x, region_z = region_xz
    else:
        raise ValueError(f"Cannot exrtact x & z from {region_path.name}")
    region = read_region_file(region_path, region_x, region_z)
    nbt = region["chunkxz2nbt"][(chunk_x, chunk_z)]
    stdout.buffer.write(nbt)


if __name__ == "__main__":
    typer.run(export_chunk)
