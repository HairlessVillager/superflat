from pathlib import Path

import typer


def count_mca(path: Path):
    with path.open("rb") as f:
        f.seek(0x1000)
        timestamp_header = f.read(0x1000)
        timestamps = [
            int.from_bytes(timestamp_header[i * 4 : (i + 1) * 4]) for i in range(1024)
        ]
        count = len([1 for t in timestamps if t != 0])
        print(count)


if __name__ == "__main__":
    typer.run(count_mca)
