# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [1.0.0] - 2026-05-04

### Added

- **template / apply 構文**: 共通フォーマットをテンプレート化して再利用できる DSL 構文を追加（#39）
- **Wikidataキャッシュ**: 取得結果を `~/.cache/tdsl/` に保存し TTL/オフライン連携を実現（#28）
- **Wikidata APIリトライ**: HTTP 429・5xx に対する exponential backoff リトライ（最大3回）を追加（#48）
- **SVG直接出力**: `tdsl render --format svg` でスタンドアロン SVG ファイルを出力（#50）
- **VS Code構文ハイライト**: `editors/vscode/` に TextMate grammar ベースの VS Code 拡張を追加（#52）
- **Homebrew formula**: `brew tap keroway/tap && brew install tdsl` によるインストールをサポート（#57）
- **E2Eテスト拡充**: 全 12 CLIサブコマンドの正常系・異常系を網羅した E2E スクリプトを整備（#59）
- **Getting Startedチュートリアル**: `docs/tutorial.md` を追加（#54）
- **サンプルファイル拡充**: 日本史・戦国武将（Wikidata連携）・世界大戦・科学技術の4サンプルを追加（#55）

### Fixed

- エラー診断の改善: 未定義lane参照時に利用可能な lane 候補を提示（#40）

### Changed

- リリースワークフローに Homebrew formula 自動更新ジョブを追加
- README.md にエディタサポートセクションと Homebrew インストール手順を追加

## [0.1.0] - 2026-04-20

### Added

- PEG文法 + パーサ（7種のstatement: timeline, lane, span, event, event_range, import, map）
- AST → IR変換（静的定義 / Wikidata連携 両方）
- Wikidata HTTPクライアント（wbgetentities API, wbsearchentities, SPARQL）
- CLI 12サブコマンド: `build` / `check` / `ast` / `fetch` / `search` / `inspect` / `resolve` / `scaffold` / `render` / `init` / `import-csv` / `lint`
- JSON IR出力（`origin` フィールドを含む）
- コメント構文（行 `//` / ブロック `/* */`）
- `map` の `target_type` enum 検証（span / event / event_range のみ許可）
- imported item の `source` を `wd:<entity_id>` で自動付与
- 日本語 lane 名で `as` 省略時の `lane_N` 自動採番
- 静的アイテム（event / event_range）の `sources[]` 登録
- 再インポートポリシー（merge_by_source / overwrite_imported / keep_manual）
- `query "SPARQL" as alias` による複数エンティティ一括インポート
- HTMLレンダリング（`tdsl-render` クレート、インラインSVG）
- `tdsl lint` による品質チェックと自動修正（`--fix`）
- validate における `start > end` チェック
- SPARQL QID 抽出改善

[1.0.0]: https://github.com/keroway/timeline-dsl/releases/compare/v0.1.0...v1.0.0
[0.1.0]: https://github.com/keroway/timeline-dsl/releases/tag/v0.1.0
