# 移行ガイド: 秒精度・UTCオフセット（ADR 0003）

秒精度（`HH:MM:SS`）と UTC オフセット（`Z` / `±HH:MM`）は、#612〜#616（親: #610）で追加された機能です。本ドキュメントは、既存の minute-level（秒・offset なし）`.tdsl` ファイルへの影響と、Wikidata インポートと静的 offset 付きデータを混在させたい場合の対処方法をまとめます。

詳細な設計判断は [ADR 0003: 秒精度 + タイムゾーン/オフセットのデータモデル](./adr/0003-second-precision-and-timezone-offset-data-model.md) を参照してください。

## 破壊的変更はありません

**既存の minute-level（秒・offset なし）`.tdsl` ファイルは変更なくそのまま動作します。**

秒（`:SS`）とオフセット（`Z` / `±HH:MM`）は、いずれも既存の時刻構文 `YYYY-MM-DDTHH:MM` の後ろに追加される **任意のオプション部** です。

```ebnf
<date_time> ::= "YYYY-MM-DDTHH:MM" [":SS"] ["Z" | "±HH:MM"]
```

- 秒を省略すれば `DateTime`（既存の分精度 variant）のまま
- オフセットを省略すれば「offset なし」（既存の暦時刻セマンティクス）のまま
- 両方省略すれば、v1.25 以前と完全に同一の構文・意味

移行手順は不要です。既存ファイルの回帰は `crates/tdsl-core/src/tests/golden.rs` の `existing_minute_level_examples_still_parse_and_lower_unchanged` テストで保証しています（`examples/*.tdsl` の主要な既存静的サンプルを実際に `tdsl_parser::parse` + `lower_static` に通し、エラーなく成功することを検証）。

## 新しい比較エラー: `MixedOffsetComparison`

秒・offset 自体は非破壊ですが、**offset を新しく使い始める場合** に踏まえておくべき挙動が1つあります（ADR 0003 D2）。

### ルール

- offset 付き値（`+09:00`, `-05:00`, `Z` 等）同士は、UTC に正規化してから比較する
- offset なし値同士は、従来どおり暦時刻の値そのままで比較する
- **offset 付き値と offset なし値を同一の比較コンテキストで混在させると、`MixedOffsetComparison` エラーになる**

「同一の比較コンテキスト」とは、具体的には以下のような箇所です。

- 同一 `span` / `event_range` の `start..end`
- `timeline { range start..end; }` の範囲チェック
- レンダリング時のソート・順序付け

この比較は暗黙に「offset なしは UTC とみなす」という正規化を **行いません**（CLAUDE.md「No silent fallback」原則）。曖昧な比較を機械的に解決せず、著者にどちらかへの統一を求めます。

```tdsl
# エラーになる例（offset ありと offset なしが同一 span に混在）
span a 2024-01-01T10:00:00+09:00..2024-01-02T10:00 "S" {};
# => Error: Cannot compare a UTC-offset time value with a value
#    that has no offset (author must make both sides consistent):
#    2024-01-01T10:00:00+09:00 vs 2024-01-02T10:00

# 修正例1: 両方に offset を付与
span a 2024-01-01T10:00:00+09:00..2024-01-02T10:00+09:00 "S" {};

# 修正例2: 両方から offset を外す
span a 2024-01-01T10:00:00..2024-01-02T10:00 "S" {};
```

## Wikidata インポートと静的 offset 付きデータの混在

Wikidata インポート（`import wikidata { entity Qxxxx; }` / `map wd.xxx to ...`）で生成されるアイテムは、**常に offset なし**（`DateTime` / `DateTimeSecond`）として格納されます。Wikidata API が返す時刻は instant（UTC）ですが、既存の「裸の civil time」というセマンティクスを変えないため、インポート後の値には offset を付与しません。

このため、**静的アイテムに offset を付与し、それを同一の比較コンテキストで Wikidata インポートアイテムと混在させると、`MixedOffsetComparison` エラーになります**。

```tdsl
import wikidata as wd {
    entity Q7209 as han_dynasty;
}

map wd.han_dynasty to span {
    lane han;
    start claim(P571).year;  // Wikidata起点 → offset なしで格納される
    end claim(P576).year;
}

// 同一 lane 内の別アイテムに offset を付けても、start/end が別アイテムの
// 別スパンであれば直接の比較コンテキストにはならず、通常はエラーにならない。
// ただし同一アイテムの start/end や、同一比較コンテキストで明示的に
// 混在させるロジックを書くとエラーになる。
span han_static_offset 2024-01-01T10:00:00+09:00..2024-01-02T10:00:00+09:00 "静的補足イベント" {};
```

### 対処方法

同一比較コンテキストで両方を扱いたい場合は、以下のいずれかで統一してください。

1. **静的データ側の offset を外す**（推奨）: Wikidata データは常に offset なしなので、静的データ側も offset なしに揃えるのが最も単純です。この場合、静的データの時刻は「著者が意図した現地時刻の値をそのまま暦時刻として扱う」という既存のセマンティクスになります（タイムゾーン変換は行われません）。
2. **比較コンテキストを分離する**: 同一 `span`/`event_range` の `start`/`end` に Wikidata 由来の値と静的 offset 付き値を混在させず、別々の lane やアイテムに分けて記述する。

offset 付き値を積極的に使いたい場合（例: 複数タイムゾーンの国際イベントを正確に UTC 基準で比較したい）は、そのタイムライン全体を offset 付きの値のみで統一して構築することを推奨します。この場合、Wikidata インポートとの直接比較は避けるか、Wikidata 由来の値を別の比較コンテキスト（別 lane 等）に分離してください。

## 参考実装例

- [`examples/iss_docking_second_precision.tdsl`](../examples/iss_docking_second_precision.tdsl) — 秒精度 + UTC (`Z`) の一貫した使用例（静的データのみ）
- [`examples/global_conference_timezones.tdsl`](../examples/global_conference_timezones.tdsl) — 複数タイムゾーンオフセット（`+09:00` / `-05:00` / `Z`）を lane ごとに一貫して使用し、UTC 正規化比較で前後関係が自動解決される例（静的データのみ）

いずれのサンプルも `crates/tdsl-core/src/tests/golden.rs` のスナップショットテストで、IR への変換結果が意図せず変化しないことを継続的に検証しています。

## まとめ

| 状況 | 対応 |
|---|---|
| 既存の minute-level ファイルをそのまま使う | 何もしなくてよい（非破壊変更） |
| 秒精度だけを新しく使う（offset なし） | そのまま `:SS` を追加するだけ。既存の暦時刻比較ルールのまま |
| offset 付きの値を使う | 同一比較コンテキスト内では offset の有無を統一する |
| Wikidata インポートと offset 付き静的データを混在させたい | 静的データ側の offset を外すか、比較コンテキストを分離する |
