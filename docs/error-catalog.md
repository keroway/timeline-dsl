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

**原因**: 同じエイリアスの `template` 宣言が複数あります（将来機能、現在未実装）。

---

### E110: 未定義のテンプレート参照

**メッセージ**: `Unknown template reference: {id}`

**原因**: `apply` で参照しているテンプレートが定義されていません（将来機能、現在未実装）。

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

## Wikidataエラー（tdsl-wikidata）

Wikidata APIとの通信・データ解析で発生するエラーです。

### E301: HTTP通信エラー

**メッセージ**: `HTTP error: ...`

**原因**: Wikidata APIへのHTTPリクエストが失敗しました。ネットワーク障害・DNS解決失敗などが原因として考えられます。

**修正方法**: ネットワーク接続を確認してください。開発中は `--offline` フラグでWikidataアクセスをスキップできます。

```bash
tdsl build examples/my.tdsl --offline
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
