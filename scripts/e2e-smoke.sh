#!/usr/bin/env bash
set -euo pipefail

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$ROOT_DIR"

TMP_DIR="$(mktemp -d)"
trap 'rm -rf "$TMP_DIR"' EXIT

echo "[e2e] verify CLI help includes documented commands"
cargo run -q -p tdsl-cli -- --help >"$TMP_DIR/help.txt"
for cmd in build check ast fetch search inspect scaffold render init import-csv lint; do
  rg -q "\\b${cmd}\\b" "$TMP_DIR/help.txt"
done

echo "[e2e] Wikidata-origin flow (fixture + offline)"
cargo run -q -p tdsl-cli -- check examples/china_with_import.tdsl
cargo run -q -p tdsl-cli -- build examples/china_with_import.tdsl --offline --pretty --output "$TMP_DIR/wikidata_fixture.json"
cargo run -q -p tdsl-cli -- render examples/china_with_import.tdsl --offline --output "$TMP_DIR/wikidata_fixture.html"
test -s "$TMP_DIR/wikidata_fixture.json"
test -s "$TMP_DIR/wikidata_fixture.html"

echo "[e2e] Manual-origin flow (init -> import-csv -> lint --fix -> build/render)"
cargo run -q -p tdsl-cli -- init \
  --output "$TMP_DIR/manual.tdsl" \
  --timeline "架空世界年表" \
  --range-start 1000 \
  --range-end 1300 \
  --lanes "王国:kingdom,事件:incidents"
cargo run -q -p tdsl-cli -- import-csv examples/fictional_empire_items.csv --append "$TMP_DIR/manual.tdsl"
cargo run -q -p tdsl-cli -- lint "$TMP_DIR/manual.tdsl" --fix --format json >"$TMP_DIR/manual_lint.json"
rg -q '"ok": true' "$TMP_DIR/manual_lint.json"
cargo run -q -p tdsl-cli -- check "$TMP_DIR/manual.tdsl"
cargo run -q -p tdsl-cli -- build "$TMP_DIR/manual.tdsl" --pretty --output "$TMP_DIR/manual.json"
cargo run -q -p tdsl-cli -- render "$TMP_DIR/manual.tdsl" --output "$TMP_DIR/manual.html"
test -s "$TMP_DIR/manual.json"
test -s "$TMP_DIR/manual.html"

echo "[e2e] smoke completed"
