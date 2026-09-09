#!/usr/bin/env bash
set -euo pipefail

# scripts/generate-homebrew-formula.sh が生成する Formula/tdsl.rb を検証する（#889）。
#
# 2つのモードを持つ:
#   通常呼び出し（引数なし）: ダミー値でスクリプトを実行し生成物を検証する。
#     ci.yml から毎PR実行される回帰テスト。
#   --check-only <path>: 既存ファイルをそのまま検証する。
#     release.yml が実データで生成した直後の健全性チェックに使う。
#
# 検証内容:
#   - `ruby -c` で構文が妥当か
#   - `version` 行を宣言していないか（宣言すると `brew audit` が
#     "version X is redundant with version scanned from URL" で落ちる。v2.1.0 で発生）
#   - url / sha256 の宣言が4プラットフォーム分ちょうど揃っているか

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"

check_formula() {
  local formula_path="$1"

  if ! command -v ruby >/dev/null 2>&1; then
    echo "::error::ruby is not available; cannot validate Formula/tdsl.rb syntax." >&2
    exit 1
  fi

  echo "[test-homebrew-formula] ruby -c syntax check"
  ruby -c "$formula_path"

  echo "[test-homebrew-formula] checking for redundant 'version' declaration"
  if grep -nE '^\s*version\s+"' "$formula_path"; then
    echo "::error::Formula declares an explicit 'version' line; Homebrew derives version from url and 'brew audit' rejects a redundant declaration (regressed in v2.1.0)." >&2
    exit 1
  fi

  echo "[test-homebrew-formula] checking url/sha256 pair count (expect 4 each)"
  local url_count sha_count
  url_count=$(grep -cE '^\s*url "' "$formula_path")
  sha_count=$(grep -cE '^\s*sha256 "' "$formula_path")
  if [ "$url_count" -ne 4 ] || [ "$sha_count" -ne 4 ]; then
    echo "::error::Expected 4 url and 4 sha256 declarations (linux x86_64/aarch64, macos x86_64/aarch64); found url=$url_count sha256=$sha_count." >&2
    exit 1
  fi

  echo "[test-homebrew-formula] OK: $formula_path"
}

if [ "${1:-}" = "--check-only" ]; then
  formula_path="${2:?--check-only requires a path to an existing Formula file}"
  check_formula "$formula_path"
  exit 0
fi

TMP_DIR="$(mktemp -d)"
trap 'rm -rf "$TMP_DIR"' EXIT
FORMULA_PATH="$TMP_DIR/tdsl.rb"

VERSION="v9.9.9" \
SHA_LINUX="0000000000000000000000000000000000000000000000000000000000aa" \
SHA_LINUX_ARM="0000000000000000000000000000000000000000000000000000000000bb" \
SHA_MACOS_X86="0000000000000000000000000000000000000000000000000000000000cc" \
SHA_MACOS_ARM="0000000000000000000000000000000000000000000000000000000000dd" \
OUTPUT_PATH="$FORMULA_PATH" \
  "$ROOT_DIR/scripts/generate-homebrew-formula.sh"

check_formula "$FORMULA_PATH"
