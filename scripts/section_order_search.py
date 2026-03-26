# /// script
# requires-python = ">=3.11"
# dependencies = ["numpy"]
# ///
"""
Search for optimal section ordering to maximize zlib compression.

Loads 8×8×24 block section dumps (u16-LE, 8192 bytes each).
Treats them as nodes in a 3D grid (cx, cz, sy) with 6-directional adjacency.
Tries several traversal strategies and measures compressed size.

Usage:
    python section_order_search.py /tmp/sections/
"""

import argparse
import struct
import time
import zlib
from collections import Counter
from pathlib import Path

import numpy as np

BLOCKS = 4096
SECTION_BYTES = BLOCKS * 2
ZLIB_LEVEL_SEARCH = 1   # fast during search
ZLIB_LEVEL_EVAL   = 6   # accurate for final results
CONTEXT_WINDOW    = 32768  # bytes of context kept per beam state


# ── I/O ───────────────────────────────────────────────────────────────────────

def load_sections(directory: str):
    """Return list of (cx, cz, sy, data) tuples and a coord→index dict."""
    sections = []
    for path in sorted(Path(directory).glob("*.bin")):
        parts = path.stem.split("_")
        if len(parts) != 3:
            continue
        cx, cz, sy = int(parts[0]), int(parts[1]), int(parts[2])
        data = path.read_bytes()
        if len(data) != SECTION_BYTES:
            continue
        sections.append((cx, cz, sy, data))
    grid = {(cx, cz, sy): i for i, (cx, cz, sy, _) in enumerate(sections)}
    return sections, grid


# ── Similarity matrix (cosine on block histograms) ────────────────────────────

def build_similarity(sections) -> np.ndarray:
    """
    Cosine similarity between section block-ID histograms.
    Result[i,j] ∈ [0,1]: 1 = identical distributions.
    """
    all_ids = set()
    histograms = []
    for _, _, _, data in sections:
        vals = struct.unpack_from(f"<{BLOCKS}H", data)
        h = Counter(vals)
        histograms.append(h)
        all_ids |= h.keys()

    id_list = sorted(all_ids)
    id_index = {v: i for i, v in enumerate(id_list)}
    N, D = len(sections), len(id_list)

    H = np.zeros((N, D), dtype=np.float32)
    for i, h in enumerate(histograms):
        for v, cnt in h.items():
            H[i, id_index[v]] = cnt

    norms = np.linalg.norm(H, axis=1, keepdims=True)
    norms[norms == 0] = 1.0
    H /= norms
    return (H @ H.T).astype(np.float32)


def top_k_similar(sim: np.ndarray, k=20) -> list[list[int]]:
    """Precompute top-k most similar sections for each section (excluding self)."""
    N = sim.shape[0]
    result = []
    for i in range(N):
        row = sim[i].copy()
        row[i] = -1.0  # exclude self
        top = np.argpartition(row, -k)[-k:]
        top = top[np.argsort(row[top])[::-1]]
        result.append(top.tolist())
    return result


# ── Compression helpers ───────────────────────────────────────────────────────

def compress_order(order: list[int], sections, level=ZLIB_LEVEL_EVAL) -> int:
    blob = b"".join(sections[i][3] for i in order)
    return len(zlib.compress(blob, level=level))


def incremental_cost(context: bytes, new_data: bytes, level=ZLIB_LEVEL_SEARCH) -> int:
    """How many compressed bytes does new_data add given the current context?"""
    return len(zlib.compress(context + new_data, level)) - len(zlib.compress(context, level))


# ── Morton (Z-order) curve for 2D ────────────────────────────────────────────

def _interleave(n: int) -> int:
    n &= 0xFF
    n = (n | (n << 4)) & 0x0F0F
    n = (n | (n << 2)) & 0x3333
    n = (n | (n << 1)) & 0x5555
    return n

def morton2d(x: int, z: int) -> int:
    return _interleave(x) | (_interleave(z) << 1)


# ── Deterministic orderings ───────────────────────────────────────────────────

def order_row_major(sections, _grid):
    """cx → cz → sy (default C-order scan)."""
    return sorted(range(len(sections)), key=lambda i: (sections[i][0], sections[i][1], sections[i][2]))

