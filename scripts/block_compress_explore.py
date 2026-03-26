"""
Explore compression strategies for block section dumps.

Input files: raw u16-LE block state IDs (8192 bytes = 4096 blocks per section)
Layout: cube[y][z][x], flattened as Y-outer, Z-middle, X-inner (Y slowest)

Usage:
    python block_compress_explore.py /tmp/dump_*.bin
"""

import argparse
import struct
import zlib
from collections import Counter
from itertools import chain

BLOCKS_PER_SECTION = 4096
DIM = 16


def load_dump(path: str) -> list[int]:
    with open(path, "rb") as f:
        data = f.read()
    assert len(data) == BLOCKS_PER_SECTION * 2, f"Expected 8192 bytes, got {len(data)}"
    return list(struct.unpack_from(f"<{BLOCKS_PER_SECTION}H", data))


def blocks_to_bytes_le(blocks: list[int]) -> bytes:
    return struct.pack(f"<{len(blocks)}H", *blocks)


# ── Transformations ──────────────────────────────────────────────────────────

def t_raw(sections: list[list[int]]) -> bytes:
    """Baseline: raw u16-LE concatenation."""
    return b"".join(blocks_to_bytes_le(s) for s in sections)


def t_byte_split(sections: list[list[int]]) -> bytes:
    """Separate low bytes and high bytes across all sections."""
    all_blocks = list(chain.from_iterable(sections))
    lo = bytes(v & 0xFF for v in all_blocks)
    hi = bytes(v >> 8   for v in all_blocks)
    return lo + hi


def t_delta_u16(sections: list[list[int]]) -> bytes:
    """Delta encode within each section, then u16-LE."""
    out = []
    for s in sections:
        deltas = [s[0]] + [(s[i] - s[i-1]) & 0xFFFF for i in range(1, len(s))]
        out.append(blocks_to_bytes_le(deltas))
    return b"".join(out)


def t_delta_byte_split(sections: list[list[int]]) -> bytes:
    """Delta encode then byte-split."""
    all_deltas: list[int] = []
    for s in sections:
        all_deltas.append(s[0])
        for i in range(1, len(s)):
            all_deltas.append((s[i] - s[i-1]) & 0xFFFF)
    lo = bytes(v & 0xFF for v in all_deltas)
    hi = bytes(v >> 8   for v in all_deltas)
    return lo + hi


def t_xor_delta(sections: list[list[int]]) -> bytes:
    """XOR delta within each section, then byte-split."""
    all_deltas: list[int] = []
    for s in sections:
        all_deltas.append(s[0])
        for i in range(1, len(s)):
            all_deltas.append(s[i] ^ s[i-1])
    lo = bytes(v & 0xFF for v in all_deltas)
    hi = bytes(v >> 8   for v in all_deltas)
    return lo + hi


def t_reorder_yzx_to_xzy(sections: list[list[int]]) -> bytes:
    """
    Reorder spatial traversal: current is Y-outer Z-middle X-inner.
    New: X-outer Z-middle Y-inner — groups vertical columns (same XZ, all Y).
    Then byte-split.
    """
    all_blocks: list[int] = []
    for s in sections:
        # s[i] = block at y=i//256, z=(i//16)%16, x=i%16
        # Reorder to x-outer, z-middle, y-inner
        cube = [[[s[y * 256 + z * 16 + x] for y in range(DIM)] for z in range(DIM)] for x in range(DIM)]
        for x in range(DIM):
            for z in range(DIM):
                for y in range(DIM):
                    all_blocks.append(cube[x][z][y])
    lo = bytes(v & 0xFF for v in all_blocks)
    hi = bytes(v >> 8   for v in all_blocks)
    return lo + hi


def t_cross_section_transpose(sections: list[list[int]]) -> bytes:
    """
    Interleave blocks at the same position across all sections.
    pos0_sec0, pos0_sec1, ..., pos0_secN, pos1_sec0, ...
    Then byte-split.
    """
    n = len(sections)
    all_blocks = [sections[s][p] for p in range(BLOCKS_PER_SECTION) for s in range(n)]
    lo = bytes(v & 0xFF for v in all_blocks)
    hi = bytes(v >> 8   for v in all_blocks)
    return lo + hi


