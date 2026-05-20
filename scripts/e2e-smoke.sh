#!/usr/bin/env bash
set -euo pipefail

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$ROOT_DIR"

TMP_DIR="$(mktemp -d)"
trap 'rm -rf "$TMP_DIR"' EXIT

# helper: assert that a command exits with non-zero status
assert_fails() {
  if "$@" 2>/dev/null; then
    echo "FAIL: expected failure but succeeded: $*"
    exit 1
  fi
}

echo "[e2e] verify CLI help includes all 12 documented commands"
cargo run -q -p tdsl-cli -- --help >"$TMP_DIR/help.txt"
for cmd in build check ast fetch search inspect resolve scaffold render init import-csv lint; do
  grep -Eq "[[:space:]]${cmd}[[:space:]]" "$TMP_DIR/help.txt"
done

# ---- tdsl ast ---------------------------------------------------------------
echo "[e2e] ast: dump parsed AST for static file"
cargo run -q -p tdsl-cli -- ast examples/china_dynasties.tdsl >"$TMP_DIR/ast.txt"
grep -Fq "Timeline" "$TMP_DIR/ast.txt"

# ---- tdsl build (static) ----------------------------------------------------
echo "[e2e] build: static file with --pretty"
cargo run -q -p tdsl-cli -- build examples/china_dynasties.tdsl --pretty --output "$TMP_DIR/static.json"
test -s "$TMP_DIR/static.json"
grep -Fq '"title"' "$TMP_DIR/static.json"

# ---- tdsl check (normal) ----------------------------------------------------
echo "[e2e] check: valid static file"
cargo run -q -p tdsl-cli -- check examples/china_dynasties.tdsl

# ---- tdsl check (abnormal) --------------------------------------------------
echo "[e2e] check: syntax error file exits non-zero"
assert_fails cargo run -q -p tdsl-cli -- check tests/fixtures/invalid_syntax.tdsl

echo "[e2e] check: semantic error file exits non-zero"
assert_fails cargo run -q -p tdsl-cli -- check tests/fixtures/invalid_semantics.tdsl

# ---- tdsl build (abnormal) --------------------------------------------------
echo "[e2e] build: missing file exits non-zero"
assert_fails cargo run -q -p tdsl-cli -- build tests/fixtures/nonexistent.tdsl

echo "[e2e] build: syntax error file exits non-zero"
assert_fails cargo run -q -p tdsl-cli -- build tests/fixtures/invalid_syntax.tdsl

# ---- tdsl build / render (offline, Wikidata import) -------------------------
echo "[e2e] build+render: Wikidata-origin flow (offline)"
cargo run -q -p tdsl-cli -- check examples/china_with_import.tdsl
cargo run -q -p tdsl-cli -- build examples/china_with_import.tdsl --offline --pretty --output "$TMP_DIR/wikidata_fixture.json"
test -s "$TMP_DIR/wikidata_fixture.json"
cargo run -q -p tdsl-cli -- render examples/china_with_import.tdsl --offline --output "$TMP_DIR/wikidata_fixture.html"
test -s "$TMP_DIR/wikidata_fixture.html"

# ---- tdsl render (theme options) --------------------------------------------
echo "[e2e] render: --theme dark outputs HTML"
cargo run -q -p tdsl-cli -- render examples/china_dynasties.tdsl --theme dark --output "$TMP_DIR/dark.html"
test -s "$TMP_DIR/dark.html"
grep -Fq "background" "$TMP_DIR/dark.html"

echo "[e2e] render: --theme pastel outputs HTML"
cargo run -q -p tdsl-cli -- render examples/china_dynasties.tdsl --theme pastel --output "$TMP_DIR/pastel.html"
test -s "$TMP_DIR/pastel.html"

echo "[e2e] render: stdout output contains SVG"
cargo run -q -p tdsl-cli -- render examples/china_dynasties.tdsl >"$TMP_DIR/render_stdout.html"
grep -Fq "<svg" "$TMP_DIR/render_stdout.html"

# ---- tdsl render --format png (resvg rasterization) -------------------------
echo "[e2e] render: --format png outputs a valid PNG file"
cargo run -q -p tdsl-cli -- render examples/china_dynasties.tdsl --format png --output "$TMP_DIR/china.png"
test -s "$TMP_DIR/china.png"
# Verify PNG file signature: 89 50 4E 47 0D 0A 1A 0A
head -c 8 "$TMP_DIR/china.png" | od -A n -t x1 | tr -d ' \n' | grep -Eq "^89504e470d0a1a0a$" \
  || { echo "FAIL: PNG signature mismatch in $TMP_DIR/china.png"; exit 1; }

