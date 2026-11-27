#!/usr/bin/env bash
# action.yml の入力がシェル構文として実行されないことを検証する（オフライン、#1033）。
# 1. run 本文に ${{ inputs.* }} の直接展開が無いこと
# 2. Render ステップを偽 tdsl で実行し、特殊文字入力が引数として保持されること
set -euo pipefail

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
ACTION="$ROOT/action.yml"

if ! command -v yq >/dev/null 2>&1; then
  echo "yq is required" >&2
  exit 1
fi

# 1. 静的検査
# shellcheck disable=SC2016  # ${{ はリテラルとして検索する
if yq '.runs.steps[].run' "$ACTION" | grep -F '${{' >/dev/null; then
  echo "FAIL: run 本文に \${{ }} の直接展開が残っている" >&2
  exit 1
fi

# 2. 動的検査
WORK="$(mktemp -d)"
trap 'rm -rf "$WORK"' EXIT
mkdir -p "$WORK/bin"

cat >"$WORK/bin/tdsl" <<'EOF'
#!/usr/bin/env bash
for a in "$@"; do printf '%s\n' "$a"; done > "$TDSL_ARGS_OUT"
: > "$TDSL_OUT_FILE"
EOF
chmod +x "$WORK/bin/tdsl"

yq '.runs.steps[] | select(.id == "render") | .run' "$ACTION" >"$WORK/render.sh"

PROBE_FILE="$WORK/probe-ran"
PAYLOAD="x\$(touch $PROBE_FILE)\`touch $PROBE_FILE\`\"; touch $PROBE_FILE; \"y"

run_render() {
  : >"$WORK/args"
  ( cd "$WORK" \
    && PATH="$WORK/bin:$PATH" \
       TDSL_ARGS_OUT="$WORK/args" TDSL_OUT_FILE="$WORK/out.svg" \
       GITHUB_OUTPUT="$WORK/gh-output" \
       INPUT_FILE="$1" INPUT_FORMAT="svg" INPUT_OUTPUT="$WORK/out.svg" \
       INPUT_OFFLINE="false" INPUT_INTERACTIVE="false" \
       INPUT_THEME="$2" INPUT_SCALE="" \
       bash "$WORK/render.sh" >/dev/null )
}

run_render "$PAYLOAD.tdsl" "$PAYLOAD"

if [ -e "$PROBE_FILE" ]; then
  echo "FAIL: 入力内のシェル構文が実行された" >&2
  exit 1
fi
if ! grep -qxF "$PAYLOAD.tdsl" "$WORK/args"; then
  echo "FAIL: file 入力が引数として保持されていない" >&2
  exit 1
fi
if ! grep -qxF "$PAYLOAD" "$WORK/args"; then
  echo "FAIL: theme 入力が引数として保持されていない" >&2
  exit 1
fi

echo "OK: action.yml inputs are not shell-evaluated"
