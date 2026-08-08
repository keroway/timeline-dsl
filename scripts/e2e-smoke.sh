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

echo "[e2e] verify CLI help includes all 14 documented commands"
cargo run -q -p tdsl-cli -- --help >"$TMP_DIR/help.txt"
for cmd in build check ast fetch search inspect resolve scaffold render init import-csv export-csv lint fmt; do
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

# ---- tdsl render --format pdf (svg2pdf vector PDF) ---------------------------
echo "[e2e] render: --format pdf outputs a valid vector PDF file"
cargo run -q -p tdsl-cli -- render examples/china_dynasties.tdsl --format pdf --output "$TMP_DIR/china.pdf"
test -s "$TMP_DIR/china.pdf"
# Verify PDF file signature: %PDF-  (25 50 44 46 2D)
head -c 5 "$TMP_DIR/china.pdf" | grep -Fq '%PDF-' \
  || { echo "FAIL: PDF signature %%PDF- not found in $TMP_DIR/china.pdf"; exit 1; }

echo "[e2e] render: --format pdf --pdf-size a3 --pdf-landscape --pdf-margin 15 outputs a valid PDF"
cargo run -q -p tdsl-cli -- render examples/china_dynasties.tdsl --format pdf --pdf-size a3 --pdf-landscape --pdf-margin 15 --output "$TMP_DIR/china_a3_landscape.pdf"
test -s "$TMP_DIR/china_a3_landscape.pdf"
head -c 5 "$TMP_DIR/china_a3_landscape.pdf" | grep -Fq '%PDF-' \
  || { echo "FAIL: PDF signature %%PDF- not found in $TMP_DIR/china_a3_landscape.pdf"; exit 1; }

# ---- tdsl render --watch (NOT smoke-tested) ----------------------------------
# NOTE: `tdsl render --watch` はファイル監視の常駐プロセスで自発的に終了しないため、
# 終了コードベースの本スモークテストでは対象外とする（タイムアウト頼みの検証は
# flaky になる）。挙動はユニットテスト / 手動確認でカバーする。

# ---- tdsl render --grid (auxiliary grid lines) -------------------------------
echo "[e2e] render: --grid decade outputs SVG with grid lines"
cargo run -q -p tdsl-cli -- render examples/world_wars.tdsl --format svg --grid decade --output "$TMP_DIR/grid_decade.svg"
test -s "$TMP_DIR/grid_decade.svg"
grep -Fq "tdsl-grid-line" "$TMP_DIR/grid_decade.svg"

echo "[e2e] render: --grid year outputs more grid lines than --grid decade"
cargo run -q -p tdsl-cli -- render examples/world_wars.tdsl --format svg --grid year --output "$TMP_DIR/grid_year.svg"
test -s "$TMP_DIR/grid_year.svg"
GRID_DECADE=$(grep -o "tdsl-grid-line" "$TMP_DIR/grid_decade.svg" | wc -l)
GRID_YEAR=$(grep -o "tdsl-grid-line" "$TMP_DIR/grid_year.svg" | wc -l)
[ "$GRID_YEAR" -gt "$GRID_DECADE" ] \
  || { echo "FAIL: year grid ($GRID_YEAR lines) should have more lines than decade grid ($GRID_DECADE lines)"; exit 1; }

echo "[e2e] render: --grid month outputs more grid lines than --grid year"
cargo run -q -p tdsl-cli -- render examples/world_wars.tdsl --format svg --grid month --output "$TMP_DIR/grid_month.svg"
test -s "$TMP_DIR/grid_month.svg"
GRID_MONTH=$(grep -o "tdsl-grid-line" "$TMP_DIR/grid_month.svg" | wc -l)
[ "$GRID_MONTH" -gt "$GRID_YEAR" ] \
  || { echo "FAIL: month grid ($GRID_MONTH lines) should have more lines than year grid ($GRID_YEAR lines)"; exit 1; }

# ---- tdsl render --orientation vertical --------------------------------------
echo "[e2e] render: --orientation vertical outputs SVG taller than wide"
cargo run -q -p tdsl-cli -- render examples/china_dynasties.tdsl --format svg --orientation vertical --output "$TMP_DIR/vertical.svg"
test -s "$TMP_DIR/vertical.svg"
grep -Fq "<svg" "$TMP_DIR/vertical.svg"
V_WIDTH=$(sed -nE 's/.*<svg[^>]* width="([0-9]+)".*/\1/p' "$TMP_DIR/vertical.svg" | head -n 1)
V_HEIGHT=$(sed -nE 's/.*<svg[^>]*height="([0-9]+)".*/\1/p' "$TMP_DIR/vertical.svg" | head -n 1)
[ "$V_HEIGHT" -gt "$V_WIDTH" ] \
  || { echo "FAIL: vertical SVG should be taller (h=$V_HEIGHT) than wide (w=$V_WIDTH)"; exit 1; }

