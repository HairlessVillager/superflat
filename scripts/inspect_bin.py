from collections import Counter
from itertools import batched
from pathlib import Path

import typer


def main(file: Path, offset: str, length: str, size: str):
    offset_n = int(offset, 0)
    length_n = int(length, 0)
    size_n = int(size, 0)
    bs = file.read_bytes()[offset_n : offset_n + length_n]
    for k, v in Counter(batched(bs, size_n)).most_common():
        print(bytes(k).hex(), v)


if __name__ == "__main__":
    typer.run(main)
