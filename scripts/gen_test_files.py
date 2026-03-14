#!/usr/bin/env python3
"""Generate test text files simulating human editing operations.

Each file is a modified version of the previous, producing a version history
suitable for testing delta compression algorithms (e.g. git pack objects).
"""

from __future__ import annotations

import random
from pathlib import Path
from typing import Annotated

import typer

# ---------------------------------------------------------------------------
# Corpus – realistic-ish prose so deltas are meaningful
# ---------------------------------------------------------------------------

WORDS = (
    "the quick brown fox jumps over the lazy dog "
    "pack my box with five dozen liquor jugs "
    "how vexingly dull gymnastic frogs "
    "sphinx of black quartz judge my vow "
    "two driven jocks help fax my big quiz "
    "the five boxing wizards jump quickly "
    "jackdaws love my big sphinx of quartz "
    "we promptly judged antique ivory buckles "
    "crazy frederick bought many very exquisite opal jewels "
    "a mad boxer shot a quick gloved jab to the jaw "
).split()

PARAGRAPH_TEMPLATES = [
    "In {year}, the project was initialized with {count} modules.",
    "The algorithm processes each {noun} in O(n log n) time.",
    "Configuration values are stored in the {noun} registry.",
    "Error handling is delegated to the {noun} subsystem.",
    "Performance benchmarks show a {count}% improvement over baseline.",
    "The {noun} interface exposes {count} public methods.",
    "Commit {sha} introduced support for {noun} compression.",
    "All {noun} objects are serialized using the pack format.",
    "The delta encoder computes the diff between {noun} versions.",
    "Garbage collection runs every {count} seconds by default.",
    "Blob {sha} contains the canonical {noun} implementation.",
    "The {noun} cache is invalidated on each write operation.",
    "Unit tests cover {count} of the {noun} code paths.",
    "Legacy support for {noun} format was removed in v{count}.",
    "The {noun} layer abstracts filesystem and network I/O.",
]

NOUNS = [
    "object", "blob", "tree", "commit", "pack", "index", "delta",
    "reference", "buffer", "stream", "cache", "config", "repo",
    "branch", "tag", "remote", "hook", "filter", "transport",
]


# ---------------------------------------------------------------------------
# Text generation helpers
# ---------------------------------------------------------------------------

def _random_sha(rng: random.Random) -> str:
    return "".join(rng.choices("0123456789abcdef", k=8))


def _random_line(rng: random.Random) -> str:
    tmpl = rng.choice(PARAGRAPH_TEMPLATES)
    return tmpl.format(
        year=rng.randint(2018, 2025),
        count=rng.randint(1, 999),
        noun=rng.choice(NOUNS),
        sha=_random_sha(rng),
    )


def _random_sentence(rng: random.Random, min_words: int = 6, max_words: int = 14) -> str:
    n = rng.randint(min_words, max_words)
    words = [rng.choice(WORDS) for _ in range(n)]
    words[0] = words[0].capitalize()
    return " ".join(words) + "."


def _make_base_document(rng: random.Random, num_lines: int) -> list[str]:
    """Generate the initial document as a list of lines."""
    lines: list[str] = []
    while len(lines) < num_lines:
        # Alternate between structured lines and prose sentences
        if rng.random() < 0.5:
            lines.append(_random_line(rng))
        else:
            lines.append(_random_sentence(rng))
        # Occasionally add a blank line (paragraph break)
        if rng.random() < 0.15:
            lines.append("")
    return lines


# ---------------------------------------------------------------------------
# Human editing operations
# ---------------------------------------------------------------------------

def _op_insert(lines: list[str], rng: random.Random) -> list[str]:
    """Insert 1-4 new lines at a random position."""
    pos = rng.randint(0, len(lines))
    count = rng.randint(1, 4)
    new_lines = [_random_line(rng) if rng.random() < 0.6 else _random_sentence(rng)
                 for _ in range(count)]
    return lines[:pos] + new_lines + lines[pos:]


def _op_delete(lines: list[str], rng: random.Random) -> list[str]:
    """Delete 1-5 consecutive lines at a random position."""
    if len(lines) < 5:
        return lines
    pos = rng.randint(0, len(lines) - 1)
    count = rng.randint(1, min(5, len(lines) - pos))
    return lines[:pos] + lines[pos + count:]


def _op_modify(lines: list[str], rng: random.Random) -> list[str]:
    """Modify 1-3 existing lines (word substitution)."""
    if not lines:
        return lines
    result = lines[:]
    num_mods = rng.randint(1, min(3, len([l for l in lines if l])))
    non_blank = [i for i, l in enumerate(result) if l]
    if not non_blank:
        return result
    for idx in rng.sample(non_blank, min(num_mods, len(non_blank))):
        words = result[idx].split()
        if not words:
            continue
        # Replace one random word
        word_idx = rng.randint(0, len(words) - 1)
        words[word_idx] = rng.choice(WORDS)
        result[idx] = " ".join(words)
    return result