def order_col_first(sections, _grid):
    """(cx, cz) outer, sy inner — vertical columns together."""
    return sorted(range(len(sections)), key=lambda i: (sections[i][0], sections[i][1], sections[i][2]))
    # same as row_major if cx/cz are outer; explicitly: sort by (cx,cz,sy)

def order_sy_first(sections, _grid):
    """sy outer, then (cx, cz) — horizontal slices together."""
    return sorted(range(len(sections)), key=lambda i: (sections[i][2], sections[i][0], sections[i][1]))

def order_morton_col(sections, _grid):
    """Morton curve on (cx, cz), sy inner within each column."""
    return sorted(range(len(sections)), key=lambda i: (morton2d(sections[i][0], sections[i][1]), sections[i][2]))

def order_morton_slice(sections, _grid):
    """sy outer, then Morton curve on (cx, cz) within each slice."""
    return sorted(range(len(sections)), key=lambda i: (sections[i][2], morton2d(sections[i][0], sections[i][1])))


# ── Greedy search (histogram similarity + 3D neighbor priority) ───────────────

def greedy_hist(sections, grid, top_k, start=0):
    """
    Greedy nearest-neighbor using precomputed histogram similarity.
    At each step: prefer unvisited 3D neighbors, else fall back to top-K similar.
    """
    N = len(sections)
    visited = bytearray(N)
    order = [start]
    visited[start] = 1

    for _ in range(N - 1):
        cur = order[-1]
        cx, cz, sy = sections[cur][:3]

        # 6-directional neighbors
        candidates = []
        for dx, dz, ds in [(1,0,0),(-1,0,0),(0,1,0),(0,-1,0),(0,0,1),(0,0,-1)]:
            nb = grid.get((cx+dx, cz+dz, sy+ds))
            if nb is not None and not visited[nb]:
                candidates.append(nb)

        # Fallback to top-K similar
        if not candidates:
            candidates = [j for j in top_k[cur] if not visited[j]]

        # Last resort: any unvisited (shouldn't happen often)
        if not candidates:
            candidates = [j for j in range(N) if not visited[j]]

        # Pick best among candidates by similarity to current section
        best = max(candidates, key=lambda j: top_k[cur].index(j) if j in top_k[cur] else len(top_k[cur]))
        # Rewrite: pick the one with highest similarity rank (lower index in top_k = more similar)
        # Among neighbors, pick the one appearing earliest in top_k[cur]
        sim_rank = {j: len(top_k[cur]) for j in candidates}
        for rank, j in enumerate(top_k[cur]):
            if j in sim_rank:
                sim_rank[j] = rank
        best = min(candidates, key=lambda j: sim_rank[j])

        visited[best] = 1
        order.append(best)

    return order


# ── Greedy search (actual zlib incremental scoring) ───────────────────────────

def greedy_zlib(sections, grid, top_k, start=0):
    """
    Greedy nearest-neighbor using actual incremental zlib cost.
    At each step: score top candidates (neighbors + top-K similar) and pick cheapest.
    """
    N = len(sections)
    visited = bytearray(N)
    order = [start]
    visited[start] = 1
    context = sections[start][3][-CONTEXT_WINDOW:]

    for step in range(N - 1):
        cur = order[-1]
        cx, cz, sy = sections[cur][:3]

        # Neighbors
        neighbors = []
        for dx, dz, ds in [(1,0,0),(-1,0,0),(0,1,0),(0,-1,0),(0,0,1),(0,0,-1)]:
            nb = grid.get((cx+dx, cz+dz, sy+ds))
            if nb is not None and not visited[nb]:
                neighbors.append(nb)

        # Top similar (unvisited)
        top_sim = [j for j in top_k[cur] if not visited[j]][:6]

        candidates = list(dict.fromkeys(neighbors + top_sim))  # dedup, preserve order

        if not candidates:
            # Last resort: first unvisited
            candidates = [next(j for j in range(N) if not visited[j])]

        # Score all candidates
        best = min(candidates, key=lambda j: incremental_cost(context, sections[j][3]))

        visited[best] = 1
        order.append(best)
        context = (context + sections[best][3])[-CONTEXT_WINDOW:]

        if (step + 1) % 100 == 0:
            print(f"  greedy_zlib: {step+1}/{N-1}", end="\r", flush=True)

    print()
    return order


# ── Beam search (actual zlib incremental scoring) ─────────────────────────────

