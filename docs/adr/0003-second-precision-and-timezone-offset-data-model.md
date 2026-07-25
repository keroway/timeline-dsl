# ADR 0003: 秒精度 + タイムゾーン/オフセットのデータモデル

- **Status**: Accepted
- **Date**: 2026-07-15
- **Deciders**: keroway
- **Related issues**: #611（本 ADR）, #610（親: sub-minute precision / timezone 再評価）, 実装先行 issue: #612（parser/AST）, #613（IR/schema/lowering/Wikidata）, #614（renderer/decompile/fmt/CSV）, #615（LSP/WASM）, #616（docs/移行）
- **Supersedes**: なし

## コンテキスト

現行の時刻表現は `crates/tdsl-parser/src/ast.rs` の `TimeValue` enum で、精度タグ付きの **civil time（タイムゾーン非依存の暦時刻）** をそのまま保持する設計になっている。

```rust
pub enum TimeValue {
    Year(i64),
    YearMonth(i64, u8),
    Date(i64, u8, u8),
    DateTime(i64, u8, u8, u8, u8), // year, month, day, hour, minute
}
```

- 外部日時クレート（`chrono` 等）には依存していない。
- 比較は `Eq`（精度差を区別する厳密等価）とは別に `to_sortable() -> (i64, u8, u8)` を明示的に呼び出すタプル比較で行う（`PartialOrd` は実装しない）。
- BCE（紀元前）は年を負の `i64` で表現し、既存の Wikidata インポート・パーサ双方で対応済み。
- `now` はビルド時点の UTC 年に解決される（#550）。offset の概念は現状存在しない＝すべての値はタイムゾーン注記のない「裸の」暦時刻。
- Wikidata インポート（`crates/tdsl-wikidata/src/entity.rs`）は `precision`（0〜13 の Wikidata 独自スケール）から year/month/day/hour/minute を抽出しており、API 上の時刻は常に UTC の instant だが、インポート後は「UTC で読み替えた裸の civil time」として `TimeValue` に格納される（offset は持たない）。

秒未満精度（分より細かい秒）と明示的なタイムゾーン/オフセットは、これまで意図的にスコープ外としてきた（README/dsl-spec に明記）。用途が録音・移動ログ等の秒単位イベントに広がり、再評価が必要になった。

## 決めること（受け入れ条件からの引用）

- IR が保持するのは instant か、civil time + offset か、precision-tag 付き値か。
- 正規化・順序付け（ordering）セマンティクスの定義。
- BCE日付・オフセット付きtimestamp・`now`・オフセット不在時の扱い。
- カレンダー挙動とバリデーション/エラーコード（曖昧・不正なTZは明示的に失敗、silent fallback禁止）。
- ブラウザ対応前提のバンドル/ランタイム影響の事前見積り。

## 決定事項

### D1. データモデル: 既存 precision-tag 付き civil-time enum を維持・拡張する（instant/epoch 方式は不採用）

`TimeValue` を **UTC epoch instant に刷新することはしない**。現行の「精度タグ付き civil time」設計をそのまま踏襲し、秒精度を新しい precision タグとして追加する。

```rust
pub enum TimeValue {
    Year(i64),
    YearMonth(i64, u8),
    Date(i64, u8, u8),
    DateTime(i64, u8, u8, u8, u8),               // 既存: y, m, d, h, mi（offsetなし）
    DateTimeSecond(i64, u8, u8, u8, u8, u8),      // 新規: y, m, d, h, mi, s（offsetなし）
    DateTimeOffset(i64, u8, u8, u8, u8, i16),     // 新規: y, m, d, h, mi, offset_minutes
    DateTimeSecondOffset(i64, u8, u8, u8, u8, u8, i16), // 新規: y, m, d, h, mi, s, offset_minutes
}
```

