#!/usr/bin/env python3
"""
analyze_blobs.py - Analyze blob objects in a Git bare repository.

Usage:
    python analyze_blobs.py <bare_repo_path> [output.csv]

Output CSV columns:
    hash, storage_type, compressed_size, commit_hash, file_path

storage_type values:
    loose        - loose object file
    packed-base  - stored in a packfile as a full object
    packed-delta - stored in a packfile as a delta
"""

import csv
import os
import re
import struct
import subprocess
import sys
import zlib
from pathlib import Path


def run(args, cwd=None, capture=True):
    result = subprocess.run(args, cwd=cwd, capture_output=capture, text=True)
    if result.returncode != 0:
        raise RuntimeError(f"Command failed: {args}\n{result.stderr}")
    return result.stdout


def get_all_blobs(repo):
    """Return set of all blob hashes in the repo."""
    out = run(
        [
            "git",
            "cat-file",
            "--batch-all-objects",
            "--batch-check=%(objecttype) %(objectname)",
        ],
        cwd=repo,
    )
    blobs = set()
    for line in out.splitlines():
        parts = line.split()
        if len(parts) == 2 and parts[0] == "blob":
            blobs.add(parts[1])
    return blobs


# ---------------------------------------------------------------------------
# Loose object helpers
# ---------------------------------------------------------------------------


def get_loose_objects(repo):
    """Return dict {hash: compressed_size} for all loose blob objects."""
    objects_dir = Path(repo) / "objects"
    loose = {}
    for subdir in objects_dir.iterdir():
        if (
            not subdir.is_dir()
            or len(subdir.name) != 2
            or not all(c in "0123456789abcdef" for c in subdir.name)
        ):
            continue
        for obj_file in subdir.iterdir():
            if not obj_file.is_file():
                continue
            obj_hash = subdir.name + obj_file.name
            if len(obj_hash) != 40:
                continue
            loose[obj_hash] = obj_file.stat().st_size
    return loose


# ---------------------------------------------------------------------------
# Pack object helpers
# ---------------------------------------------------------------------------


def read_pack_object_type_and_size(f):
    """
    Read the type and uncompressed size from the pack object header at
    the current file position. Returns (type_num, uncompressed_size, header_bytes_read).

    Pack object types:
        1 = OBJ_COMMIT, 2 = OBJ_TREE, 3 = OBJ_BLOB,
        4 = OBJ_TAG,    6 = OBJ_OFS_DELTA, 7 = OBJ_REF_DELTA
    """
    byte = struct.unpack("B", f.read(1))[0]
    obj_type = (byte >> 4) & 0x7
    size = byte & 0x0F
    shift = 4
    header_bytes = 1
    while byte & 0x80:
        byte = struct.unpack("B", f.read(1))[0]
        size |= (byte & 0x7F) << shift
        shift += 7
        header_bytes += 1
    return obj_type, size, header_bytes


def get_packed_objects(repo):
    """
    Parse all packfiles and return:
        dict {hash: {"type": "packed-base"|"packed-delta", "compressed_size": int}}
    """
    pack_dir = Path(repo) / "objects" / "pack"
    result = {}

    if not pack_dir.exists():
        return result

    for idx_file in pack_dir.glob("*.idx"):
        pack_file = idx_file.with_suffix(".pack")
        if not pack_file.exists():
            continue

        # Read index v2: get (hash -> offset) mapping
        offsets = _parse_idx(idx_file)
        if offsets is None:
            continue

        _parse_pack(pack_file, offsets, result)

    return result


def _parse_idx(idx_path):
    """Parse a pack index file (v2) and return {hash: offset} dict."""
    with open(idx_path, "rb") as f:
        magic = f.read(4)
        if magic == b"\xff\x74\x4f\x63":
            version = struct.unpack(">I", f.read(4))[0]
            if version != 2:
                return None
            # Fan-out table (256 * 4 bytes)
            fan_out = struct.unpack(">256I", f.read(256 * 4))
            n_objects = fan_out[255]
            # Hashes
            hashes = []
            for _ in range(n_objects):
                hashes.append(f.read(20).hex())
            # CRCs (skip)
            f.read(4 * n_objects)
            # 32-bit offsets
            small_offsets = struct.unpack(f">{n_objects}I", f.read(4 * n_objects))
            # Large offset table (for offsets >= 2^31)
            large_offset_entries = []
            # Collect entries that need large offsets
            needs_large = [
                (i, o) for i, o in enumerate(small_offsets) if o & 0x80000000
            ]
            # Read remaining data for large offsets
            large_data = f.read()
            large_offsets_table = []
            for i in range(0, len(large_data) - 40, 8):  # -40 for two SHA1s at end
                large_offsets_table.append(
                    struct.unpack(">Q", large_data[i : i + 8])[0]
                )

            offsets = {}
            for i, h in enumerate(hashes):
                off = small_offsets[i]
                if off & 0x80000000:
                    large_idx = off & 0x7FFFFFFF
                    if large_idx < len(large_offsets_table):
                        off = large_offsets_table[large_idx]
                    else:
                        continue
                offsets[h] = off
            return offsets
        else:
            # v1 index
            f.seek(0)
            fan_out = struct.unpack(">256I", f.read(256 * 4))
            n_objects = fan_out[255]
            offsets = {}
            for _ in range(n_objects):
                offset = struct.unpack(">I", f.read(4))[0]
                obj_hash = f.read(20).hex()
                offsets[obj_hash] = offset
            return offsets


