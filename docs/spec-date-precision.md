# 月・日精度の時間表現 仕様

> 関連 Issue: [#64](https://github.com/keroway/timeline-dsl/issues/64) / [#242](https://github.com/keroway/timeline-dsl/issues/242) / [#243](https://github.com/keroway/timeline-dsl/issues/243)
> ステータス: **設計確定**（実装は #243 以降で進行）

## 背景

Timeline DSL の時刻値は当初「整数の年」のみを受け付けていたが、近代史・スポーツイベント・プロジェクト管理などのユースケースで月・日精度が必要となる。

調査の結果、IR / Wikidata クライアント / Renderer 層は既に月精度のサポートが組み込まれており、実装の主戦場は **DSL 構文 / AST / 静的 lowering** の3点に絞られる。本書はその設計を確定する。

## 1. 文法（パーサ）

### 1.1 リテラル

4 種類の精度を許可する:

| 形式 | 例 | 精度 |
|---|---|---|
| `YYYY` | `1969`, `-206` | 年 |
| `YYYY-MM` | `1969-07`, `-0206-01` | 月 |
| `YYYY-MM-DD` | `1969-07-20`, `-0206-01-15` | 日 |
| `YYYY-MM-DDTHH:MM` | `1969-07-20T20:17` | 分 |

### 1.2 PEG ルール（実装ガイド）

```pest
year_lit       = @{ "-"? ~ ASCII_DIGIT+ }                       // year 精度: 桁数制限なし
year_pos       = @{ ASCII_DIGIT{1,4} }                          // 月日付きでは符号なし・最大4桁
year_month_lit = ${ year_pos ~ "-" ~ ASCII_DIGIT{2} ~ !("-" ~ ASCII_DIGIT) }
date_lit       = ${ year_pos ~ "-" ~ ASCII_DIGIT{2} ~ "-" ~ ASCII_DIGIT{2} }
time_value     = { date_lit | year_month_lit | year_lit }       // 長いものから優先
```

- `${ }`（atomic）でトークン間の空白を許可しないことで、`1969 - 07 - 20` のような誤マッチを防ぐ
- 検証は builder 側で実施: 月は 1〜12、日は 1〜31。範囲外はパースエラー
- カレンダー妥当性チェック（2月30日など）は lowering 側の責務（lint で警告／エラー）
- `year_lit` は桁数制限を設けない: 既存 `range 0..10000;` のような4桁超リテラルを後方互換で許容するため。月日精度の `year_pos` のみ最大 4 桁に制限する（`10000-07-20` のような5桁年付き日付はサポート外）。
- `year_month_lit` の末尾には `!("-" ~ ASCII_DIGIT)` を置き、`date_lit` との曖昧性を回避する

### 1.3 紀元前（負の年）

紀元前も月日・時分精度をサポートする。

- 月日付き BCE は符号付き 1〜4 桁年で表す（例: `-0206-01-15`）
- `TimeValue` は負の年でも `month` / `day` / `hour` / `minute` を保持する
- Wikidata 由来データも BCE の月日・時分精度を丸めず IR に保持する

### 1.4 範囲（`..`）

`time_value..time_value` で記述。両端の精度が異なってもよい。

```tdsl
span ww2 1939-09-01..1945-09-02 "第二次世界大戦" {};
event moon 1969-07-20 "月面着陸" {};
span partial 1900..1969-07-20 "混在許可" {};   // year + date OK
```

精度混在時の補完規則（lowering で適用）:

- 範囲の **start** に year 単位が来た場合 → `MM=01, DD=01` 相当
- 範囲の **end** に year 単位が来た場合 → `MM=12, DD=31` 相当
- 範囲の **start/end** で `YearMonth` が来た場合 → start は `DD=01`、end はその月の末日
- IR 上は補完前の精度（`*_month` / `*_day` が `Some` か `None` か）を保持する。**補完は描画時の x 座標計算でのみ適用**（IR には完全な year/month/day を埋めない）

### 1.5 `range` ディレクティブ

`timeline` ブロック内の `range` も `time_value` を受け付けるよう拡張する:

```tdsl
timeline "近代史" {
    unit month;
    range 1939-01..1946-01;
}
```

精度は IR の `Meta::range_*` フィールドに保存される（§3.2 参照）。年に丸めて `Meta::range` に入れるだけだと `range 1939-09..1945-09` と `range 1939..1945` の区別が失われ、Renderer が月単位で正しく描画できないため、precision フィールドの追加が必須となる。

## 2. AST 型

```rust
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum TimeValue {
    Year(i64),
    YearMonth(i64, u8),    // year, month (1-12)
    Date(i64, u8, u8),     // year, month (1-12), day (1-31)
}
```

影響範囲（`crates/tdsl-parser/src/ast.rs`）:

| 型 | 既存 | 変更後 |
|---|---|---|
| `RangeExpr::start` / `end` | `i64` | `TimeValue` |
| `SpanDecl::start` / `end` | `i64` | `TimeValue` |
| `EventDecl::time` | `i64` | `TimeValue` |
| `EventRangeDecl::start` / `end` | `i64` | `TimeValue` |

builder.rs では PEG の `time_value` ノードを `TimeValue` に変換し、月日の値域検証を行う。

## 3. IR 表現

### 3.1 Item variant — 変更なし

既存の Item variant に存在する以下フィールドをそのまま利用する:

- `Span`: `start_month`, `start_day`, `end_month`, `end_day`
- `Event`: `time_month`, `time_day`
- `EventRange`: `start_month`, `start_day`, `end_month`, `end_day`

すべて `Option<u8>`、JSON 出力時は `None` のとき省略される（既存挙動）。

### 3.2 `Meta::range` — precision フィールドを追加

`Meta::range: (i64, i64)` のままだと §1.5 で許可した `range 1939-09..1945-09;` のような月日付き range の精度が失われる（year に丸めると Renderer の `month_ticks()` は 1939年1月〜1945年12月 を描画してしまい、ユーザー意図の 1939年9月〜1945年9月 と一致しない、いわゆる **year-range regression**）。

これを防ぐため、Item と同じパターンで `Meta` に precision フィールドを追加する:

```rust
pub struct Meta {
    pub title: String,
    pub unit: String,
    pub range: (i64, i64),                          // year（既存・年に丸めた値）
    #[serde(skip_serializing_if = "Option::is_none")]
    pub range_start_month: Option<u8>,              // 新規
    #[serde(skip_serializing_if = "Option::is_none")]
    pub range_start_day:   Option<u8>,              // 新規
    #[serde(skip_serializing_if = "Option::is_none")]
    pub range_end_month:   Option<u8>,              // 新規
    #[serde(skip_serializing_if = "Option::is_none")]
    pub range_end_day:     Option<u8>,              // 新規
    pub calendar: String,
    pub color_map: HashMap<String, String>,
}
```

- `range` は引き続き year を保持（既存の Renderer / WebUI の表示ロジックを壊さないため）
- 新規 4 フィールドは `Option<u8>` + `skip_serializing_if = "Option::is_none"` で **既存 IR JSON の後方互換を保つ**
- `range 1939-09..1945-09;` → `range = (1939, 1945)`, `range_start_month = Some(9)`, `range_end_month = Some(9)`, day は `None`
- `range 1939..1945;` → `range = (1939, 1945)`, 4 フィールドとも `None`（従来挙動と完全一致）

### 3.3 `precision` 専用フィールドは追加しない

月・日が `Some` か否かから導出可能で、冗長を避ける。

### 3.4 後方互換

- 既存の年精度のみの IR JSON は変化なし（新フィールドはすべて `None` で skip 出力）
- WebUI / WASM / decompile は既に Item の `*_month` / `*_day` を受ける構造になっている。`Meta::range_*` の追加対応のみ必要
- decompile（IR → `.tdsl` 逆変換）は `Some` の場合に `YYYY-MM` / `YYYY-MM-DD` 形式で出力するよう拡張する（Item / Meta::range の両方）

## 4. Wikidata 整合性

既存実装で対応済み:

- `crates/tdsl-wikidata/src/entity.rs` の `time_value_to_timepoint()` が precision 9/10/11 に応じて year/month/day を抽出
- `crates/tdsl-core/src/lower.rs` の `eval_claim_expr()` が `claim(P569).month` / `.day` アクセサを評価し、対応する Item フィールドにセット

追加対応:

- 紀元前データ（`year < 0`）の場合も、Wikidata precision に応じて month/day/hour/minute を保持する

## 5. レンダリング

### 5.1 `unit month`

既に実装済み:

- `crates/tdsl-render/src/layout.rs:216` `month_ticks()` が `unit == "month"` のとき年内に月目盛り（2〜12月）を生成
- `to_year_frac()` が year/month/day を分数年に変換

`range` を `YYYY-MM` 受けに拡張すれば、追加コードなしで意味のある軸表示になる見込み。

### 5.2 `unit day`

**本仕様外（別 Issue で対応）**。実装時に必要となる項目:

- `day_ticks()` の追加（月内日目盛り）
- 月またぎラベルの設計（年・月・日のレベル別ラベル）
- スケール（pixel-per-day）の妥当な範囲設計

## 6. 実装 Issue の分解

| Issue | 内容 | 依存 |
|---|---|---|
| **#243** (更新) | parser + AST: `time_value` PEG ルール、`TimeValue` enum、builder | なし |
| **新規 A** | core lowering: 静的アイテムへの月日反映、混在範囲の補完規則、紀元前丸め、`range` の `time_value` 化、decompile 拡張 | #243 |
| **新規 B** | render(day): `unit day` の day_ticks と日精度ラベル | 新規 A |

## 受け入れ条件（#242）

- [x] 上記 5 項目の設計決定を本ドキュメント / Issue コメントとして記録
- [ ] 設計を元に実装 Issue を分解して起票（`#243` 更新 + 新規2件）
- [x] `docs/spec-date-precision.md` として追加
