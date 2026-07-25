# エラーコードカタログ

`tdsl check` / `tdsl build` / `tdsl lint` が出力する診断メッセージの一覧です。
各エラーの原因と修正方法を記載しています。

---

## パースエラー（tdsl-parser）

ファイルの構文解析中に発生するエラーです。`.tdsl` ファイルの記法が誤っている場合に報告されます。

### E001: 構文エラー

**メッセージ**: `Syntax error: ...`

**原因**: DSLの文法に違反した記述があります。トークンの欠落、括弧の不一致、未知のキーワードなどが該当します。

**修正方法**: `tdsl check` / `tdsl build` はエラー行とその下にキャレット（`^`）でエラー箇所を強調表示します。表示位置を確認し、その記述を `docs/dsl-spec.md` の文法仕様と照合してください。

**表示例（v1.14.0 以降: miette キャレット表示）**

before（v1.13.0 以前）:

```
Error: Syntax error:  --> 1:1
  |
1 | xyzzy "bad" {
  | ^---
  |
  = expected file
```

after（v1.14.0 以降）:

```
tdsl::parse_error

  × 構文エラー: expected EOI, timeline_block, lane_decl, ...
   ╭─[myfile.tdsl:1:1]
 1 │ xyzzy "bad" {
   · ┬
   · ╰── ここに問題があります
 2 │   title "T";
   ╰────
  help: DSL 仕様書 docs/dsl-spec.md を確認してください
```

```
# 誤り例
span dynasty { ... }   # laneキーワードより前にlane参照を置けない

# 正しい例
lane "王朝" as dynasty
span dynasty 206..-9 "秦" { ... }
```

---

### E002: 整数変換エラー

**メッセージ**: `Invalid integer at {location}: {value}`

**原因**: 年数として解釈できない値が記述されています。

**修正方法**: 年数は整数で記述してください。紀元前は負数（例: `-206`）で表します。小数・文字列は使用できません。

```
# 誤り
span dynasty 200bc..0 "秦"

# 正しい
span dynasty -206..-9 "秦"
```

---

### E003: 不明な再インポートポリシー

**メッセージ**: `Unknown re-import policy: {value}`

**原因**: `import` ブロックの `on_reimport` に指定したポリシー名が不正です。

**修正方法**: 使用できるポリシーは以下の3つです。

| 値 | 説明 |
|---|---|
| `merge_by_source` | ソースが同じアイテムをマージ（デフォルト） |
| `overwrite_imported` | インポートアイテムを上書き |
| `keep_manual` | 手動アイテムを優先して保持 |

---

### E004: 不明な map ターゲット型

**メッセージ**: `Unknown map target type '{value}' (expected one of: span, event, event_range)`

**原因**: `map <alias> to <target_type> { ... }` の `<target_type>` に `span` / `event` / `event_range` 以外の値が指定されています。

**修正方法**: `<target_type>` は `span`・`event`・`event_range` のいずれかを指定してください（`to` の直後に置きます）。

```tdsl
# 誤り（timeline は target_type として無効）
map wd.han_dynasty to timeline {
    lane han;
}

# 正しい
map wd.han_dynasty to span {
    lane han;
    start claim(P571).year;
    end claim(P576).year;
}
```

