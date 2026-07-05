# Changelog

## [Unreleased]

## [1.24.0] - 2026-07-05

### Changed

- Sync extension version with timeline-dsl v1.24.0 release

## [1.23.0] - 2026-07-03

### Changed

- Sync extension version with timeline-dsl v1.23.0 release

## [1.22.0] - 2026-06-26

### Changed

- Sync extension version with timeline-dsl v1.22.0 release

## [1.21.0] - 2026-06-24

### Changed

- Sync extension version with timeline-dsl v1.21.0 release

## [1.20.0] - 2026-06-22

### Changed

- Sync extension version with timeline-dsl v1.20.0 release

## [1.19.0] - 2026-06-17

### Added

- LSP クライアント統合 (#388 / #470): `tdsl lsp` を spawn して VS Code に LanguageClient 接続
  - 診断（エラー・警告）、補完、hover、定義ジャンプ、コードアクション、ドキュメントシンボル、参照検索、リネーム、フォーマットが VS Code 上で動作する
  - PATH 優先 + `timelineDsl.serverPath` 設定でバイナリの上書き解決が可能
  - バイナリ解決に失敗した場合にインストール手順を案内するエラー通知を表示する

## [1.18.0] - 2026-06-16

### Changed

- Sync extension version with timeline-dsl v1.18.0 release

## [1.17.0] - 2026-06-10

### Changed

- Sync extension version with timeline-dsl v1.17.0 release

## [1.16.0] - 2026-06-07

### Changed

- Sync extension version with timeline-dsl v1.16.0 release

## [1.15.0] - 2026-06-03

### Changed

- Sync extension version with timeline-dsl v1.15.0 release

## [1.14.0] - 2026-06-02

### Changed

- Sync extension version with timeline-dsl v1.14.0 release

## [1.13.0] - 2026-05-30

### Changed

- Sync extension version with timeline-dsl v1.13.0 release

## [1.12.0] - 2026-05-27

### Changed

- Sync extension version with timeline-dsl v1.12.0 release

## [1.11.0] - 2026-05-24

### Changed

- Sync extension version with timeline-dsl v1.11.0 release

## [1.10.1] - 2026-05-23

### Changed

- Sync extension version with timeline-dsl v1.10.1 release

## [1.10.0] - 2026-05-20

### Added

- Snippets for day-precision (`YYYY-MM-DD`) and month-precision (`YYYY-MM`) literals ([#259](https://github.com/keroway/timeline-dsl/issues/259))
  - `span-day`, `span-month`, `event-day`, `event_range`, `tl-day`
- `event_range` snippet (previously missing)

### Notes

- Syntax highlighting for date literals (`YYYY-MM-DD` / `YYYY-MM`) is already supported via the `number` pattern in `tdsl.tmLanguage.json` since the v1.9.0 month/day precision release ([#243](https://github.com/keroway/timeline-dsl/issues/243))
- Sync extension version with timeline-dsl v1.10.0 release

## [1.5.0] - 2026-05-13

### Added

- Sync extension version with timeline-dsl v1.5.0 release
- Add VS Code snippets for `timeline`, `lane`, and `span` blocks

## [1.2.2] - 2026-05-06

### Changed

- Add extension README with feature description, example, and links

## [1.2.1] - 2026-05-06

### Changed

- Update GitHub Actions workflow to Node.js 24 / setup-node@v5

## [1.2.0] - 2026-05-06

### Added

- Initial release on VS Code Marketplace
- Syntax highlighting for Timeline DSL (`.tdsl`) files
- Highlights: keywords, strings, comments, Wikidata IDs, claim/label expressions, numeric literals
