#!/usr/bin/env bash
# ドキュメントが案内する `keroway/timeline-dsl@<ref>` が、workspace の版と同じ
# 完全な版番号（vX.Y.Z）であることを検査する（オフライン、#1067）。
# メジャー版だけの移動タグ（v3 等）は運用しないため、それ以外の ref は失敗にする。
set -euo pipefail

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
FILES=(README.md README.ja.md docs/ci-integration.md .claude/rules/feature-status.md)

VERSION="$(awk '
  /^\[workspace\.package\]/ { in_section = 1; next }
  /^\[/ { in_section = 0 }
  in_section && /^version[[:space:]]*=/ { gsub(/[^0-9.]/, "", $0); print; exit }
' "$ROOT/Cargo.toml")"
if [ -z "$VERSION" ]; then
  echo "FAIL: Cargo.toml から [workspace.package].version を読めない" >&2
  exit 1
fi
EXPECTED="keroway/timeline-dsl@v$VERSION"

status=0
total=0
for file in "${FILES[@]}"; do
  path="$ROOT/$file"
  if [ ! -f "$path" ]; then
    echo "FAIL: $file が無い" >&2
    status=1
    continue
  fi
  while IFS= read -r line; do
    total=$((total + 1))
    ref="${line#*:}"
    if [ "$ref" != "$EXPECTED" ]; then
      echo "FAIL: $file:${line%%:*}: $ref（期待値: $EXPECTED）" >&2
      status=1
    fi
  done < <(grep -noE 'keroway/timeline-dsl@[A-Za-z0-9._/-]+' "$path" || true)
done

if [ "$total" -eq 0 ]; then
  echo "FAIL: 検査対象の参照が1件も見つからない" >&2
  exit 1
fi
if [ "$status" -eq 0 ]; then
  echo "OK: $total 件の参照がすべて $EXPECTED"
fi
exit "$status"