def _parse_pack(pack_path, offsets, result):
    """
    For each object in the pack, determine type (base/delta) and compressed size.
    Populates result in-place.
    """
    # Build a sorted list of (offset, hash) to compute compressed sizes
    sorted_entries = sorted((off, h) for h, off in offsets.items())

    pack_size = pack_path.stat().st_size
    # Trailer is 20 bytes SHA1 checksum
    data_end = pack_size - 20

    with open(pack_path, "rb") as f:
        # Validate pack header
        header = f.read(4)
        if header != b"PACK":
            return
        _version = struct.unpack(">I", f.read(4))[0]
        _num_objects = struct.unpack(">I", f.read(4))[0]

        for idx, (offset, obj_hash) in enumerate(sorted_entries):
            # Next object's offset (or end of data) gives us the boundary
            if idx + 1 < len(sorted_entries):
                next_offset = sorted_entries[idx + 1][0]
            else:
                next_offset = data_end

            f.seek(offset)
            obj_type, _uncompressed_size, header_bytes_read = (
                read_pack_object_type_and_size(f)
            )

            # For delta objects, skip the base reference bytes
            extra = 0
            if obj_type == 6:  # OBJ_OFS_DELTA
                # Read variable-length negative offset
                b = struct.unpack("B", f.read(1))[0]
                extra = 1
                while b & 0x80:
                    b = struct.unpack("B", f.read(1))[0]
                    extra += 1
            elif obj_type == 7:  # OBJ_REF_DELTA
                extra = 20  # 20-byte base object SHA1

            compressed_start = offset + header_bytes_read + extra
            compressed_size = next_offset - compressed_start

            # Map type number to storage type string
            OFS_DELTA = 6
            REF_DELTA = 7
            if obj_type in (OFS_DELTA, REF_DELTA):
                storage_type = "packed-delta"
            else:
                storage_type = "packed-base"

            result[obj_hash] = {
                "type": storage_type,
                "compressed_size": max(0, compressed_size),
            }


# ---------------------------------------------------------------------------
# Blob -> commit/path mapping
# ---------------------------------------------------------------------------


def build_blob_to_commit_path(repo):
    """
    Walk all commits and map each blob hash to (commit_hash, file_path).
    If a blob appears in multiple commits, the earliest (first parent) commit wins.
    Returns dict {blob_hash: (commit_hash, file_path)}.
    """
    # Get commits in reverse chronological order (newest first)
    commit_list = run(["git", "log", "--all", "--format=%H"], cwd=repo).splitlines()

    blob_map = {}  # blob_hash -> (commit_hash, file_path)

    for commit_hash in commit_list:
        # List all blobs in this commit's tree
        try:
            tree_out = run(["git", "ls-tree", "-r", "--long", commit_hash], cwd=repo)
        except RuntimeError:
            continue
        for line in tree_out.splitlines():
            # Format: <mode> <type> <hash> <size>\t<path>
            parts = line.split(None, 4)
            if len(parts) < 4:
                continue
            obj_type = parts[1]
            obj_hash = parts[2]
            if obj_type != "blob":
                continue
            file_path = parts[4] if len(parts) == 5 else ""
            # Keep earliest commit (we iterate newest-first, so always overwrite)
            blob_map[obj_hash] = (commit_hash, file_path)

    return blob_map


# ---------------------------------------------------------------------------
# Main
# ---------------------------------------------------------------------------


def main():
    if len(sys.argv) < 2:
        print(__doc__)
        sys.exit(1)

    repo = sys.argv[1]
    output_csv = sys.argv[2] if len(sys.argv) >= 3 else "blobs.csv"

    if not Path(repo).exists():
        print(f"Error: repository path does not exist: {repo}", file=sys.stderr)
        sys.exit(1)

    # Verify it's a git repo
    try:
        run(["git", "rev-parse", "--git-dir"], cwd=repo)
    except RuntimeError:
        print(f"Error: not a git repository: {repo}", file=sys.stderr)
        sys.exit(1)

    print("Collecting all blob hashes...")
    all_blobs = get_all_blobs(repo)
    print(f"  Found {len(all_blobs)} blobs")

    print("Scanning loose objects...")
    loose = get_loose_objects(repo)

    print("Scanning packfiles...")
    packed = get_packed_objects(repo)

    print("Building blob -> commit/path map (this may take a while)...")
    blob_to_commit_path = build_blob_to_commit_path(repo)

    print(f"Writing results to {output_csv}...")
    rows_written = 0

    with open(output_csv, "w", newline="", encoding="utf-8") as csvfile:
        writer = csv.writer(csvfile)
        writer.writerow(
            ["hash", "storage_type", "compressed_size", "commit_hash", "file_path"]
        )

        rows = []
        for blob_hash in sorted(all_blobs):
            if blob_hash in loose:
                storage_type = "loose"
                compressed_size = loose[blob_hash]
            elif blob_hash in packed:
                storage_type = packed[blob_hash]["type"]
                compressed_size = packed[blob_hash]["compressed_size"]
            else:
                storage_type = "unknown"
                compressed_size = -1

            commit_hash, file_path = blob_to_commit_path.get(blob_hash, ("", ""))
            rows.append(
                (blob_hash, storage_type, compressed_size, commit_hash, file_path)
            )

        rows.sort(key=lambda x: x[4])

        writer.writerows(rows)
        rows_written += len(rows)

    print(f"Done. {rows_written} rows written to {output_csv}")

    # Print a quick summary
    type_counts = {}
    for blob_hash in all_blobs:
        if blob_hash in loose:
            t = "loose"
        elif blob_hash in packed:
            t = packed[blob_hash]["type"]
        else:
            t = "unknown"
        type_counts[t] = type_counts.get(t, 0) + 1

    print("\nSummary:")
    for t, count in sorted(type_counts.items()):
        print(f"  {t}: {count}")


if __name__ == "__main__":
    main()
