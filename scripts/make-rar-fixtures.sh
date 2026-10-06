#!/usr/bin/env bash
# Creates the RAR test fixtures. Requires the rar CLI from https://www.rarlab.com/download.htm (put rar on PATH).
set -euo pipefail

ROOT="$(cd "$(dirname "$0")/.." && pwd)"
OUT="$ROOT/src-tauri/tests/fixtures"
WORK="$(mktemp -d)"
trap 'rm -rf "$WORK"' EXIT

python3 - "$WORK" <<'PY'
import pathlib, struct, sys, zlib

def png(w, h, rgb):
    raw = b"".join(b"\x00" + bytes(rgb) * w for _ in range(h))
    def chunk(kind, data):
        crc = zlib.crc32(kind + data) & 0xFFFFFFFF
        return struct.pack(">I", len(data)) + kind + data + struct.pack(">I", crc)
    ihdr = struct.pack(">IIBBBBB", w, h, 8, 2, 0, 0, 0)
    return b"\x89PNG\r\n\x1a\n" + chunk(b"IHDR", ihdr) + chunk(b"IDAT", zlib.compress(raw)) + chunk(b"IEND", b"")

root = pathlib.Path(sys.argv[1]) / "book"
(root / "ch2").mkdir(parents=True)
(root / "ch10").mkdir()
(root / "ch2" / "page 10.png").write_bytes(png(30, 40, (200, 0, 0)))
(root / "ch2" / "page 2.png").write_bytes(png(20, 30, (0, 200, 0)))
(root / "ch10" / "page 1.png").write_bytes(png(10, 20, (0, 0, 200)))
(root / "notes.txt").write_text("not an image\n")
PY

mkdir -p "$OUT"
rm -f "$OUT"/*.rar "$OUT"/*.cbr
cd "$WORK/book"
rar a -r -s -idq "$OUT/book.rar" .
cp "$OUT/book.rar" "$OUT/book.cbr"
rar a -r -idq -pSecret "$OUT/encrypted-files.rar" .
rar a -r -idq -hpSecret "$OUT/encrypted-headers.rar" .
ls -l "$OUT"