def _op_move_block(lines: list[str], rng: random.Random) -> list[str]:
    """Cut a block of 2-6 lines and paste it elsewhere."""
    if len(lines) < 8:
        return lines
    src = rng.randint(0, len(lines) - 4)
    size = rng.randint(2, min(6, len(lines) - src))
    block = lines[src:src + size]
    remaining = lines[:src] + lines[src + size:]
    dst = rng.randint(0, len(remaining))
    return remaining[:dst] + block + remaining[dst:]


def _op_append(lines: list[str], rng: random.Random) -> list[str]:
    """Append a small paragraph at the end."""
    count = rng.randint(2, 5)
    new_lines = [""] + [_random_sentence(rng) for _ in range(count)]
    return lines + new_lines


def _op_replace_section(lines: list[str], rng: random.Random) -> list[str]:
    """Replace a block of lines with freshly generated content."""
    if len(lines) < 6:
        return lines
    pos = rng.randint(0, len(lines) - 3)
    size = rng.randint(2, min(6, len(lines) - pos))
    replacement = [_random_line(rng) for _ in range(size)]
    return lines[:pos] + replacement + lines[pos + size:]


def _op_duplicate_block(lines: list[str], rng: random.Random) -> list[str]:
    """Copy a block of lines and insert the copy nearby (copy-paste pattern)."""
    if len(lines) < 4:
        return lines
    src = rng.randint(0, len(lines) - 2)
    size = rng.randint(2, min(5, len(lines) - src))
    block = lines[src:src + size]
    dst = rng.randint(src + size, len(lines))
    return lines[:dst] + block + lines[dst:]


OPERATIONS = [
    (_op_insert,          0.25),
    (_op_delete,          0.15),
    (_op_modify,          0.25),
    (_op_move_block,      0.10),
    (_op_append,          0.10),
    (_op_replace_section, 0.10),
    (_op_duplicate_block, 0.05),
]

_OPS, _WEIGHTS = zip(*OPERATIONS)


def _apply_edits(lines: list[str], rng: random.Random, num_ops: int) -> list[str]:
    """Apply `num_ops` randomly chosen editing operations."""
    for op in rng.choices(_OPS, weights=_WEIGHTS, k=num_ops):
        lines = op(lines, rng)
    return lines


# ---------------------------------------------------------------------------
# CLI
# ---------------------------------------------------------------------------

app = typer.Typer(help=__doc__, add_completion=False)


@app.command()
def generate(
    output_dir: Annotated[
        Path,
        typer.Argument(help="Directory to write generated files into."),
    ],
    num_files: Annotated[
        int,
        typer.Option("--num-files", "-n", help="Number of files to generate.", min=1),
    ] = 10,
    seed: Annotated[
        int,
        typer.Option("--seed", "-s", help="Random seed for reproducibility."),
    ] = 42,
    base_lines: Annotated[
        int,
        typer.Option("--base-lines", "-l", help="Approximate line count of the initial document.", min=5),
    ] = 80,
    edits_per_step: Annotated[
        int,
        typer.Option("--edits", "-e", help="Number of editing operations applied per file version.", min=1),
    ] = 6,
    prefix: Annotated[
        str,
        typer.Option("--prefix", "-p", help="Filename prefix (files are named <prefix>_NNN.txt)."),
    ] = "doc",
    verbose: Annotated[
        bool,
        typer.Option("--verbose", "-v", help="Print stats for each generated file."),
    ] = False,
) -> None:
    """Generate a series of text files in OUTPUT_DIR that simulate human editing."""

    output_dir.mkdir(parents=True, exist_ok=True)

    rng = random.Random(seed)

    typer.echo(f"Seed: {seed}  files: {num_files}  base_lines: {base_lines}  edits/step: {edits_per_step}")

    # Build the initial document
    lines = _make_base_document(rng, base_lines)

    width = len(str(num_files - 1))

    for i in range(num_files):
        if i > 0:
            lines = _apply_edits(lines, rng, edits_per_step)

        filename = output_dir / f"{prefix}_{i:0{width}d}.txt"
        content = "\n".join(lines) + "\n"
        filename.write_text(content, encoding="utf-8")

        if verbose:
            typer.echo(
                f"  {filename.name}  lines={len(lines):4d}  bytes={len(content.encode()):6d}"
            )
        else:
            typer.echo(f"  wrote {filename.name}")

    typer.echo(f"\nDone. {num_files} files written to {output_dir}/")


if __name__ == "__main__":
    app()