- **理由**: instant(epoch) 方式に刷新すると、既存の「入力した暦時刻がそのまま出力・比較・レンダリングに使われる」という不変条件（＝ローカルタイムゾーンの概念を持ち込まない）が壊れ、全消費箇所（parser/AST/lowering/renderer/decompile/CSV/WASM）が破壊的変更を受ける。既存の precision-tag 方式を維持し、新しい variant を追加するだけであれば、既存 variant（`Year`〜`DateTime`）に対する既存コードパスは無変更で動作し、後方互換性が自然に保たれる。
- `offset_minutes: i16` は分単位（例: `+09:00` → `540`、`-05:00` → `-300`）。offset は「時刻」ではなく「時刻に付与された注記」として扱う＝offset の有無自体が意味を持つ（Q2 参照）。
- variant 数が増える点はトレードオフとして許容する（比較対象案「外側に `struct Timestamp { value: TimeValue, offset_minutes: Option<i16> }` でラップ」は、`SpanDecl`/`EventDecl`/`EventRangeDecl` 等すべての呼び出し箇所の型が変わるためブラスト半径が大きく不採用。variant 追加の方が既存コードへの影響を局所化できる）。

### D2. 正規化・順序付けセマンティクス: offset 付き値同士は UTC 正規化して比較、offset なしとの混在比較はエラー

- **offset 付き値同士**（`DateTimeOffset` / `DateTimeSecondOffset`）は、`offset_minutes` を引いて UTC 相当の civil time に正規化してから比較する。
- **offset なし値**（`Year`〜`DateTimeSecond`）同士は、従来どおり暦時刻の値そのもので比較する（タイムゾーンの概念を持ち込まない）。
- **offset 付き値と offset なし値の比較は明示的エラーとする**（`LoweringError` の新規バリアント、例: `MixedOffsetComparison`）。offset なしの値を「暗黙に UTC」とみなして正規化することはしない。これは AGENTS.md §4.1「no silent fallback」の直接適用で、曖昧な比較を機械的に解決せず著者に明示させる。
  - 実務上の含意: 同一 lane / 同一 span-end 比較などで offset 付きと offset なしを混在させたい場合、著者はどちらかに揃える（全値に offset を付与するか、全値から外す）必要がある。
- Wikidata インポートは常に offset なし（`DateTime` / `DateTimeSecond`）として格納する（Wikidata API の instant は UTC だが、既存動作を変えず「裸の civil time」のまま）。したがって静的データに offset を付けた場合、同一比較コンテキストで Wikidata データと混在させると上記エラーになりうる — これは意図した挙動であり、ドキュメント（#616）で明記する。

### D3. BCE・`now`・オフセット不在の扱い

- **BCE**: 既存どおり負の `i64` 年で継続。秒・offset 追加による変更なし。
- **`now`**: 引き続きビルド時点の UTC 年に解決される、offset なしの値として扱う（`now` に offset を付与する構文は本 ADR のスコープ外・将来拡張）。
- **offset 不在**: 「タイムゾーン不明」ではなく「タイムゾーンという概念を持たない裸の暦時刻」として扱う（現行のセマンティクスを維持）。これは「UTCとして扱う」のとも「ローカルタイムとして扱う」のとも異なる第三の状態であり、D2 のとおり offset 付き値との比較を強制的にエラーにすることで曖昧さを排除する。

### D4. 構文・バリデーション

- 秒: `HH:MM:SS`（`:SS` は既存の `HH:MM` の後ろに追加のオプション部として拡張）。
- オフセット: `Z`（UTC）または `[+-]HH:MM`（例: `+09:00`, `-05:00`, `+05:45`, `+12:45` の 15 分単位も許容— 実在するタイムゾーンに合わせる）。
- 許容範囲: `-14:00` 〜 `+14:00`（実在するUTCオフセットの範囲。キリバス等の `+14:00` を上限とする）。
- 範囲外・書式不正（例: `+25:00`, `+09:3`, `+9:00`）は **parse エラーで拒否**。silent fallback・クランプは行わない（AGENTS.md §4.1 / ADR 0002 の margin 方針と同じ思想）。
- error-catalog に新規エラーコードを追記する（例: `E12x` 系列。具体的な番号は実装 issue #612 で確定）。