# ---- tdsl render --show-table -------------------------------------------------
echo "[e2e] render: --show-table appends an item table to HTML"
cargo run -q -p tdsl-cli -- render examples/china_dynasties.tdsl --show-table --output "$TMP_DIR/with_table.html"
test -s "$TMP_DIR/with_table.html"
grep -Fq '<div class="tdsl-table-wrap">' "$TMP_DIR/with_table.html"
grep -Fq "<table" "$TMP_DIR/with_table.html"

echo "[e2e] render: without --show-table HTML has no item table"
cargo run -q -p tdsl-cli -- render examples/china_dynasties.tdsl --output "$TMP_DIR/no_table.html"
test -s "$TMP_DIR/no_table.html"
! grep -Fq "<table" "$TMP_DIR/no_table.html" \
  || { echo "FAIL: HTML without --show-table must not contain an item table"; exit 1; }

# ---- tdsl render --chart-pagination (lane group chart pagination, #660) ----
echo "[e2e] render: --chart-pagination 2 splits a 4-lane chart into 2 SVG page files"
cargo run -q -p tdsl-cli -- render examples/sci_tech_timeline.tdsl --format svg --chart-pagination 2 --output "$TMP_DIR/sci_tech.svg"
test -s "$TMP_DIR/sci_tech.page1.svg"
test -s "$TMP_DIR/sci_tech.page2.svg"
grep -Fq "<svg" "$TMP_DIR/sci_tech.page1.svg"
grep -Fq "<svg" "$TMP_DIR/sci_tech.page2.svg"

# ---- tdsl render --chart-pagination-range (time-range chart pagination, #733) ----
echo "[e2e] render: --chart-pagination-range 3 splits a chart into 3 SVG page files by time segment"
cargo run -q -p tdsl-cli -- render examples/china_dynasties.tdsl --format svg --chart-pagination-range 3 --output "$TMP_DIR/china_range.svg"
test -s "$TMP_DIR/china_range.page1.svg"
test -s "$TMP_DIR/china_range.page2.svg"
test -s "$TMP_DIR/china_range.page3.svg"
for f in china_range.page1.svg china_range.page2.svg china_range.page3.svg; do
  grep -Fq "<svg" "$TMP_DIR/$f"
  grep -Fq "</svg>" "$TMP_DIR/$f"
done
! test -e "$TMP_DIR/china_range.page4.svg" \
  || { echo "FAIL: --chart-pagination-range 3 must not produce a 4th page file"; exit 1; }

echo "[e2e] render: --chart-pagination-range 0 exits non-zero"
! cargo run -q -p tdsl-cli -- render examples/china_dynasties.tdsl --format svg --chart-pagination-range 0 --output "$TMP_DIR/china_range_zero.svg" 2>/dev/null \
  || { echo "FAIL: --chart-pagination-range 0 must exit non-zero"; exit 1; }

echo "[e2e] render: --chart-pagination and --chart-pagination-range together exits non-zero"
! cargo run -q -p tdsl-cli -- render examples/china_dynasties.tdsl --format svg --chart-pagination 2 --chart-pagination-range 2 --output "$TMP_DIR/china_both.svg" 2>/dev/null \
  || { echo "FAIL: combining --chart-pagination and --chart-pagination-range must exit non-zero"; exit 1; }

# ---- tdsl render --chart-pagination-range --format pdf (PDF integration, #736) ----
echo "[e2e] render: --chart-pagination-range 3 --format pdf writes a single multi-page PDF"
cargo run -q -p tdsl-cli -- render examples/china_dynasties.tdsl --format pdf --chart-pagination-range 3 --output "$TMP_DIR/china_range.pdf"
test -s "$TMP_DIR/china_range.pdf"
head -c 5 "$TMP_DIR/china_range.pdf" | grep -Fq "%PDF-"

echo "[e2e] render: --chart-pagination-range --show-table --pdf-pagination combined PDF renders"
cargo run -q -p tdsl-cli -- render examples/china_dynasties.tdsl --format pdf --chart-pagination-range 3 --show-table --pdf-pagination --output "$TMP_DIR/china_range_full.pdf"
test -s "$TMP_DIR/china_range_full.pdf"
head -c 5 "$TMP_DIR/china_range_full.pdf" | grep -Fq "%PDF-"

echo "[e2e] render: --chart-pagination with --chart-pagination-range (--format pdf) exits non-zero"
! cargo run -q -p tdsl-cli -- render examples/china_dynasties.tdsl --format pdf --chart-pagination 2 --chart-pagination-range 3 --output "$TMP_DIR/china_both.pdf" 2>/dev/null \
  || { echo "FAIL: combining both chart-pagination axes with --format pdf must exit non-zero"; exit 1; }

