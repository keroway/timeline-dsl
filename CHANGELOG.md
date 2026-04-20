# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

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

[0.1.0]: https://github.com/keroway/timeline-dsl/releases/tag/v0.1.0