### D5. Wikidata precision マッピング

- Wikidata の time precision スケールは仕様上 `14`（秒）まで定義されている（実データでの出現頻度は低いが、API仕様としては存在する）。`precision >= 14` を秒精度として `DateTimeSecond` にマッピングする。
- Wikidata は offset を提供しない（常に UTC instant）ため、インポートされた値は常に offset なし（D2 のとおり）。

### D6. ブラウザ/WASM バンドル影響

- 秒・offset 演算は `chrono` 等の外部日時クレートを追加せず、既存の自前の整数演算（year/month/day/hour/minute/second の各フィールド比較、offset は分単位の整数演算）で実装する。これにより WASM バンドルサイズへの影響を最小限に抑える方針とする。
- 実測は #615（LSP/WASM影響計測）で行い、有意な増加が見られた場合は本 ADR にフィードバックする。

## 比較した代替案

| 方式 | 判定 | 理由 |
|---|---|---|
| **precision-tag civil-time enum の拡張（採用）** | ✅ 採用 | 既存アーキテクチャ・既存 variant への非破壊性・実装のブラスト半径の小ささを優先。 |
| instant (UTC epoch, 例: `i64` ミリ秒 + 別途表示用オフセット) への刷新 | ❌ 不採用 | 全消費箇所を破壊的変更する必要があり、Effort/Riskの見積り（親issue #610: Risk HIGH）に見合わない。「入力した暦表記がそのまま尊重される」という現行の直感的な挙動も失われる。 |
| civil time + offset を必須フィールド化した新構造体でラップ (`struct Timestamp { value, offset: Option<i16> }`) | ❌ 不採用 | `SpanDecl`/`EventDecl`/`EventRangeDecl` 等すべての型シグネチャを変更する必要がありブラスト半径が大きい。variant 追加よりリファクタリングコストが高い。 |
| offset なし値を暗黙に UTC とみなして offset 付き値と正規化比較 | ❌ 不採用 | AGENTS.md §4.1 の no-silent-fallback 原則に反する。著者の意図しない比較結果を生みうる（例: 静的データがローカルタイム前提で書かれていた場合に誤って UTC 起点で比較される）。 |

## 影響範囲

実装（#612〜#616）で変更が見込まれるファイル:

- `crates/tdsl-parser/src/ast.rs` — `TimeValue` に新 variant 追加
- `crates/tdsl-parser/src/grammar.pest`（または相当のpestファイル） — 秒・オフセット構文
- `crates/tdsl-core/src/ir.rs` / JSON schema — IR 側の精度/offset 反映
- `crates/tdsl-core/src/lower/mapping.rs` — 正規化・順序付け・`MixedOffsetComparison` エラー
- `crates/tdsl-wikidata/src/entity.rs` — precision 14 (秒) マッピング
- `crates/tdsl-render/src/layout.rs` — `unit second` ティック/ラベル
- decompile / `tdsl fmt` — 秒・offset のラウンドトリップ
- CSV export/import — 秒・offset 表現
- `crates/tdsl-lsp` — range/hover の秒/offset 表示
- `crates/tdsl-wasm` — バンドル影響計測
- `docs/dsl-spec.md` / `README.md` / `examples/` — 構文・移行ルールの追記

## 既知リスク

- variant 数の増加（4種類 → 7種類）により `match` 網羅性の保守コストが上がる。既存の `to_sortable()` 相当の拡張時に精度ごとの分岐漏れが起きやすいので、実装時は網羅チェック（`#[deny(non_exhaustive_omitted_patterns)]` 相当やテストマトリクス）を用意する。
- D2 の「混在比較はエラー」は、Wikidata インポートと静的 offset 付きデータを同一タイムライン上で混在させる既存ユースケースがあった場合に、ユーザー体験上の摩擦になりうる。#616 のドキュメントで移行時の注意点として明記する。

## 未決定事項（本 ADR の範囲外）

