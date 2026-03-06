import json
import re
import shutil
from hashlib import sha256
from pathlib import Path

from platformdirs import user_cache_path
from pumpkin_py import batch_generate_chunk_nbt
from structlog import get_logger
from xdelta3_py import decode, encode

log = get_logger()


class ChunkManager:
    def __init__(self, seed: int, cache_dir: Path | None = None):
        self._seed = seed
        self._cache_dir = cache_dir or user_cache_path("superflat")

    def cache_filepath(self, chunk_x: int, chunk_z: int) -> Path:
        return self._cache_dir / str(self._seed) / f"c.{chunk_x}.{chunk_z}.nbt"

    def is_cached(self, chunk_x: int, chunk_z: int) -> bool:
        return self.cache_filepath(chunk_x, chunk_z).exists()

    def batch_generate(self, coords: list[tuple[int, int]]):
        coords = [coord for coord in coords if not self.is_cached(coord[0], coord[1])]
        nbts = batch_generate_chunk_nbt(self._seed, coords)
        for (chunk_x, chunk_z), nbt in zip(coords, nbts, strict=True):
            path = self.cache_filepath(chunk_x, chunk_z)
            path.parent.mkdir(parents=True, exist_ok=True)
            path.write_bytes(nbt)
            log.debug(f"write nbt data to {path}")

    def get(self, chunk_x: int, chunk_z: int) -> bytes | None:
        path = self.cache_filepath(chunk_x, chunk_z)
        if path.exists():
            return path.read_bytes()
        else:
            return None


