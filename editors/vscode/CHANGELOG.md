# Changelog

## [Unreleased]

### Added

- Snippets for day-precision (`YYYY-MM-DD`) and month-precision (`YYYY-MM`) literals
  - `span-day`, `span-month`, `event-day`, `event_range`, `tl-day`
- `event_range` snippet (previously missing)

### Changed

- **Breaking:** Minimum required VS Code version raised from `^1.75.0` (Feb 2023) to `^1.90.0` (Jun 2024) ([#278](https://github.com/keroway/timeline-dsl/issues/278))
  - Aligns the extension with currently supported VS Code APIs and Marketplace recommendations.
  - Users on VS Code earlier than 1.90 should update to install or upgrade this extension.

### Notes

- Syntax highlighting for date literals (`YYYY-MM-DD` / `YYYY-MM`) is already supported via the `number` pattern in `tdsl.tmLanguage.json` since the v1.9.0 month/day precision release ([#243](https://github.com/keroway/timeline-dsl/issues/243))

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
