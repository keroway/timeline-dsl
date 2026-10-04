# CLAUDE.md -- Timeline DSL プロジェクト指示書

## プロジェクト概要

年表特化のDSLコンパイラ。`.tdsl` ファイルをパースし、WikidataからデータをインポートしてJSON IRに変換する。Rustで実装。

パス限定で済む詳細（クレート構成・コンパイルパイプライン・IR構造、DSL文法の変更手順、Wikidataプロパティ表、MVP実装状況、サンプルファイル一覧、DSL実装の注意点）は `.claude/rules/*.md` に分割済み（該当パスを触るターンにだけ読み込まれる）。本ファイルは全ターン共通の必須事項のみを置く。

## ビルド・テスト

```bash
# ビルド
cargo build --workspace

# 全テスト実行
cargo test --workspace

# 特定クレートのテスト
cargo test -p tdsl-parser
cargo test -p tdsl-core
cargo test -p tdsl-wikidata

# CLIの実行（例）
cargo run -p tdsl-cli -- build examples/china_dynasties.tdsl --pretty
cargo run -p tdsl-cli -- check examples/china_dynasties.tdsl
cargo run -p tdsl-cli -- build examples/china_with_import.tdsl --pretty
cargo run -p tdsl-cli -- build examples/china_with_import.tdsl --offline --pretty
cargo run -p tdsl-cli -- fetch Q7209 --lang ja,en
```

## コーディング規約

- **Edition**: Rust 2024
- **エラー処理**: `thiserror` でエラー型定義、`miette` で整形出力
- **非同期**: `tokio`（Wikidata fetch に使用）
- **パーサ**: `pest` PEGパーサジェネレータ。文法変更は `grammar.pest` を編集後 `builder.rs` も更新すること（詳細手順: `.claude/rules/dsl-grammar-change.md`）
- **テスト**: 各クレートの `lib.rs` 末尾に `#[cfg(test)]` で統合テスト
- **シリアライズ**: IR型は `serde::Serialize + Deserialize` を derive

## 未実装 / 意図的に対応しない機能

- `map source` -- `map` ブロック内の `source:` プロパティ指定。`MapProp` に `Source` バリアントが存在せず、pest 文法（`grammar.pest` の `map_prop`）がそもそも受理しないためパース時点で拒否される（item レベルの `source wd:<QID>` のみ有効）
- サブ秒（ミリ秒未満）精度
- IANA タイムゾーン名（例: `Asia/Tokyo`）による DST 自動解決 -- 意図的に非対応と確定済み（ADR-0007、2026-07-26 決定）。固定の数値 UTC オフセット（`+09:00` 等）のみサポート

これらに遭遇した場合は silent fallback ではなく必ずパース/lowering エラーで拒否する（「No silent fallback」原則、`.claude/rules/implementation-strict.md` §2）。秒精度（`DateTimeSecond`）と UTC オフセット（`DateTimeOffset` / `DateTimeSecondOffset`）自体は #612〜#616（ADR-0003）で実装済みなので、上記の未実装リストに含めない。

この節は NO-GO 判定の絶対基準（`.claude/rules/implementation-strict.md` §2 / `.claude/agents/app-dev-director.md`）として参照されるため、パス限定にせず本ファイルに置く。

## Claude Code 用セットアップ（このリポジトリ）

このリポジトリには Claude Code 用の補助設定が `.claude/` 配下にコミットされている。実装時は以下を参照・利用すること。

- **`.claude/rules/implementation-strict.md`** -- 実装方針の strict ルール。本ファイル（`AGENTS.md` は本ファイルへの symlink）に加えて必ず参照する。NO-GO パターン、コードレベルの規約、テスト最低ライン、PR 提出前ゲートを定義。
- **`.claude/agents/rust-app-developer.md`** -- Rust 実装用サブエージェント。文法・lowering・Wikidata 連携の実装はこれに委譲する。
- **`.claude/agents/app-dev-director.md`** -- 設計判断・スコープ整理・仕様整合性レビュー用サブエージェント。実装着手前のレビュー、実装後の整合性チェックに使う。
- **`.claude/commands/fix-pr.md`** -- `/fix-pr [PR番号]` で自分の PR の CI 失敗を自動修正する。
- **`.claude/hooks/post-stop-check.sh`** -- Stop hook。応答完了時に変更ファイルを見て `cargo fmt --check` / `cargo clippy -D warnings` / `cargo test --workspace` を実行（WebUI 変更時は `npm run lint` も）。スキップは `TIMELINE_DSL_SKIP_STOP_HOOK=1`。

実装着手時は `.claude/rules/implementation-strict.md` の「§3 着手前チェックリスト」を埋めてから書き始めること。

## Agent skills

`mattpocock-skills` プラグインの engineering スキル群がリポジトリ固有設定として読む。

### Issue tracker

Issue は GitHub Issues（`keroway/timeline-dsl`）で管理し、`gh` CLI で操作する。
`area:*` はこのリポジトリ固有の分類ラベル。
詳細は [`docs/agents/issue-tracker.md`](docs/agents/issue-tracker.md)。

### Triage labels

canonical な5役割のうち `ready-for-human` だけ既存の `needs-human` へ写像し、
残りはそのままラベル名として使う（この写像はワークスペース共通の正典で、
このリポジトリ単独で書き換えない）。**既存の `refined` は 2026-08-30 に
`ready-for-agent` へリネーム済み**（同義ラベルの並立を避けるため）。
無人ループの除外ラベル一覧は `agent-assets` の
`docs/agents/triage-labels.md` を正とする。
詳細は [`docs/agents/triage-labels.md`](docs/agents/triage-labels.md)。

### Domain docs

single-context。用語の正典は [`docs/dsl-spec.md`](docs/dsl-spec.md)、決定記録は
`docs/adr/`。`CONTEXT.md` は未作成で、無い場合は黙って先に進む。
詳細は [`docs/agents/domain.md`](docs/agents/domain.md)。

## Codex 向け運用ルール

Codex 向けの横断運用ルールは `keroway/CLAUDE.md` ではなく
[agent-assets `docs/codex-common-instructions.md`](https://github.com/keroway/agent-assets/blob/main/docs/codex-common-instructions.md)
を正典とする（Codex は git ルートより上の AGENTS.md を読まないため）。