> `target_type` ごとの生成アイテム種別・必須プロパティは [dsl-spec の map セクション](./dsl-spec.md#map) を参照してください。

---

### E005: 予期しないルール

**メッセージ**: `Unexpected rule {rule} at {location}`

**原因**: パーサ内部でAST変換中に予期しない構文ルールが検出されました。通常は発生しません。

**修正方法**: `.tdsl` ファイルを単純化して再現箇所を特定し、[Issue](https://github.com/keroway/timeline-dsl/issues) に報告してください。

---

### E006: 不正な月/日

**メッセージ**: `Invalid month at {location}: {value} (expected 1-12)` / `Invalid day at {location}: {value} (expected 1-31)`

**原因**: 時刻リテラル（`YYYY-MM-DD` / `YYYY-MM-DDTHH:MM[:SS][±HH:MM]`）の月が 1〜12、または日が 1〜31 の範囲外です。

**修正方法**: 月は 1〜12、日は 1〜31 の範囲で指定してください。カレンダーの対応日数チェック（うるう年等）は行わず、単純な範囲チェックのみです。

```
# 誤り
 event a 2024-13-01 "E" {};

# 正しい
event a 2024-12-01 "E" {};
```

---

### E007: 不正な秒

**メッセージ**: `Invalid second at {location}: {value} (expected 0-59)`

**原因**: 時刻リテラルの秒部分（`HH:MM:SS`の`SS`）が 0〜59 の範囲外です（ADR 0003 D4）。うるう秒（leap second）はサポートしていないため `60` は常に拒否されます。

**修正方法**: 秒は 0〜59 の範囲で指定してください。

```
# 誤り
event a 2024-01-01T10:00:60 "E" {};

# 正しい
event a 2024-01-01T10:00:59 "E" {};
```

---

### E008: 不正な UTC オフセット

**メッセージ**: `Invalid UTC offset at {location}: {value} (expected Z or -14:00 through +14:00)`

**原因**: 時刻リテラルのオフセット部分（`Z` または `±HH:MM`）が不正です（ADR 0003 D4）。具体的には以下のいずれかです：

- 許容範囲 `-14:00`〜`+14:00`（実在する UTC オフセットの範囲）を超えている（例: `+25:00`）
- 書式不正（例: `+09:3`, `+9:00`）

**修正方法**: `Z`（UTC）または `[+-]HH:MM` 形式（例: `+09:00`, `-05:00`, `+05:45`）で、`-14:00`〜`+14:00` の範囲内で指定してください。silent fallback（クランプや無視）は行わず、常にパースエラーとして拒否されます。

```
# 誤り（範囲外）
event a 2024-01-01T10:00+15:00 "E" {};

# 誤り（書式不正）
event a 2024-01-01T10:00+9:00 "E" {};

# 正しい
event a 2024-01-01T10:00+09:00 "E" {};
event a 2024-01-01T10:00Z "E" {};
```

---

## 意味エラー（tdsl-core: lowering）

AST→IR変換（lowering）フェーズで発生するエラーです。構文は正しくても意味的に矛盾がある場合に報告されます。

### E101: 未定義のlane参照

**メッセージ**: `Unknown lane reference: {id}`

**原因**: `span` / `event` / `event_range` で参照しているlane IDが、`lane` 宣言で定義されていません。

**修正方法**: lane宣言の `as` エイリアスと、アイテムのlane参照が一致しているか確認してください。

**検出タイミング**: Wikidata フェッチ（`import` ブロックの解決）より前の lowering Pass 2 で検出されます。`--offline` フラグ不要でネットワーク接触前にエラーが報告されます。

```
# 誤り（"dynasty" が未定義）
span dynasty -206..-9 "秦"

# 正しい
lane "王朝" as dynasty
span dynasty -206..-9 "秦"
```

---

### E102: laneエイリアスの重複

**メッセージ**: `Duplicate lane alias: {id}`

**原因**: 同じ `as` エイリアスを持つ `lane` 宣言が複数あります。

**修正方法**: 各laneに一意のエイリアスを付けてください。

---

### E103: アイテムIDの重複

**メッセージ**: `Duplicate item id: {id}`

**原因**: 同じ `id` を持つアイテムが複数定義されています。

**修正方法**: `id` はファイル内で一意にしてください。

```
# 誤り（id "qin" が重複）
span dynasty -206..-9 "秦" { id: qin }
span dynasty -206..-9 "秦（再掲）" { id: qin }

# 正しい
span dynasty -206..-9 "秦" { id: qin }
span dynasty -206..-9 "秦（再掲）" { id: qin_2 }
```

---

### E104: timelineブロックなし

**メッセージ**: `No timeline block found`

**原因**: ファイルに `timeline` ブロックがありません。

**修正方法**: ファイルの先頭に `timeline` ブロックを追加してください。

```tdsl
timeline {
  title: "私の年表"
  unit: year
  range: -500..2000
}
```

---

### E105: timeline ブロックの重複

**メッセージ**: `Multiple timeline blocks found`

**原因**: `timeline` ブロックが2つ以上あります。

**修正方法**: `timeline` ブロックはファイルに1つだけ記述してください。

---

### E106: 未解決のimport参照

**メッセージ**: `Unresolved import reference: {key}`

**原因**: `map` ブロック内で参照している `wd.key` が、対応する `import` ブロックで定義されていません。

**修正方法**: `import` ブロックのエイリアス名と `map` の参照名が一致しているか確認してください。

```
# 誤り（import のエイリアスが "emperors" なのに "emperor" を参照）
import Q7209 as emperors { ... }
map wd.emperor { ... }   # "emperor" は未定義

# 正しい
map wd.emperors { ... }
```

---

### E107: 未解決のエンティティキー

**メッセージ**: `Unresolved entity key: {key}`

**原因**: `map` ブロック内で参照しているエンティティキーが、Wikidataから取得した結果に存在しません。

**修正方法**: `tdsl fetch {QID}` でエンティティの内容を確認し、存在するプロパティを使用してください。

---

### E108: mapが参照するlaneが未定義

**メッセージ**: `Map references unknown lane: {id}`

**原因**: `map` ブロックの `lane` フィールドに指定したlane IDが定義されていません。

**修正方法**: E101と同様にlane宣言を追加するか、正しいlane IDを指定してください。

---

### E109: テンプレートエイリアスの重複

**メッセージ**: `Duplicate template alias: {id}`

**原因**: 同じエイリアスの `template` 宣言が複数あります。

---

### E110: 未定義のテンプレート参照

**メッセージ**: `Unknown template reference: {id}`

**原因**: `apply` で参照しているテンプレートが定義されていません。

---

### E111: 不正なアイテム link URL

**メッセージ**: `Invalid item link URL: {url} (expected http:// or https:// URL)`

**原因**: `link` オプションに `http://` / `https://` 以外の URL（例: `javascript:`、`data:`、相対 URL）が指定されています。

**修正方法**: 参照URLは絶対 URL で、スキームを `http://` または `https://` にしてください。

---

### E112: 不正なアイテム color 値

**メッセージ**: `Invalid item color value: {value}`

**原因**: `color` オプションに安全な色値として扱えない文字列が指定されています。

**修正方法**: hex 色（`#RGB`, `#RGBA`, `#RRGGBB`, `#RRGGBBAA`）または単純な CSS 色キーワードを指定してください。

---

### E113: UTCオフセット付き値となし値の比較

**メッセージ**: `Cannot compare a UTC-offset time value with a value that has no offset (author must make both sides consistent): {0} vs {1}`

**原因**: 同一の比較コンテキスト（例: 同一 `span`/`event_range` の `start..end`、同一 lane 内の並べ換え等）で、オフセット付きの時刻値（`DateTimeOffset` / `DateTimeSecondOffset`）とオフセットなしの時刻値（`Year`〜`DateTimeSecond`）を直接比較しようとしました（ADR 0003 D2）。

オフセットなしの値は「タイムゾーン不明」ではなく「タイムゾーンという概念を持たない裸の暦時刻」として扱われるため、暗黙に UTC とみなして正規化比較することはしません（AGENTS.md §4.1 no silent fallback）。Wikidata インポートは常にオフセットなし（`DateTime` / `DateTimeSecond`）で格納されるため、静的データにオフセットを付けた場合は Wikidata データと同一比較コンテキストで混在させるとこのエラーになりうます（意図した挙動）。

**修正方法**: 同一比較コンテキスト内の値は、全てにオフセットを付与するか、全てからオフセットを取り除くかで統一してください。Wikidata インポートと静的定義を同一 `span`/`event_range` で混在させたい場合は、静的側のオフセットを削除することでどちらもオフセットなしに揃えてください（docs/migration-second-precision.md の「Wikidataと静的offset付きデータの混在」節を参照）。

```
# 誤り（オフセット付きとなしが同一 span に混在）
span a 2024-01-01T10:00:00+09:00..2024-01-02T10:00 "S" {};

# 正しい（両方にオフセットを付与）
span a 2024-01-01T10:00:00+09:00..2024-01-02T10:00+09:00 "S" {};

# 正しい（両方からオフセットを除く）
span a 2024-01-01T10:00:00..2024-01-02T10:00 "S" {};
```

---

## バリデーション警告（tdsl-core: validate）

IR生成後の整合性チェックで発生する警告です。ビルドは続行されますが、出力が意図と異なる可能性があります。

### W201: アイテムが未定義laneを参照

**メッセージ**: `Item references unknown lane: {lane}`

**原因**: バリデーション段階でlaneが見つかりません（通常はloweringで検出されます）。

---

### W202: spanの開始が終了より後

**メッセージ**: `Span "{id}" has start ({start}) > end ({end})`

**原因**: `span` の開始年が終了年より大きい値になっています。

**修正方法**: `start..end` の順に記述してください。`tdsl lint --fix` で自動修正できます。

```
# 誤り
span dynasty 9..-206 "秦"

# 正しい
span dynasty -206..9 "秦"
```

---

### W203: timelineのrangeが不正

**メッセージ**: `Timeline range is invalid: {start}..{end}`

**原因**: `timeline` ブロックの `range` で開始が終了以上になっています。

**修正方法**: `range: start..end` の形式で `start < end` となるよう修正してください。

---

### W204: laneのkindが未知の値

**メッセージ**: `Lane "{id}" uses unknown kind: {kind} (known kinds: {known}; use custom for user-defined categories)`

**原因**: `lane` の `kind` が既知値（`custom` / `dynasty` / `person` / `country` / `event`）のいずれにも一致しません。`kind` は自由分類の意図を持つためエラーにはしません。

**修正方法**: タイプミスでなければ無視して構いません。独自分類であることを明示したい場合は `kind custom;` を使ってください。

---

### W205: Eventが timeline.range 外

**メッセージ**: `Event "{id}" at {time} is outside timeline.range and will not be rendered`

**原因**: `event` の時刻が `timeline.range` の外にあります。Renderer は範囲外の Event を描画しません（以前は警告なしで無言 drop されていました）。

**修正方法**: `timeline.range` を拡大するか、意図的に表示範囲を絞っている場合は警告を無視して構いません。

---

### W206: Span / EventRange が timeline.range に完全に含まれない

**メッセージ**: `{Span|EventRange} "{id}" is entirely outside timeline.range and will not be rendered`

**原因**: `span` / `event_range` の期間が `timeline.range` と一切重なっていません。

**修正方法**: `timeline.range` を拡大するか、アイテムの日付を見直してください。

---

### W207: Span / EventRange が timeline.range を一部はみ出し（clipped）

**メッセージ**: `{Span|EventRange} "{id}" is partially outside timeline.range and will be clipped`

**原因**: `span` / `event_range` の一部のみが `timeline.range` 外にはみ出しています。意図的な表示範囲の絞り込みであれば無視して構いません。

**修正方法**: 意図した上であれば対応不要。タイプミスであれば `timeline.range` またはアイテムの日付を修正してください。

---

## Lowering 警告（tdsl-core: map / apply）

`map` / `apply` で宣言した Wikidata エンティティが、必須フィールドを解決できず
アイテムを 1 件も生成しなかった場合の警告です。エラーではないためビルドは続行
しますが、「インポートしたのに何も出力されない」サイレントな取りこぼしを検知
するために報告されます（AGENTS.md §4.1「No silent fallback」）。`tdsl build` /
`tdsl check` が `Warning:` として stderr に出力します。

### W210: マッピング対象が必須フィールド未解決でアイテム未生成

**メッセージ**:

- `Mapped entity {id} produced no item: required`lane`is unresolved/empty`
- `Mapped entity {id} produced no item: required`label`could not be resolved`
- `Mapped entity {id} produced no`span`:`start`/`end`could not be resolved`
- `Mapped entity {id} produced no`event`:`time`could not be resolved`
- `Mapped entity {id} produced no`event_range`:`start`/`end`could not be resolved`

`expand` 使用時は `{id}` に `(プロパティ#インデックス)` が付与され、どの
statement が解決できなかったかを示します（例: `Q7209 (P39#2)`）。

**原因**: 指定した `claim(...)` がエンティティに存在しない、対象言語の `label`
が無い、`lane` プロパティが未指定、などにより必須値が `None` になっています。

**修正方法**: マッピング式（`claim(P...).year` 等）のプロパティ番号を確認し、
`??` でフォールバックを与えるか、`label@en` 等の取得言語を追加してください。
対象エンティティが本当にその情報を持たない場合は `map` 対象から除外します。

---

## Wikidataエラー（tdsl-wikidata）

Wikidata APIとの通信・データ解析で発生するエラーです。

### E301: HTTP通信エラー

**メッセージ**: `HTTP error: ...`

**原因**: Wikidata APIへのHTTPリクエストが失敗しました。ネットワーク障害・DNS解決失敗などが原因として考えられます。

**修正方法**: ネットワーク接続を確認してください。開発中は `--offline` フラグでWikidataアクセスをスキップできます。

```bash
tdsl build examples/china_with_import.tdsl --offline
```

---

### E302: 不正な入力

**メッセージ**: `Invalid input: {detail}`

**原因**: QIDやプロパティIDの形式が不正です。

**修正方法**: QIDは `Q123` 形式、プロパティIDは `P569` 形式で記述してください。

---

### E303: エンティティが見つからない

**メッセージ**: `Entity not found: {id}`

**原因**: 指定したQIDのエンティティがWikidataに存在しません。

**修正方法**: `tdsl fetch {QID}` またはWikidata（wikidata.org）でQIDを確認してください。

---

### E304: 時間値のパースエラー

**メッセージ**: `Failed to parse time value: {value}`

**原因**: WikidataのAPI応答に含まれる時間値を年数に変換できませんでした。

**修正方法**: `tdsl fetch {QID}` でそのエンティティのプロパティを確認し、時間値が存在するか確認してください。非常に古い年代（数万年前以前）は変換できない場合があります。

---

### E305: クレームが存在しない

**メッセージ**: `Missing claim {property} on entity {entity}`

**原因**: `map` ブロックで参照したプロパティ（`claim(P569).year` など）がエンティティに存在しません。

**修正方法**: `tdsl fetch {QID}` でエンティティの利用可能なプロパティを確認してください。

```bash
tdsl fetch Q7209 --lang ja
```

---

### E306: タイムアウト

**メッセージ**: `Wikidata API request timed out. Try running with the --offline flag.`

**原因**: Wikidata APIへのリクエストが時間内に完了しませんでした。

**修正方法**: しばらく待ってから再実行してください。開発中は `--offline` フラグを使用してください。

---

### E307: レート制限

**メッセージ**: `Wikidata API rate limit exceeded (HTTP 429). Please wait a moment and retry.`

**原因**: 短時間に大量のリクエストを送信したため、Wikidata APIにレート制限されました。

**修正方法**: 数分待ってから再実行してください。多数のエンティティをインポートする場合は、`--offline` でまず静的アイテムを確認し、最終確認時のみオンラインビルドをすることを推奨します。

---

## Lintコード（tdsl lint）

`tdsl lint` が検出する品質上の問題です。`--fix` で自動修正できるものには ✅ マークがあります。

### ERROR: unknown_lane — 未定義のlane参照

**メッセージ**: `unknown lane reference '{id}'`

**原因**: アイテムが存在しないlane IDを参照しています。

**修正方法**: E101と同様の対処をしてください。

---

### ERROR: empty_label — 空ラベル

**メッセージ**: `label must not be empty`

**原因**: アイテムのラベルが空文字列です。

**修正方法**: アイテムに意味のあるラベルを付けてください。

---

### ERROR: invalid_tags ✅ — 不正なタグ

**メッセージ**: `tags contain empty elements` / `tags contain duplicated elements` / `tags contain empty and duplicated elements`

**原因**: タグリストに空文字列または重複したタグが含まれています。

**修正方法**: `tdsl lint --fix` で自動修正されます。手動修正する場合は空タグ・重複タグを削除してください。

---

### ERROR: duplicate_id — IDの重複

**メッセージ**: `id '{id}' duplicates line {line}`

**原因**: 同じIDが複数のアイテムに使われています。

**修正方法**: E103と同様にIDをユニークにしてください。

---

### ERROR: start_gt_end ✅ — 開始・終了が逆転

**メッセージ**: `span range is reversed: {start}..{end}` / `event_range is reversed: {start}..{end}`

**原因**: 開始年と終了年が逆になっています。

**修正方法**: `tdsl lint --fix` で自動修正されます。

---

### WARN: missing_id ✅ — IDなし

**メッセージ**: `id is missing`

**原因**: アイテムに `id` プロパティが設定されていません。IDがないとWikidata連携（map ブロック）やプログラム的な参照ができません。

**修正方法**: `tdsl lint --fix` でランダムIDが自動生成されます。意味のあるIDを付けたい場合は手動で設定してください。

```tdsl
span dynasty -206..-9 "秦" {
  id: qin
}
```

---

### WARN: invalid_calendar_date — 無効なカレンダー日付

**メッセージ**: `Invalid calendar date: YYYY-MM-DD`

**原因**: `YYYY-MM-DD` 形式の日付が実在しません。典型的なケースとして以下があります。

- 2月30日・2月31日（2月は28日または29日まで）
- 4月・6月・9月・11月の31日（これらの月は30日まで）
- 閏年でない年の2月29日（例: `1900-02-29`、`2021-02-29`）

**修正方法**: 正しいカレンダー日付に修正してください。閏年は「4で割り切れる かつ（100で割り切れない または 400で割り切れる）」年です。`2000-02-29` は有効、`1900-02-29` は無効です。

```tdsl
# 誤り（2月は最大29日まで。2024年は閏年だが30日は存在しない）
event events 2024-02-30 "存在しない日付"

# 正しい
event events 2024-02-29 "2024年は閏年"
event events 2024-03-01 "3月1日"
```

**備考**: パーサは日付の値域（月は 1〜12、日は 1〜31）のみを検証します。カレンダー上の実在確認（うるう年判定・月末日確認）は lint の責務です。月精度のみの指定（例: `2024-02`）は検証対象外です。

---

## 関連ドキュメント

- [DSL仕様書](./dsl-spec.md) — 文法の詳細
- [チュートリアル](./tutorial.md) — 基本的な使い方
- [Wikidataプロパティ一覧](./dsl-spec.md#wikidata連携) — よく使うプロパティ
