from pathlib import Path
from typing import Callable, TypedDict

class EncodeTask(TypedDict):
    chunk_xz: tuple[int, int]
    chunk_nbt: bytes
    rel_path: Path
    sections_dump: bytes

class EncodeTaskResult(TypedDict):
    chunk_xz: tuple[int, int]
    delta_sections: bytes
    other: bytes

def normalize_nbt(input: bytes) -> bytes: ...
def chunk_region_encode_batch(
    tasks: list[EncodeTask],
    compressed: bool,
) -> list[EncodeTaskResult]: ...
def chunk_region_decode_batch(
    others: list[bytes],
    sections_deltas: list[bytes],
    sections_dumps: list[bytes],
    compressed: bool,
) -> list[bytes]: ...
def seed_from_level(level_nbt: bytes) -> int: ...
def seed_to_sections_batch(seed: int, coords: list[tuple[int, int]]) -> list[bytes]: ...
def is_chunk_status_full(input: bytes) -> bool: ...
def chunk_region_flatten(
    save_dir: Path,
    repo_dir: Path,
    block_id_mapping: dict[str, str],
) -> list[Path]: ...
def chunk_region_unflatten(
    save_dir: str,
    repo_dir: str,
    dumper_get: Callable[[int, int], bytes | None],
    dumper_compressed: bool,
) -> list[str]: ...
