import argparse
import os
import sys
import zlib


def main():
    # 1. Setup Argument Parser
    parser = argparse.ArgumentParser(
        description="Concatenate multiple files and calculate the total zlib compressed size."
    )

    # Accept one or more filenames
    parser.add_argument(
        "files",
        metavar="FILE",
        type=str,
        nargs="+",
        help="Paths to the files to be processed",
    )

    # Compression level (0-9)
    parser.add_argument(
        "-l",
        "--level",
        type=int,
        choices=range(0, 10),
        default=6,
        help="zlib compression level (0=none, 1=fastest, 9=best, default=6)",
    )

    # Verbose mode
    parser.add_argument(
        "-v",
        "--verbose",
        action="store_true",
        help="Show progress for individual files",
    )

    args = parser.parse_args()

    combined_data = bytearray()
    total_original_size = 0

    # 2. Read and concatenate binary data
    print(f"[*] Reading {len(args.files)} file(s)...")

    for file_path in args.files:
        if not os.path.exists(file_path):
            print(f"[!] Error: File not found -> {file_path}", file=sys.stderr)
            continue

        try:
            with open(file_path, "rb") as f:
                data = f.read()
                combined_data.extend(data)
                total_original_size += len(data)
                if args.verbose:
                    print(f"    - Loaded: {file_path} ({len(data):,} bytes)")
        except Exception as e:
            print(f"[!] Could not read {file_path}: {e}", file=sys.stderr)

    if total_original_size == 0:
        print("[!] No valid data collected.")
        return

    # 3. Perform Compression
    print(f"[*] Compressing with level {args.level}...")
    try:
        compressed_data = zlib.compress(combined_data, level=args.level)
        compressed_size = len(compressed_data)

        # 4. Output Results
        ratio = (
            (compressed_size / total_original_size) * 100
            if total_original_size > 0
            else 0
        )

        print("\n" + "=" * 40)
        print(f"{'Original Total:':<20} {total_original_size:>15,} bytes")
        print(f"{'Compressed Size:':<20} {compressed_size:>15,} bytes")
        print(f"{'Compression Ratio:':<20} {ratio:>14.2f}%")
        print(
            f"{'Space Saved:':<20} {total_original_size - compressed_size:>15,} bytes"
        )
        print("=" * 40)

    except Exception as e:
        print(f"[!] Compression failed: {e}", file=sys.stderr)


if __name__ == "__main__":
    main()