def beam_search(sections, grid, top_k, beam_width=5, start=0):
    """
    Beam search over orderings, scored by sum of incremental zlib costs.
    State = (score, current_idx, visited_set, context_tail)
    """
    N = len(sections)

    # (score, cur, visited, context)
    beams = [(0, start, {start}, sections[start][3][-CONTEXT_WINDOW:])]

    for step in range(N - 1):
        next_beams = []

        for score, cur, visited, context in beams:
            cx, cz, sy = sections[cur][:3]

            # Generate candidates: 6 neighbors + top similar
            neighbors = []
            for dx, dz, ds in [(1,0,0),(-1,0,0),(0,1,0),(0,-1,0),(0,0,1),(0,0,-1)]:
                nb = grid.get((cx+dx, cz+dz, sy+ds))
                if nb is not None and nb not in visited:
                    neighbors.append(nb)

            top_sim = [j for j in top_k[cur] if j not in visited][:6]
            candidates = list(dict.fromkeys(neighbors + top_sim))

            if not candidates:
                # Fallback: pick best globally by histogram (rare)
                candidates = [j for j in top_k[0] if j not in visited][:3]
                if not candidates:
                    candidates = [next(j for j in range(N) if j not in visited)]

            for c in candidates:
                cost = incremental_cost(context, sections[c][3])
                new_context = (context + sections[c][3])[-CONTEXT_WINDOW:]
                new_visited = visited | {c}
                next_beams.append((score + cost, c, new_visited, new_context))

        # Keep best beam_width states
        next_beams.sort(key=lambda x: x[0])
        beams = next_beams[:beam_width]

        if (step + 1) % 100 == 0:
            best_score = beams[0][0]
            print(f"  beam_search: step {step+1}/{N-1}, best_score={best_score:,}", end="\r", flush=True)

    print()

    # Reconstruct best order by replaying greedy from best beam's state
    # (we don't store full order in beam state for memory efficiency)
    # Instead run greedy_zlib with same random seed from this start
    # Simple approach: just return the greedy order starting from the best beam's current node
    # For a more accurate result, store the order (memory trade-off)
    # Here we store order for correctness:
    return _beam_with_order(sections, grid, top_k, beam_width, start)


def _beam_with_order(sections, grid, top_k, beam_width, start):
    """Beam search that also tracks the full order (more memory)."""
    N = len(sections)
    # (score, cur, visited_frozenset, order_list, context)
    beams = [(0, start, frozenset([start]), [start], sections[start][3][-CONTEXT_WINDOW:])]

    for step in range(N - 1):
        next_beams = []

        for score, cur, visited, order, context in beams:
            cx, cz, sy = sections[cur][:3]

            neighbors = []
            for dx, dz, ds in [(1,0,0),(-1,0,0),(0,1,0),(0,-1,0),(0,0,1),(0,0,-1)]:
                nb = grid.get((cx+dx, cz+dz, sy+ds))
                if nb is not None and nb not in visited:
                    neighbors.append(nb)

            top_sim = [j for j in top_k[cur] if j not in visited][:6]
            candidates = list(dict.fromkeys(neighbors + top_sim))

            if not candidates:
                unvisited = [j for j in range(N) if j not in visited]
                if not unvisited:
                    continue
                candidates = unvisited[:1]

            for c in candidates:
                cost = incremental_cost(context, sections[c][3])
                new_context = (context + sections[c][3])[-CONTEXT_WINDOW:]
                next_beams.append((
                    score + cost,
                    c,
                    visited | {c},
                    order + [c],
                    new_context,
                ))

        next_beams.sort(key=lambda x: x[0])
        beams = next_beams[:beam_width]

        if (step + 1) % 50 == 0:
            print(f"  beam(W={beam_width}): step {step+1}/{N-1}, best≈{beams[0][0]:,}", end="\r", flush=True)

    print()
    return beams[0][2]  # best order


