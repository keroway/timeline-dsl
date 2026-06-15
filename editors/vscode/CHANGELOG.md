# Changelog

## [Unreleased]

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
