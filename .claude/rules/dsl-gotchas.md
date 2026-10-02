---
paths:
  - "crates/tdsl-parser/**"
  - "crates/tdsl-core/**"
  - "crates/tdsl-wikidata/**"
---

# 注意点

CLAUDE.md を補完するパス限定ルール。矛盾があれば CLAUDE.md を正とする。

- Wikidata APIにはレート制限あり。大量fetchする場合は `--offline` で開発し、最終確認時にオンラインビルド
- 負の年（紀元前）は整数で表現: `-206` = 紀元前206年
- lane IDの `as` 省略時はラベルからASCIIスラッグを自動生成。日本語のみの場合は `lane_N` に自動採番
- `source wd:QXXX` はWikidata出典を表し、IR の sources に CC0 ライセンスとして記録
- map ブロックの `source` プロパティは廃止済み。imported item の source は `wd:<entity_id>` で自動付与
- map の `target_type` は `span` / `event` / `event_range` のみ。不正値はパースエラー
- `wd.xxx` の entity_key が import に存在しない場合はエラー（全件フォールバックしない）
- imported item の `origin` は lowering で常に `"wikidata"` に固定される（`crates/tdsl-core/src/lower/mapping.rs`）。静的アイテムの `origin` は DSL の `origin` オプションで宣言した値がそのまま使われ、lowering は上書きしない（`crates/tdsl-core/src/lower/static_items.rs`）