echo "[e2e] render: --format png --dpi 300 produces larger PNG than default 96 DPI"
cargo run -q -p tdsl-cli -- render examples/china_dynasties.tdsl --format png --dpi 300 --output "$TMP_DIR/china_300dpi.png"
test -s "$TMP_DIR/china_300dpi.png"
head -c 8 "$TMP_DIR/china_300dpi.png" | od -A n -t x1 | tr -d ' \n' | grep -Eq "^89504e470d0a1a0a$" \
  || { echo "FAIL: PNG signature mismatch in $TMP_DIR/china_300dpi.png"; exit 1; }
SIZE_96=$(wc -c < "$TMP_DIR/china.png")
SIZE_300=$(wc -c < "$TMP_DIR/china_300dpi.png")
[ "$SIZE_300" -gt "$SIZE_96" ] \
  || { echo "FAIL: 300 DPI PNG ($SIZE_300 bytes) should be larger than 96 DPI PNG ($SIZE_96 bytes)"; exit 1; }

echo "[e2e] render: --format png --png-scale 2.0 produces larger PNG than default"
cargo run -q -p tdsl-cli -- render examples/china_dynasties.tdsl --format png --png-scale 2.0 --output "$TMP_DIR/china_2x.png"
test -s "$TMP_DIR/china_2x.png"
head -c 8 "$TMP_DIR/china_2x.png" | od -A n -t x1 | tr -d ' \n' | grep -Eq "^89504e470d0a1a0a$" \
  || { echo "FAIL: PNG signature mismatch in $TMP_DIR/china_2x.png"; exit 1; }
SIZE_2X=$(wc -c < "$TMP_DIR/china_2x.png")
[ "$SIZE_2X" -gt "$SIZE_96" ] \
  || { echo "FAIL: 2x scale PNG ($SIZE_2X bytes) should be larger than default PNG ($SIZE_96 bytes)"; exit 1; }

# ---- tdsl init -> import-csv -> lint -> build -> render (full manual flow) --
echo "[e2e] manual flow: init -> import-csv -> lint --fix -> check -> build -> render"
cargo run -q -p tdsl-cli -- init \
  --output "$TMP_DIR/manual.tdsl" \
  --timeline "架空世界年表" \
  --range-start 1000 \
  --range-end 1300 \
  --lanes "王国:kingdom,事件:incidents"
test -s "$TMP_DIR/manual.tdsl"

cargo run -q -p tdsl-cli -- import-csv examples/fictional_empire_items.csv --append "$TMP_DIR/manual.tdsl"

cargo run -q -p tdsl-cli -- lint "$TMP_DIR/manual.tdsl" --fix --format json >"$TMP_DIR/manual_lint.json"
grep -Fq '"ok": true' "$TMP_DIR/manual_lint.json"

cargo run -q -p tdsl-cli -- check "$TMP_DIR/manual.tdsl"
cargo run -q -p tdsl-cli -- build "$TMP_DIR/manual.tdsl" --pretty --output "$TMP_DIR/manual.json"
test -s "$TMP_DIR/manual.json"
cargo run -q -p tdsl-cli -- render "$TMP_DIR/manual.tdsl" --output "$TMP_DIR/manual.html"
test -s "$TMP_DIR/manual.html"

# ---- tdsl lint (no --fix) ---------------------------------------------------
echo "[e2e] lint: plain lint on examples"
cargo run -q -p tdsl-cli -- lint examples/china_dynasties.tdsl --format json >"$TMP_DIR/lint.json"
grep -Fq '"ok"' "$TMP_DIR/lint.json"

# ---- tdsl import-csv (output to new file) -----------------------------------
echo "[e2e] import-csv: write snippet to stdout"
cargo run -q -p tdsl-cli -- import-csv examples/fictional_empire_items.csv >"$TMP_DIR/csv_snippet.tdsl"
test -s "$TMP_DIR/csv_snippet.tdsl"
grep -Eq "span|event" "$TMP_DIR/csv_snippet.tdsl"

echo "[e2e] import-csv: write snippet to output file"
cargo run -q -p tdsl-cli -- import-csv examples/fictional_empire_items.csv --output "$TMP_DIR/csv_out.tdsl"
test -s "$TMP_DIR/csv_out.tdsl"

# ---- online-required commands: verify --help exits cleanly ------------------
echo "[e2e] fetch/search/inspect/resolve/scaffold: --help exits 0"
cargo run -q -p tdsl-cli -- fetch --help >/dev/null
cargo run -q -p tdsl-cli -- search --help >/dev/null
cargo run -q -p tdsl-cli -- inspect --help >/dev/null
cargo run -q -p tdsl-cli -- resolve --help >/dev/null
cargo run -q -p tdsl-cli -- scaffold --help >/dev/null

echo "[e2e] smoke completed"