class Differ:
    def __init__(self, seed: int, flatten_dir: Path, git_dir: Path):
        self._seed = seed
        self._flatten_dir = flatten_dir
        self._git_dir = git_dir
        self._chunk_manager = ChunkManager(seed)

    def diff(self):
        if self._git_dir.exists() and any(self._git_dir.iterdir()):
            raise RuntimeError("Difference directory not empty")

        index_path = self._flatten_dir / "index.json"
        if not index_path.exists():
            raise RuntimeError("index.json not found, cannot unflatten")

        with index_path.open("r") as f:
            index_json = json.load(f)

        self._chunk_manager.batch_generate([tuple(c) for c in index_json["chunks"]])

        for i, item in enumerate(index_json.get("region", [])):
            for j, rel_path in enumerate(item["output_paths"]):
                if not isinstance(rel_path, str):
                    raise TypeError(
                        f'index_json["region"][{i}]["output_paths"][{j}] should be a string, got {rel_path}'
                    )

                patched_filepath = self._flatten_dir / rel_path
                fname = patched_filepath.name
                if "chunk" in fname:
                    if "region" in item["input_path"]:
                        cm = re.search(r"chunk-x(-?\d+)-z(-?\d+)", fname)
                        if not cm:
                            raise ValueError(f"Cannot parse {patched_filepath}")
                        chunk_x = int(cm.group(1))
                        chunk_z = int(cm.group(2))

                        base = self._chunk_manager.get(chunk_x, chunk_z)
                        if not base:
                            raise RuntimeError(
                                f"ChunkManager cannot get chunk data at ({chunk_x}, {chunk_z}), but it should be generated before"
                            )
                        patched = patched_filepath.read_bytes()

                        diff = encode(base, patched)
                        if patched_filepath.name == "chunk-x-9-z-8":
                            log.debug(
                                "encode",
                                patch_file=patched_filepath,
                                base_sha256=sha256(base).hexdigest(),
                                diff_sha256=sha256(diff).hexdigest(),
                                patched_sha256=sha256(patched).hexdigest(),
                            )
                        path = self._git_dir / rel_path
                        path.parent.mkdir(parents=True, exist_ok=True)
                        path.write_bytes(diff)
                        log.info(f"write {path}")
                    else:
                        path = self._git_dir / rel_path
                        path.parent.mkdir(parents=True, exist_ok=True)
                        path.symlink_to(self._flatten_dir / rel_path)
                        log.info(f"symlink {path} to {self._flatten_dir / rel_path}")
                elif "timestamp-header" in fname:
                    path = self._git_dir / rel_path
                    path.parent.mkdir(parents=True, exist_ok=True)
                    path.symlink_to(self._flatten_dir / rel_path)
                    log.info(f"symlink {path} to {self._flatten_dir / rel_path}")
                else:
                    raise ValueError(f"Unknown file {str(rel_path)}")

        for t in ["raw", "gzip-nbt"]:
            for item in index_json.get(t, []):
                assert isinstance(item["input_path"], str)
                assert isinstance(item["output_path"], str)
                target_path: Path = self._flatten_dir / item["output_path"]
                source_path: Path = self._git_dir / t / item["id"]
                source_path.parent.mkdir(parents=True, exist_ok=True)
                source_path.symlink_to(target_path)
                log.info(f"symlink {source_path} to {target_path}")

        (self._git_dir / "index.json").symlink_to(self._flatten_dir / "index.json")

    def patch(self):
        if self._flatten_dir.exists() and any(self._flatten_dir.iterdir()):
            raise RuntimeError(f"Output directory {self._flatten_dir} is not empty")

        self._flatten_dir.mkdir(parents=True, exist_ok=True)

        index_path = self._git_dir / "index.json"
        if not index_path.exists():
            raise RuntimeError("index.json not found in git dir, cannot patch")

        with index_path.open("r") as f:
            index_json = json.load(f)

        self._chunk_manager.batch_generate([tuple(c) for c in index_json["chunks"]])

        for i, item in enumerate(index_json.get("region", [])):
            for j, rel_path in enumerate(item["output_paths"]):
                diff_file = self._git_dir / rel_path
                target_file = self._flatten_dir / rel_path
                target_file.parent.mkdir(parents=True, exist_ok=True)

                fname = Path(rel_path).name
                if "chunk" in fname:
                    if "region" in item["input_path"]:
                        cm = re.search(r"chunk-x(-?\d+)-z(-?\d+)", fname)
                        if not cm:
                            raise ValueError(f"Cannot parse coordinates from {fname}")
                        chunk_x, chunk_z = int(cm.group(1)), int(cm.group(2))

                        base = self._chunk_manager.get(chunk_x, chunk_z)
                        if not base:
                            raise RuntimeError(
                                f"Base data missing for ({chunk_x}, {chunk_z})"
                            )

                        delta = diff_file.read_bytes()
                        log.debug(
                            "decode",
                            diff_file=diff_file,
                            base_sha256=sha256(base).hexdigest(),
                            diff_sha256=sha256(delta).hexdigest(),
                        )
                        patched_data = decode(base, delta)

                        target_file.write_bytes(patched_data)
                        log.info(f"patched chunk: {target_file}")
                    else:
                        shutil.copy2(diff_file, target_file)
                        log.info(f"copied non-region chunk: {target_file}")

                elif "timestamp-header" in fname:
                    shutil.copy2(diff_file, target_file)
                    log.info(f"restored header: {target_file}")
                else:
                    raise ValueError(f"Unknown file type in index: {rel_path}")

        for t in ["raw", "gzip-nbt"]:
            for item in index_json.get(t, []):
                rel_output_path = item["output_path"]
                diff_source = self._git_dir / t / item["id"]
                target_file = self._flatten_dir / rel_output_path

                target_file.parent.mkdir(parents=True, exist_ok=True)
                shutil.copy2(diff_source, target_file)
                log.info(f"restored {t} file: {target_file}")

        shutil.copy2(index_path, self._flatten_dir / "index.json")


if __name__ == "__main__":
    flatten_path = "/home/hlsvillager/Desktop/superflat/temp/flatten"
    diff_path = "/home/hlsvillager/Desktop/superflat/temp/diff"
    patch_path = "/home/hlsvillager/Desktop/superflat/temp/patch"
    differ = Differ(42, Path(flatten_path), Path(diff_path))
    differ.diff()
    differ = Differ(42, Path(patch_path), Path(diff_path))
    differ.patch()