- `now` に明示的な offset を持たせるかどうかは ADR-0006 で検討中（現時点では実装方式未確定、`now` の解決粒度拡張を扱う先行 issue が必要）。
- ミリ秒未満（サブ秒）精度は本 ADR のスコープ外（AGENTS.md §5 に明記のとおり）。
- IANA タイムゾーン名（`Asia/Tokyo` 等）による DST 自動解決は対象外とすることを ADR-0007 で正式決定した（2026-07-26）。offset は数値（分単位）のみを扱う固定オフセットモデルであり、DST 遷移の自動計算は行わない（著者が期間ごとに offset を明示する）。

## 実測結果（#615, D6 フィードバック）

本節は #615（LSP range/hover + WASM バンドル影響計測）実施時の実測結果を追記する（D6 の事前見積もりに対する実測フィードバック）。

### WASM バンドルサイズ before/after

`wasm-pack build crates/tdsl-wasm --target web --release`（CI の `Test WASM build check` ジョブと同一コマンド）で生成される `tdsl_wasm_bg.wasm` のバイトサイズを比較した。

| 時点 | コミット | `.wasm` サイズ |
|---|---|---|
| before（#612以前、秒/offset未実装） | `fe99c60`（ADR 0003/0004 確定直後） | 736,801 bytes (≈719.5 KiB) |
| after（#612〜#614 実装済み） | 本 issue 作業ブランチ HEAD | 762,099 bytes (≈744.2 KiB) |
| 差分 | | +25,298 bytes (≈+24.7 KiB, +3.43%) |

**評価**: D6 で見積もったとおり、`chrono` 等の外部日時クレートを追加していないため増分は小さい（+3.43%）。増分の内訳は主に（a）`TimeValue` の新 variant 3種（`DateTimeSecond` / `DateTimeOffset` / `DateTimeSecondOffset`）とそれらを扱う `match` 分岐の増加、（b）IR（`ir.rs`）の新しい `Option<u8>`/`Option<i16>` フィールド群とその serde コード、（c）decompile/CSV/renderer の新しい分岐ロジックであり、想定範囲内。ブラウザ対応可否を左右するような有意な増加ではなく、D6の方針（自前整数演算の継続）は妥当と確認された。

### native + wasm 経路のIR一致確認

`crates/tdsl-wasm/src/lib.rs` のテスト（`compile_to_ir_roundtrips_second_precision_without_offset` 他）で、`compile_to_ir` が内部で呼ぶのと完全に同一のパス（`tdsl_parser::parse` + `lower_static_with_source` + プリティ直列化）を直接検証し、秒・offset付き `TimeValue` を含むソースが期待どおりのIR JSONフィールド（`start_second`/`start_offset_minutes` 等）を持つことを確認した。`tdsl-wasm` の cfg(test) は native triple で実行されるため、wasm32 ターゲット自体を実行した native/wasm 間の実行時ラウンドトリップ確認は行っていない（wasm-bindgen-test によるブラウザ/Node実行は本ADRのスコープ外）。DST/offset境界は `+09:00`/`-05:00`/`Z`/`+14:00` 等の固定オフセット値をテストデータとして使用しており、ホストロケールに依存しない。

### LSP hover/range

`crates/tdsl-lsp/src/hover.rs` の `compute_hover_with` に、時刻リテラル（`span`/`event`/`event_range`/`timeline range`）上のカーソル位置で `TimeValue` の精度（year〜second）と offset（有無・分数）を表示する hover を追加した。既存の lane/QID hoverとは独立なパスであり、word分割（`[A-Za-z0-9_]`）では分断されてしまう `:`/`+`/`-`/`T` を含むリテラル全体を対象にできるよう、ASTの `Span` 情報を基に元ソースを逆検索する方式を採用した。hover/range のユニットテストを `crates/tdsl-lsp/src/hover.rs` に追加済み。

### 結論

D6 の方針（外部日時クレートを追加しない自前整数演算）は妥当であり、WASMバンドルサイズ増加は +3.43% と軽微であることが実測で確認された。本 ADR へのフィードバックは不要（D6の方針変更は不要）。
