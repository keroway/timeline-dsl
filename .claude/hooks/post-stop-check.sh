#!/usr/bin/env bash
# Stop hook: Claude の応答完了時に format / lint / test を実行
#
# 動作:
#   1. 変更ファイル（uncommitted + 未 push の commit）に Rust ファイルがあれば
#      cargo fmt --check, cargo clippy, cargo test を実行
#   2. apps/webui 配下に変更があれば npm run lint を実行
#   3. 失敗時は stderr に内容を出力し exit 2 で Claude にフィードバック
#   4. cargo / npm が見つからないのに対象変更がある場合は FAIL として通知する
#      （silent-pass しない）
#
# 無限ループ防止:
#   stop_hook_active=true の場合（hook 由来の再起動）はスキップ
#
# スキップしたい場合:
#   TIMELINE_DSL_SKIP_STOP_HOOK=1 を設定する

set -u

# Claude から渡される JSON を読み取り、stop_hook_active を確認
INPUT="$(cat || true)"

if command -v jq >/dev/null 2>&1; then
  STOP_HOOK_ACTIVE="$(printf '%s' "$INPUT" | jq -r '.stop_hook_active // false' 2>/dev/null || echo false)"
else
  # jq 非依存フォールバック: 空白を除いた生 JSON を直接照合する
  # （jq が無い環境ではここで "false" 固定にすると下の無限ループ防止が丸ごと無効になる）。
  COMPACT_INPUT="$(printf '%s' "$INPUT" | tr -d ' \t\n\r')"
  case "$COMPACT_INPUT" in
    *'"stop_hook_active":true'*) STOP_HOOK_ACTIVE="true" ;;
    *) STOP_HOOK_ACTIVE="false" ;;
  esac
fi

if [ "$STOP_HOOK_ACTIVE" = "true" ]; then
  exit 0
fi

if [ "${TIMELINE_DSL_SKIP_STOP_HOOK:-}" = "1" ]; then
  exit 0
fi

# プロジェクトルートに移動
# silent-pass 禁止: cd / git 確認に失敗したら exit 2 で Claude に通知する。
# 「検証できない」ことを「成功」と扱わない。
PROJECT_DIR="${CLAUDE_PROJECT_DIR:-$(git rev-parse --show-toplevel 2>/dev/null || pwd)}"
if ! cd "$PROJECT_DIR" 2>/dev/null; then
  {
    echo "Stop hook: PROJECT_DIR ($PROJECT_DIR) に cd できません。検証をスキップしました。"
    echo "  CLAUDE_PROJECT_DIR=${CLAUDE_PROJECT_DIR:-(unset)}"
    echo "  PWD=$(pwd 2>/dev/null || echo unknown)"
    echo "hook の設定 (.claude/settings.json) と作業ディレクトリを確認してください。"
  } >&2
  exit 2
fi

# git で追跡しているリポジトリでなければ検証不能 → exit 2
if ! git rev-parse --git-dir >/dev/null 2>&1; then
  {
    echo "Stop hook: $(pwd) は git リポジトリではありません。変更ファイルを判定できないため検証をスキップしました。"
    echo "（一時的に止めたい場合は環境変数 TIMELINE_DSL_SKIP_STOP_HOOK=1）"
  } >&2
  exit 2
fi

# 変更ファイル一覧（unstaged + staged + untracked + 未 push の commit）
# 未 push commit を含めるのは、Claude がコミット済みの変更も検証対象に含めるため。
# 未 push 範囲は上流ブランチ → origin/main → 空、の順に degrade する
# （直近 N commit を見る fallback は、そのターンで触っていない main の変更まで
#   拾ってしまい、毎ターン全ステップが走る原因になるため使わない）。
UPSTREAM="$(git rev-parse --abbrev-ref --symbolic-full-name '@{u}' 2>/dev/null || true)"
if [ -n "$UPSTREAM" ]; then
  UNPUSHED_RANGE="${UPSTREAM}..HEAD"
elif git rev-parse --verify origin/main >/dev/null 2>&1; then
  UNPUSHED_RANGE="origin/main..HEAD"
else
  UNPUSHED_RANGE=""
fi

CHANGED_FILES="$(
  {
    git diff --name-only
    git diff --cached --name-only
    git ls-files --others --exclude-standard
    if [ -n "$UNPUSHED_RANGE" ]; then
      git log --name-only --pretty=format: "$UNPUSHED_RANGE" 2>/dev/null || true
    fi
  } | sed '/^$/d' | sort -u
)"

# 変更がなければスキップ
if [ -z "$CHANGED_FILES" ]; then
  exit 0
fi

RUST_CHANGED=0
WEBUI_CHANGED=0

