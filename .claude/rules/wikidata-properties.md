---
paths:
  - "crates/tdsl-wikidata/**"
  - "crates/tdsl-core/src/lower/**"
  - "examples/**"
---

# Wikidataプロパティ（頻用）

CLAUDE.md を補完するパス限定ルール。矛盾があれば CLAUDE.md を正とする。

| プロパティ | 意味 | DSL式 |
|---|---|---|
| P569 | 誕生年 | `claim(P569).year` |
| P570 | 死亡年 | `claim(P570).year` |
| P571 | 成立年 | `claim(P571).year` |
| P576 | 消滅年 | `claim(P576).year` |
| P580 | 開始時点 | `claim(P580).year` |
| P582 | 終了時点 | `claim(P582).year` |
