from pathlib import Path

def normalize_nbt(input: bytes) -> bytes: ...
def chunk_region_decode_batch(
    others: list[bytes],
    sections_deltas: list[bytes],
    sections_dumps: list[bytes],
    compressed: bool,
) -> list[bytes]: ...
def is_chunk_status_full(input: bytes) -> bool: ...
def chunk_region_flatten(
    save_dir: Path,
    repo_dir: Path,
    block_id_mapping: dict[str, str],
) -> list[Path]: ...
def chunk_region_unflatten(
    save_dir: Path,
    repo_dir: Path,
) -> list[Path]: ...