# パス判定は [[ == pattern ]] を使う（case と異なり挙動が読みやすく、
# 複合条件もそのまま書ける）。`*` は `/` を跨いでマッチする。
while IFS= read -r file; do
  [ -z "$file" ] && continue

  # Rust 系: crates/ 配下の .rs / .pest / Cargo.toml、ワークスペースルートの Cargo.{toml,lock}
  if [[ "$file" == crates/*.rs ]] \
      || [[ "$file" == crates/*/src/*.rs ]] \
      || [[ "$file" == crates/*/tests/*.rs ]] \
      || [[ "$file" == crates/*/benches/*.rs ]] \
      || [[ "$file" == crates/*/examples/*.rs ]] \
      || [[ "$file" == crates/*/build.rs ]] \
      || [[ "$file" == crates/tdsl-parser/src/grammar.pest ]] \
      || [[ "$file" == crates/*/Cargo.toml ]] \
      || [[ "$file" == "Cargo.toml" ]] \
      || [[ "$file" == "Cargo.lock" ]]; then
    RUST_CHANGED=1
  fi

  # `*` が `/` を跨ぐ前提でネスト配下も拾う最終フォールバック
  if [[ "$file" == crates/*.rs ]] || [[ "$file" == *.rs && "$file" == crates/* ]]; then
    RUST_CHANGED=1
  fi

  # WebUI: apps/webui 配下の任意ファイル
  if [[ "$file" == apps/webui/* ]]; then
    WEBUI_CHANGED=1
  fi
done <<< "$CHANGED_FILES"

# どちらも該当しなければ終了
if [ "$RUST_CHANGED" -eq 0 ] && [ "$WEBUI_CHANGED" -eq 0 ]; then
  exit 0
fi

FAILED=0
REPORT=""

append_report() {
  REPORT="${REPORT}$1"$'\n'
}

# 実行: 作業ディレクトリを明示的に受け取り、サブシェルで FAILED を喪失しない。
# 関数内で FAILED と REPORT を直接書き換えるためサブシェルは使わない。
run_step() {
  local workdir="$1"; shift
  local label="$1"; shift
  echo "→ [stop-hook] $label (in $workdir)" >&2

  local output
  local rc=0
  # `pushd/popd` で chdir した上でコマンドを起動 → 子プロセスのみ chdir 影響を受ける。
  # ここで FAILED を書き換えるため、関数自体はサブシェルを作らない。
  pushd "$workdir" >/dev/null || {
    FAILED=1
    append_report ""
    append_report "❌ $label: 作業ディレクトリ $workdir に cd できませんでした。"
    return
  }
  output="$("$@" 2>&1)" || rc=$?
  popd >/dev/null || true

  if [ "$rc" -ne 0 ]; then
    FAILED=1
    append_report ""
    append_report "❌ $label が失敗しました (rc=$rc)"
    append_report "コマンド: $*"
    append_report "$output"
  fi
}

if [ "$RUST_CHANGED" -eq 1 ]; then
  if command -v cargo >/dev/null 2>&1; then
    run_step "." "cargo fmt --check" cargo fmt --all -- --check
    run_step "." "cargo clippy" cargo clippy --workspace --all-targets -- -D warnings
    run_step "." "cargo test" cargo test --workspace --quiet
  else
    # silent-pass 禁止: 検証できなかったことを失敗として通知する
    FAILED=1
    append_report ""
    append_report "❌ cargo が見つかりません。Rust ファイルが変更されているのに検証できませんでした。"
    append_report "  PATH=$PATH"
  fi
fi

if [ "$WEBUI_CHANGED" -eq 1 ]; then
  if command -v npm >/dev/null 2>&1; then
    # worktree では node_modules が存在しないため、main repo の apps/webui を優先する。
    # git rev-parse --git-common-dir は main repo の .git を返す（worktree では別パス）。
    WEBUI_DIR="$PROJECT_DIR/apps/webui"
    if [ ! -d "$WEBUI_DIR/node_modules" ]; then
      GIT_COMMON_DIR="$(git rev-parse --git-common-dir 2>/dev/null || true)"
      if [ -n "$GIT_COMMON_DIR" ]; then
        MAIN_ROOT="$(cd "${GIT_COMMON_DIR}/.." 2>/dev/null && pwd || true)"
        if [ -d "${MAIN_ROOT}/apps/webui/node_modules" ]; then
          WEBUI_DIR="${MAIN_ROOT}/apps/webui"
        fi
      fi
    fi
    if [ -d "$WEBUI_DIR" ]; then
      run_step "$WEBUI_DIR" "npm run lint (apps/webui)" npm run --silent lint
    else
      FAILED=1
      append_report ""
      append_report "❌ apps/webui ディレクトリが見つかりません。WebUI が変更されているのに検証できませんでした。"
    fi
  else
    FAILED=1
    append_report ""
    append_report "❌ npm が見つかりません。WebUI が変更されているのに検証できませんでした。"
  fi
fi

if [ "$FAILED" -eq 1 ]; then
  {
    echo "Stop hook: format / lint / test に失敗があります。下記を修正してから完了してください。"
    echo "（再実行をスキップしたい場合は環境変数 TIMELINE_DSL_SKIP_STOP_HOOK=1）"
    echo "$REPORT"
  } >&2
  exit 2
fi

exit 0