def t_cross_transpose_yzx_to_xzy(sections: list[list[int]]) -> bytes:
    """
    Cross-section transpose after spatial reorder to XZY.
    Groups: same (x,z) column across all sections and all Y layers.
    Then byte-split.
    """
    n = len(sections)
    all_blocks: list[int] = []
    for x in range(DIM):
        for z in range(DIM):
            for si in range(n):
                s = sections[si]
                for y in range(DIM):
                    all_blocks.append(s[y * 256 + z * 16 + x])
    lo = bytes(v & 0xFF for v in all_blocks)
    hi = bytes(v >> 8   for v in all_blocks)
    return lo + hi


TRANSFORMS = [
    ("raw",                      t_raw),
    ("byte_split",               t_byte_split),
    ("delta_u16",                t_delta_u16),
    ("delta+byte_split",         t_delta_byte_split),
    ("xor_delta+byte_split",     t_xor_delta),
    ("reorder_XZY+byte_split",   t_reorder_yzx_to_xzy),
    ("cross_transpose+split",    t_cross_section_transpose),
    ("cross_XZY_transpose+split",t_cross_transpose_yzx_to_xzy),
]

# ── Stats ────────────────────────────────────────────────────────────────────

def print_data_stats(sections: list[list[int]]):
    all_blocks = list(chain.from_iterable(sections))
    total = len(all_blocks)
    counts = Counter(all_blocks)
    top10 = counts.most_common(10)
    n_unique = len(counts)
    n_zero = counts.get(0, 0)
    hi_nonzero = sum(1 for v in all_blocks if v >> 8 != 0)

    print(f"  Sections: {len(sections)}, total blocks: {total:,}")
    print(f"  Unique IDs: {n_unique}")
    print(f"  Zero (air?): {n_zero:,} ({n_zero/total*100:.1f}%)")
    print(f"  High byte ≠ 0: {hi_nonzero:,} ({hi_nonzero/total*100:.1f}%)")
    print(f"  Top 10 IDs: {top10}")


# ── Main ─────────────────────────────────────────────────────────────────────

def zsize(data: bytes, level: int = 6) -> int:
    return len(zlib.compress(data, level=level))


def main():
    parser = argparse.ArgumentParser(description=__doc__, formatter_class=argparse.RawDescriptionHelpFormatter)
    parser.add_argument("files", metavar="FILE", nargs="+")
    parser.add_argument("-l", "--level", type=int, default=6, choices=range(10))
    parser.add_argument("--best", action="store_true", help="Also run with level=9")
    args = parser.parse_args()

    sections = [load_dump(f) for f in args.files]
    raw_bytes = len(args.files) * BLOCKS_PER_SECTION * 2

    print(f"\n=== Data stats ===")
    print_data_stats(sections)

    levels = [args.level]
    if args.best:
        levels = sorted(set(levels + [9]))

    for level in levels:
        print(f"\n=== Compression results (zlib level={level}) ===")
        print(f"{'Transform':<30} {'Compressed':>12} {'Ratio':>8}  vs raw")
        print("-" * 65)

        baseline = None
        for name, fn in TRANSFORMS:
            data = fn(sections)
            assert len(data) == raw_bytes, f"{name}: size mismatch {len(data)} vs {raw_bytes}"
            compressed = zsize(data, level)
            ratio = compressed / raw_bytes * 100
            marker = ""
            if baseline is None:
                baseline = compressed
                marker = "  (baseline)"
            else:
                improvement = (baseline - compressed) / baseline * 100
                marker = f"  {improvement:+.1f}%"
            print(f"  {name:<28} {compressed:>12,}  {ratio:>6.2f}%{marker}")


if __name__ == "__main__":
    main()
