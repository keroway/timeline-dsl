---
paths:
  - "examples/**"
---

# サンプルファイル

CLAUDE.md を補完するパス限定ルール。矛盾があれば CLAUDE.md を正とする。

- `examples/china_dynasties.tdsl` -- 静的定義のみ（インポートなし）
- `examples/china_with_import.tdsl` -- Wikidata連携つき
- `examples/japanese_history.tdsl` -- 日本史
- `examples/samurai_wikidata.tdsl` -- 武将（Wikidata連携）
- `examples/world_wars.tdsl` -- 世界大戦
- `examples/sci_tech_timeline.tdsl` -- 科学技術史
- `examples/fictional_empire.tdsl` -- 架空の帝国（CSV連携例付き）
- `examples/template_apply_example.tdsl` -- `template` / `apply` 構文の使用例
- `examples/grouped_dynasties.tdsl` -- `group` ブロックの使用例（静的定義のみ）
- `examples/officeholder_wikidata.tdsl` -- `expand claim(P39)` / `qualifier(P580/P582)` の使用例（Wikidata連携）
- `examples/iss_docking_second_precision.tdsl` -- 秒精度 + UTC(`Z`)オフセットの使用例（#612〜#616、ADR 0003、静的定義のみ）
- `examples/global_conference_timezones.tdsl` -- 複数タイムゾーン（`+09:00`/`-05:00`/`Z`）の使用例とoffset付き値同士のUTC正規化比較（#612〜#616、ADR 0003 D2、静的定義のみ）
- `examples/feature_showcase.tdsl` -- `note` / `link` / `color`（block_options）・open-ended `now` の使用例（#663、静的定義のみ）
- `examples/china_dynasties_filtered.tdsl` -- `filter` 句によるインポートエンティティの絞り込み例（#142、Wikidata連携）