def compress_grouped(sections, group_size: int, level=ZLIB_LEVEL_EVAL) -> int:
    """
    Divide the (cx,cz) grid into group_size×group_size tiles.
    Within each tile, order sections by (sy, morton(cx%g, cz%g)) then compress independently.
    Returns the sum of compressed sizes across all groups.
    """
    groups: dict[tuple, list[int]] = {}
    for i, (cx, cz, sy, _) in enumerate(sections):
        key = (cx // group_size, cz // group_size)
        groups.setdefault(key, []).append(i)

    total = 0
    for indices in groups.values():
        ordered = sorted(indices, key=lambda i: (
            sections[i][2],                                          # sy
            morton2d(sections[i][0] % group_size, sections[i][1] % group_size),  # morton(cx%g, cz%g)
        ))
        blob = b"".join(sections[i][3] for i in ordered)
        total += len(zlib.compress(blob, level))
    return total



def fmt_bytes(n):
    return f"{n:,}"

def main():
    parser = argparse.ArgumentParser(description=__doc__, formatter_class=argparse.RawDescriptionHelpFormatter)
    parser.add_argument("directory", help="Directory with cx_cz_sy.bin files")
    parser.add_argument("--beam-width", type=int, default=5)
    parser.add_argument("--skip-beam", action="store_true", help="Skip beam search (slow)")
    parser.add_argument("--skip-greedy-zlib", action="store_true", help="Skip greedy-zlib (medium slow)")
    args = parser.parse_args()

    print("Loading sections...")
    sections, grid = load_sections(args.directory)
    N = len(sections)
    raw_total = N * SECTION_BYTES
    print(f"  {N} sections loaded, {raw_total:,} bytes raw")

    print("Building histogram similarity matrix...")
    t0 = time.time()
    sim = build_similarity(sections)
    topk = top_k_similar(sim, k=20)
    print(f"  done in {time.time()-t0:.1f}s")

    results = []

    def run(name, fn, *a, **kw):
        print(f"\n[{name}]")
        t0 = time.time()
        order = fn(sections, grid, *a, **kw)
        assert len(order) == N and len(set(order)) == N, "invalid order"
        compressed = compress_order(order, sections)
        ratio = compressed / raw_total * 100
        elapsed = time.time() - t0
        print(f"  compressed: {fmt_bytes(compressed)} bytes  ({ratio:.2f}%)  [{elapsed:.1f}s]")
        results.append((name, compressed, ratio))
        return order

    # Baselines
    run("row_major (cx→cz→sy)",            order_row_major)
    run("sy_first (sy→cx→cz)",             order_sy_first)
    run("morton_col (Z-curve, col-first)",  order_morton_col)
    run("morton_slice (Z-curve, slice-first)", order_morton_slice)

    # Greedy histogram
    run("greedy_hist", greedy_hist, topk)

    # Greedy zlib
    if not args.skip_greedy_zlib:
        run("greedy_zlib", greedy_zlib, topk)

    # Beam search
    if not args.skip_beam:
        run(f"beam_search (W={args.beam_width})", _beam_with_order, topk,
            beam_width=args.beam_width, start=0)

    # Summary
    # Baseline A: each section compressed individually
    individual_total = sum(
        len(zlib.compress(sections[i][3], ZLIB_LEVEL_EVAL)) for i in range(N)
    )
    # Baseline B: all concatenated in index order (unordered)
    concat_unordered = compress_order(list(range(N)), sections)

    # Grouped experiments: morton_slice within NxN chunk tiles
    grouped_results = []
    for g in [1, 2, 4, 8]:
        comp = compress_grouped(sections, group_size=g)
        grouped_results.append((g, comp))

    def vs(baseline, comp):
        return f"{(baseline - comp) / baseline * 100:>+.1f}%"

    print(f"\n{'='*72}")
    print(f"{'Strategy':<40} {'Compressed':>12} {'vs individual':>14} {'vs unordered':>13}")
    print(f"{'-'*72}")
    print(f"  {'individual (each section alone)':<38} {fmt_bytes(individual_total):>12}  {'(baseline)':>14}")
    print(f"  {'concat unordered':<38} {fmt_bytes(concat_unordered):>12}  "
          f"{vs(individual_total, concat_unordered):>14}  {'(baseline)':>13}")
    print()
    for name, comp, _ in results:
        print(f"  {name:<38} {fmt_bytes(comp):>12}  {vs(individual_total, comp):>14}  {vs(concat_unordered, comp):>13}")
    print()
    for g, comp in grouped_results:
        label = f"morton_slice grouped {g}×{g} chunk{'s' if g>1 else ''}"
        print(f"  {label:<38} {fmt_bytes(comp):>12}  {vs(individual_total, comp):>14}  {vs(concat_unordered, comp):>13}")
    print(f"{'='*72}")


if __name__ == "__main__":
    main()
