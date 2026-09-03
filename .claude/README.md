# timeline-dsl — Claude Code Setup

このディレクトリは Claude Code / Codex / pi 等の AI コーディングエージェントの動作をこのプロジェクト用に
整える共有設定です。`CLAUDE.md`（リポジトリルート。`AGENTS.md` は同ファイルへの symlink）と一緒に読んでください。

## 構成

```
.claude/
├── agents/
│   ├── rust-app-developer.md  # 文法・lowering・Wikidata 連携の実装担当
│   └── app-dev-director.md    # 設計判断・スコープ整理・仕様整合性レビュー担当
├── commands/
│   └── fix-pr.md              # /fix-pr スラッシュコマンド
├── rules/
│   └── implementation-strict.md # 実装の厳密化ルール（NO-GO パターン等）
├── settings.json              # 共有設定（hook 登録・許可プラグイン等、コミット対象）
├── settings.local.json        # 個人設定（.gitignore で除外）
└── agent-memory/              # エージェントの観察ログ（.gitignore で除外）
```

## 依存ツール

| ツール | 用途 | 必須？ |
|---|---|---|
| `cargo` | ワークスペース全体の検証（fmt/clippy/test） | 変更がある場合のみ。無い環境では FAILED として通知 |
| `npm` | `apps/webui` の検証 | webui 変更がある場合のみ |
| `jq` | hook 内 JSON 抽出 | 無い環境では非依存フォールバックで動作 |

## Hooks の挙動

### Stop: `post-stop-check.sh`

- 発火条件: Claude が応答を終えたとき（変更ファイルが無ければ即終了）
- 動作: 変更ファイル（uncommitted + untracked + 未 push commit、未 push 範囲は `@{u}` →
  `origin/main..HEAD` → 空 の3段 degrade）を分類し、`cargo fmt --check` / `cargo clippy -D warnings` /
  `cargo test --workspace` を実行（WebUI 変更時は `npm run lint` も）
- 失敗時: exit 2 で Claude にフィードバック（ブロッキング）
- 一時的に止めたい場合: `TIMELINE_DSL_SKIP_STOP_HOOK=1`

## Slash Commands

### `/fix-pr [PR番号]`

自分の PR の CI 失敗を自動修正する。

## Agents の役割分担

| Agent | 担当領域 |
|---|---|
| `rust-app-developer` | 文法・lowering・Wikidata 連携の実装 |
| `app-dev-director` | 設計判断・スコープ整理・仕様整合性レビュー |

## Rules の参照階層

`implementation-strict.md` を `CLAUDE.md`（`AGENTS.md` は symlink）より優先する
（timeline-dsl はこの向きが正しい。reflectorbit とは逆だが、それぞれ明示されているため統一しない）。`implementation-strict.md` 自体は `.claude/` 配下の補助ルールであり、リポジトリルートの `CLAUDE.md` より優先して適用する。

## AGENTS.md との関係

`AGENTS.md`（リポジトリルート）は `CLAUDE.md` への symlink（#705）。正典は `CLAUDE.md` 側のみで、
Codex / pi など非 Claude エージェント向けにも同一内容が `AGENTS.md` 経由で届く。二重編集は不要
（symlink なので編集は自動的に両方に反映される）。

## 他環境への移植

このディレクトリは macOS / Linux いずれでも動作するように書かれています:

- hook スクリプトは `#!/usr/bin/env bash`
- 絶対パスは `$CLAUDE_PROJECT_DIR` で解決する
- `.claude/agent-memory/` は `.gitignore` で除外（個人のメモ）
- `playwright-cli` skill はリポジトリ層の symlink を撤去し、`~/.claude/skills/`
  （ユーザー層。agent-assets の `install.sh` が張る）だけで足りるようにした
  （agent-assets#276、2026-09-03）

新しい開発者がリポジトリをクローンした場合、追加でやることはありません。Claude Code が
`settings.json` を読み込めば hook が有効になります。