echo "[e2e] render: --format pdf without --chart-pagination-range is unchanged (single page)"
cargo run -q -p tdsl-cli -- render examples/china_dynasties.tdsl --format pdf --output "$TMP_DIR/china_plain.pdf"
test -s "$TMP_DIR/china_plain.pdf"

# ---- tdsl build --json-schema -------------------------------------------------
echo "[e2e] build: --json-schema outputs TimelineIr JSON Schema without input file"
cargo run -q -p tdsl-cli -- build --json-schema >"$TMP_DIR/schema.json"
test -s "$TMP_DIR/schema.json"
# shellcheck disable=SC2016 # JSON Schema の "$schema" キーをリテラル検索（展開は不要）
grep -Fq '"$schema"' "$TMP_DIR/schema.json"
grep -Fq '"TimelineIr"' "$TMP_DIR/schema.json"

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

# ---- tdsl fmt ---------------------------------------------------------------
echo "[e2e] fmt: format static example to stdout"
cargo run -q -p tdsl-cli -- fmt examples/china_dynasties.tdsl >"$TMP_DIR/fmt_out.tdsl"
test -s "$TMP_DIR/fmt_out.tdsl"
grep -Eq "timeline|lane|span" "$TMP_DIR/fmt_out.tdsl"

echo "[e2e] fmt --check: already-formatted file exits 0"
# フォーマット済みファイルに対して --check は成功するはず
cargo run -q -p tdsl-cli -- fmt "$TMP_DIR/fmt_out.tdsl" --check

echo "[e2e] fmt --check: unformatted file exits non-zero"
# インデントを崩した入力を作成
cat > "$TMP_DIR/unformatted.tdsl" <<'TDSL'
timeline "T" {title "T";unit year;range 1900..2000;}
lane "A" as a {kind custom;order 1;}
TDSL
assert_fails cargo run -q -p tdsl-cli -- fmt "$TMP_DIR/unformatted.tdsl" --check

echo "[e2e] fmt --write: rewrites file in-place"
cp "$TMP_DIR/unformatted.tdsl" "$TMP_DIR/to_write.tdsl"
cargo run -q -p tdsl-cli -- fmt "$TMP_DIR/to_write.tdsl" --write
cargo run -q -p tdsl-cli -- fmt "$TMP_DIR/to_write.tdsl" --check

echo "[e2e] fmt: idempotent (fmt output re-formatted equals itself)"
cargo run -q -p tdsl-cli -- fmt "$TMP_DIR/fmt_out.tdsl" >"$TMP_DIR/fmt_out2.tdsl"
diff "$TMP_DIR/fmt_out.tdsl" "$TMP_DIR/fmt_out2.tdsl"

echo "[e2e] fmt: parse error exits non-zero"
assert_fails cargo run -q -p tdsl-cli -- fmt tests/fixtures/invalid_syntax.tdsl

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

# ---- tdsl export-csv (IR -> CSV, symmetric with import-csv) ------------------
echo "[e2e] export-csv: .tdsl source to stdout with header"
cargo run -q -p tdsl-cli -- export-csv examples/fictional_empire.tdsl --offline >"$TMP_DIR/export.csv"
test -s "$TMP_DIR/export.csv"
head -n1 "$TMP_DIR/export.csv" | grep -Fxq "lane,type,start,end,time,label,tags,id,source,origin"

echo "[e2e] export-csv: accepts .json IR input (build -> export)"
cargo run -q -p tdsl-cli -- build examples/fictional_empire.tdsl --offline --output "$TMP_DIR/export_ir.json"
cargo run -q -p tdsl-cli -- export-csv "$TMP_DIR/export_ir.json" >"$TMP_DIR/export_from_json.csv"
diff "$TMP_DIR/export.csv" "$TMP_DIR/export_from_json.csv"

echo "[e2e] export-csv -> import-csv round-trip preserves items"
cargo run -q -p tdsl-cli -- import-csv "$TMP_DIR/export.csv" >"$TMP_DIR/export_roundtrip.tdsl"
grep -Fq "span kingdom 1001..1180" "$TMP_DIR/export_roundtrip.tdsl"
grep -Fq "event_range incidents 1175..1180" "$TMP_DIR/export_roundtrip.tdsl"

# ---- online-required commands: verify --help exits cleanly ------------------
echo "[e2e] fetch/search/inspect/resolve/scaffold: --help exits 0"
cargo run -q -p tdsl-cli -- fetch --help >/dev/null
cargo run -q -p tdsl-cli -- search --help >/dev/null
cargo run -q -p tdsl-cli -- inspect --help >/dev/null
cargo run -q -p tdsl-cli -- resolve --help >/dev/null
cargo run -q -p tdsl-cli -- scaffold --help >/dev/null

echo "[e2e] smoke completed"
