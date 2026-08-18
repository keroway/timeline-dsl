# Timeline DSL 言語仕様

## 概要

Timeline DSL（`.tdsl`）は年表データを宣言的に記述するためのドメイン固有言語。C風の波括弧+セミコロン構文を採用し、可読性とGit差分管理のしやすさを重視している。

> 月・日精度の時間表現（`YYYY-MM` / `YYYY-MM-DD` 形式）の詳細設計は [spec-date-precision.md](spec-date-precision.md) を参照。

## 文法（EBNF）

```ebnf
<document>     ::= { <statement> }

<statement>    ::= <timeline>
                 | <lane>
                 | <group>
                 | <span>
                 | <event>
                 | <event_range>
                 | <import_block>
                 | <map_block>
                 | <template_block>
                 | <apply_block>

<timeline>     ::= "timeline" <string> "{" { <timeline_setting> } "}"
<timeline_setting>
               ::= "title" <string> ";"
                 | "unit" <identifier> ";"
                 | "range" <time_value> ".." <time_value> ";"
                 | "calendar" <identifier> ";"
                 | "color_map" "{" { <color_map_key> ":" <string> ";" } "}"
<color_map_key> ::= <string> | <identifier>

<lane>         ::= "lane" <string> ["as" <identifier>] "{" { <lane_prop> } "}"
<lane_prop>    ::= "kind" <identifier> ";"
                 | "order" <number> ";"
                 | "color" <string> ";"

<group>        ::= "group" <string> "{" <lane> { <lane> } "}"

<span>         ::= "span" <identifier> <time_value> ".." <open_ended_time_value> <string>
                   <block_options> ";"
<event>        ::= "event" <identifier> <time_value> <string>
                   <block_options> ";"
<event_range>  ::= "event_range" <identifier> <time_value> ".." <open_ended_time_value> <string>
                   <block_options> ";"
<open_ended_time_value> ::= "now" | <time_value>

<block_options> ::= "{" { <option> } "}"
<option>       ::= "tags" "[" <string_list> "]" ";"
                 | "source" <source_ref> ";"
                 | "origin" <identifier> ";"
                 | "id" <string> ";"
                 | "note" <string> ";"
                 | "link" <string> ";"
                 | "color" <string> ";"

<import_block> ::= "import" <source_name> ["as" <identifier>]
                   "{" { <import_stmt> } "}"
<import_stmt>  ::= "entity" <qid> ["as" <identifier>] ";"
                 | "query" <string> ["as" <identifier>] ";"
                 | "policy" <policy_name> ";"
                 | "policy" "field_priority" "{" { <field_strategy> } "}"
<field_strategy> ::= ("label" | "time" | "tags") ":" ("manual" | "wikidata" | "merge") ";"

<map_block>    ::= "map" <import_ref> "to" <mapping_target>
                   "{" { <mapping_rule> } "}"
<mapping_target> ::= "span" | "event" | "event_range"
<mapping_rule> ::= "lane" <identifier> ";"
                 | "start" <map_expr> ";"
                 | "end" <map_expr> ";"
                 | "time" <map_expr> ";"
                 | "label" <lang_expr> ";"
                 | "tags" "[" <string_list> "]" ";"
                 | "filter" <filter_expr> ";"
                 | "expand" "claim(" <property_id> ")" ";"
<filter_expr>  ::= <filter_or>
<filter_or>    ::= <filter_and> { "||" <filter_and> }
<filter_and>   ::= <filter_not> { "&&" <filter_not> }
<filter_not>   ::= ["!"] <filter_atom>
<filter_atom>  ::= "(" <filter_expr> ")"
                 | <label_ref> <string_match_op> <string>
                 | <filter_operand> <compare_op> <filter_operand>
<string_match_op> ::= "contains" | "startswith"
<compare_op>   ::= ">=" | "<=" | "==" | "!=" | ">" | "<"
<filter_operand> ::= "null" | <claim_expr> | <number>

<template_block> ::= "template" <string> ["as" <identifier>]
                   "to" <mapping_target> "{" { <mapping_rule> } "}"

<apply_block>  ::= "apply" <identifier> "to" <identifier>
                   "{" { <apply_override> } "}"
<apply_override> ::= "lane" <identifier> ";"

<claim_expr>   ::= "claim(" <property_id> ")" ["." "qualifier(" <property_id> ")"] ["." <accessor>] [<claim_offset>]
<accessor>     ::= "year" | "month" | "day" | "hour" | "minute" | "second"
<claim_offset> ::= ("+" | "-") <digit>+   ; 年シフト。i32 の範囲内
<map_expr>     ::= (<claim_expr> | <number>) { "??" (<claim_expr> | <number>) }
<lang_expr>    ::= <label_ref> { "??" <label_ref> }
<label_ref>    ::= "label@" <lang_code>

<source_ref>   ::= <identifier> ":" <qid>
<string_list>  ::= <string> { "," <string> }
<qid>          ::= "Q" <digits>
<property_id>  ::= "P" <digits>
<identifier>   ::= /[A-Za-z_][A-Za-z0-9_-]*/
<number>       ::= /"-"? [0-9]+/
// `now` は span / event_range の `end` 位置専用（#550。ビルド時点の現在年に解決され、アイテムは継続中としてマークされる）
<time_value>   ::= <date_time> | <date> | <year_month> | <year>
<year>         ::= /"-"? [0-9]+/
<year_month>   ::= /"-"? [0-9]{1,4} "-" [0-9]{2}/
<date>         ::= /"-"? [0-9]{1,4} "-" [0-9]{2} "-" [0-9]{2}/
// 秒・UTCオフセットは ADR 0003 D4 で追加（#612）。<second> と <tz_offset> は
// いずれも省略可能で独立に組み合わせられる（例: 秒のみ、offsetのみ、両方、両方省略）。
<date_time>    ::= /"-"? [0-9]{1,4} "-" [0-9]{2} "-" [0-9]{2} "T" [0-9]{2} ":" [0-9]{2}/ [<second>] [<tz_offset>]
<second>       ::= /":" [0-9]{2}/
<tz_offset>    ::= "Z" | /("+" | "-") [0-9]{2} ":" [0-9]{2}/
```

## 構文要素の詳細

### timeline

年表全体のメタ情報を定義するブロック。ファイルにつき1つ。

```
timeline "中国王朝年表" {
    title "中国王朝年表";
    unit year;
    range -500..2000;
    calendar proleptic_gregorian;
    color_map {
        dynasty: "#3366cc";
        war:     "#cc0000";
    }
}
```

| プロパティ | 必須 | 説明 |
|---|---|---|
| `title` | 任意 | 年表の表示タイトル |
| `unit` | 任意 | 時間単位（`year`, `month`, `day`, `hour`, `minute`, `second`）。未知の値は lowering エラー |
| `range` | 任意 | 表示範囲。`開始..終了` の形式。負の値は紀元前 |
| `calendar` | 任意 | 暦法。`proleptic_gregorian` 等 |
| `color_map` | 任意 | タグ→色のマッピング。`タグ名: "#16進数カラーコード";` の形式で複数定義可能 |

`unit hour` / `unit minute`（#556）は `YYYY-MM-DDTHH:MM` のような時分精度のタイムライン向け。Renderer は `timeline.range` の月日・時分精度に基づき、過密にならないよう 1h→3h→6h→12h、または 1min→5min→15min→30min のように目盛りを間引く。単日範囲では `HH:MM`、複数日範囲では `MM-DD HH:MM` 形式の軸ラベルを使う。

`unit second`（#614、ADR 0003）は `YYYY-MM-DDTHH:MM:SS` のような秒精度のタイムライン向け。`hour`/`minute` と同様に 1s→5s→15s→30s の間引きを行い、単日範囲では `HH:MM:SS`、複数日範囲では `MM-DD HH:MM:SS` 形式の軸ラベルを使う。秒・UTCオフセット（`Z` / `±HH:MM`）構文の詳細は ADR 0003 を参照。offset の有無が混在する値同士の比較は明示的エラーになる（silent fallback しない）。

`color_map` で定義した色は `tdsl render` 時に自動適用される。`color_map` は hex 色（`#RGB`, `#RGBA`, `#RRGGBB`, `#RRGGBBAA`）と単純な CSS 色キーワードを受け付ける。複雑な CSS 値は安全のため renderer が無視する。高度な装飾は CLI の `--custom-css` を使う。`--color-map "war=#cc0000"` CLIフラグで上書きも可能。

`color_map` のキーは `ident`（ASCII）または文字列リテラル（任意 Unicode）で指定できる。`tags ["戦争"]` のような非 ASCII タグに色を割り当てる場合は引用符で囲む（#551）。

```
color_map {
    war: "#cc0000";       // バレアイド(ident)キー
    "戦争": "#cc0000";     // 文字列リテラルキー（非 ASCII 可）
}
```

### lane

年表の縦軸カテゴリ（レーン）を定義する。王朝・人物・国・組織などを表す。

```
lane "漢" as han { kind dynasty; order 20; }
```

| プロパティ | 必須 | 説明 |
|---|---|---|
| `as <id>` | 任意 | 内部識別子。省略時はラベルからスラッグを自動生成 |
| `kind` | 任意 | 分類（既知値: `custom`, `dynasty`, `person`, `country`, `event`）。未知の値は検証警告（独自分類は `custom` 推奨） |
| `order` | 任意 | 初期表示順（整数） |
| `color` | 任意 | lane の色を固定する（`"#4a9eff"` / `"rebeccapurple"` 等）。**省略時は lane の並び順からパレットを機械的に割り当てるため、lane を 1 つ足したり `order` を変えると既存 lane の色がずれる。** 色を固定したい場合に指定する。値の検証は item の `color` と同じで、不正値はエラー（パレットへフォールバックしない） |

色の解決優先順位は `item.color` > `timeline.color_map`（タグ経由） > `lane.color` > パレット。

```tdsl
lane "漢" as han { kind dynasty; order 20; color "#c0392b"; }
```


### group

複数の lane をまとめて視覚的に階層化するグループを定義する。レンダリング時にグループラベルとグループ境界線が表示される。

```
group "古代" {
    lane "秦" as qin { kind dynasty; order 10; }
    lane "漢" as han { kind dynasty; order 20; }
}
```

- グループ内には 1 つ以上の `lane` 宣言を記述する
- グループ内の lane は IR 上で `group`（グループ名）を持つ。`group` を使わない lane では省略される
- `group` を使わない既存の `.tdsl` はそのまま動作する（後方互換）

### span

存続期間を表す。王朝の在位期間や人物の生没年など。

```
span han -206..220 "漢" { tags ["dynasty"]; source wd:Q7209; id "span:han"; };

// 月・日精度の例
span ww2 1939-09-01..1945-09-02 "第二次世界大戦" { tags ["war"]; };

// 継続中（オープンエンド）の例（#550）
span reiwa 2019..now "令和" { tags ["era"]; };
```

- 第1引数: レーンID
- 第2引数: `開始..終了`（時刻値の範囲。`1939-09-01..1945-09-02` のように月・日精度も指定可）。`終了` に `now` を指定すると「現在も継続中」の意味になる（下記参照）
- 第3引数: ラベル（文字列）

#### 継続中（open-ended）の表現（#550）

`span` / `event_range` の `end` に `now` キーワードを指定すると、その期間が現在も継続中であることを表す。

```
span reiwa 2019..now "令和" {};
```

- `now` はビルド時点の現在年（UTC）に解決され、IR の `end` にはその具体値が入る（既存ツールとの後方互換のため）。同時に IR の `end_open: true` フラグで「継続中」の意味情報を保持する
- Renderer は `end_open` のアイテムに `tdsl-item-open-ended` CSS クラスを付与（デフォルトで破線の囲みを適用、`--custom-css` で上書き可能）し、ツールチップには終了日の代わりに「進行中」と表示する
- `tdsl decompile` は `end_open == true` の場合に `now` を再出力する（往復安定）
- **スコープ外**: `map` ブロック（Wikidata インポート）からの `now` フォールバック（例: `end claim(P582).year ?? now;`）は本対応のスコープ外。`now` は `span` / `event_range` の直接定義の `end` 位置のみで使える

### event

特定の時点に起きたイベント。

```
event han -209 "陳勝・呉広の乱" {};
```

- 第1引数: レーンID
- 第2引数: 時点（時刻値。`1969-07-20` のように月・日精度も指定可）
- 第3引数: ラベル（文字列）

### event_range

一定期間のイベント。戦争・災害・プロジェクトなど。

```
event_range han 184..204 "黄巾の乱" { tags ["war"]; };

// 継続中（オープンエンド）の例（#550）
event_range ongoing_conflict 2020..now "進行中の紛争" {};
```

- 第1引数: レーンID
- 第2引数: `開始..終了`（時刻値の範囲。`end` に `now` を指定すると継続中を表す（span と同様。上記参照）
- 第3引数: ラベル（文字列）

### block_options（共通オプション）

span / event / event_range に付与できるオプション。

| オプション | 説明 | 例 |
|---|---|---|
| `tags` | タグのリスト | `tags ["war", "major"];` |
| `source` | データソース（Wikidata等） | `source wd:Q7209;` |
| `id` | 要素の安定識別子 | `id "span:han";` |
| `origin` | 由来の識別子 | `origin imported;` |
| `note` | アイテム説明文。ツールチップと SVG の `data-note` 属性、インタラクティブ HTML の詳細パネル「メモ」行に出る | `note "出典メモ";` |
| `link` | 参照URL。lowering時に `http://` / `https://` のみ許可。SVG の `data-link` 属性と、インタラクティブ HTML の詳細パネル「参照リンク」行（クリック可能なアンカー）に出る | `link "https://example.com";` |

> `link` を持つ item の SVG グループは **`<a>` で包まない**。グループは既に `role="group"` と `tabindex="0"` を持つため、包むとフォーカス可能な要素が入れ子になり Tab が二重に止まる。PNG / PDF は `usvg` を通るためアンカーは描画結果に現れない。リンクを押せる形で提供するのはインタラクティブ HTML（`--interactive`）の役割で、SVG は `data-link` を載せるところまで（埋め込みページの JS からは読める）。
| `color` | アイテム個別色。`color_map` や lane 色より優先 | `color "#3366cc";` |

### import

外部データソースからの取り込みを宣言する。

```
import wikidata as wd {
    entity Q7183 as qin_dynasty;
    entity Q7209 as han_dynasty;
    query "SELECT ?item WHERE { ... }" as samurai;
    policy merge_by_source;
    policy field_priority {
        label: manual;    // ラベルは手動定義を優先
        time:  wikidata;  // 時刻はWikidataを優先
        tags:  merge;     // タグは両方をマージ
    }
}
```

| 要素 | 説明 |
|---|---|
| `entity <QID>` | 特定のWikidataエンティティを指定 |
| `query <SPARQL>` | SPARQLクエリで複数エンティティを取得 |
| `policy <name>` | 再インポート時のマージ戦略 |
| `policy field_priority { ... }` | フィールド単位のマージ戦略 |
| `as <alias>` | インポートブロック/エンティティの別名 |

#### 再インポートポリシー

| ポリシー | 動作 |
|---|---|
| `merge_by_source` | ID衝突をエラーとして扱う（デフォルト） |
| `overwrite_imported` | 既存のインポート済み項目のみ上書き。手動定義との衝突はエラー |
| `keep_manual` | ID衝突時はインポート側をスキップして既存項目を保持 |

#### フィールド優先度ポリシー（field_priority）

`merge_by_source` 等の全体ポリシーよりも細かく、フィールドごとにマージ戦略を指定できる。

| フィールド | 値 | 動作 |
|---|---|---|
| `label` / `time` / `tags` | `manual` | 既存の手動定義を優先（Wikidata側を無視） |
| `label` / `time` / `tags` | `wikidata` | Wikidata側を優先（手動定義を上書き） |
| `label` / `time` / `tags` | `merge` | 両方を保持（`tags` では和集合、`label`/`time`はWikidata側を採用） |

**フィールド単位のマージは、同じ種別のアイテムどうしでのみ定義される。**
ID が衝突したアイテムの種別が食い違う場合（例: 手書きの `event` と Wikidata の `span`）は
エラー `E114` になり、既存アイテムは置換されない。

以前は取り込み側が黙って既存を丸ごと置き換えており、`label manual` の指定も無視されて
「手動データを守る」という field_priority の意図と逆の結果になっていた
（[error-catalog の E114](./error-catalog.md#e114-field_priority-でのアイテム種別不一致) を参照）。

### map

インポートしたエンティティを年表要素に変換するルール。

```
map wd.han_dynasty to span {
    lane han;
    start claim(P571).year;
    end claim(P576).year;
    label label@ja ?? label@en;
    tags ["dynasty", "imported"];
}
```

`map <alias> to <target_type> { ... }` の `<target_type>` には **`span` / `event` / `event_range` のいずれか**のみを指定できます。これ以外の値（例: `timeline`・`item`）を書くとパースエラー `Unknown map target type '<値>' (expected one of: span, event, event_range)` になります（[error-catalog の E004](./error-catalog.md#e004-不明な-map-ターゲット型) を参照）。

| target_type | 生成されるアイテム種別 | 必須の時間プロパティ |
|---|---|---|
| `span` | 期間（開始〜終了） | `start` / `end` |
| `event` | 点イベント | `time` |
| `event_range` | 範囲イベント | `start` / `end` |

> `source` はインポートされたアイテムに `wd:<entity_id>` として自動付与されます。`map` ブロック内での明示指定は廃止されています。

| プロパティ | 説明 |
|---|---|
| `lane` | 対象レーンのID |
| `start` | 開始時点を計算する式 |
| `end` | 終了時点を計算する式 |
| `time` | 点イベントの時点を計算する式（event用） |
| `label` | ラベルを計算する式 |
| `tags` | タグのリスト |
| `filter` | エンティティを絞り込む条件式（複数記述すると全て AND で評価） |

#### filter 式

`filter` ルールを使ってエンティティを絞り込める。複数書くとすべて AND として評価される。

**数値比較**（`>=`, `<=`, `==`, `!=`, `>`, `<`）:

```
filter claim(P580).year > 1000;
filter claim(P576).year != null;
```

**文字列マッチング**（`contains` / `startswith`）:

```
filter label@ja contains "王朝";          // ラベルに "王朝" を含むエンティティのみ
filter label@en startswith "Han";         // ラベルが "Han" で始まるエンティティのみ
filter !(label@ja contains "候補");       // "候補" を含まないエンティティのみ
```

`label@<lang>` には任意の言語コードを指定できる。指定した言語のラベルが存在しないエンティティは `false`（除外）として扱われる（silent fallback しない）。

**論理演算**（`&&`, `||`, `!`, 括弧）:

```
filter label@ja contains "王朝" && claim(P580).year > 0;
filter claim(P580).year > 500 || claim(P571).year > 500;
```

### template / apply

`template` でマッピングパターンを再利用可能な形で定義し、`apply` で複数の import に適用する。

```
// テンプレート定義
template "王朝スパン" as dynasty_span
    to span {
        start claim(P571).year;
        end claim(P576).year;
        label label@ja ?? label@en;
    }

// テンプレートを適用（lane のみ apply 側で上書き可能）
apply dynasty_span to dynasties {
    lane dynasty;
}
```

| 要素 | 説明 |
|---|---|
| `template <名前> [as <id>] to <target_type> { ... }` | マッピングルールを定義（`map` と同じプロパティを使用） |
| `apply <template_id> to <import_id> { ... }` | 定義済みテンプレートを指定の import に適用 |
| `lane <id>;`（apply 内） | テンプレートの `lane` を apply 側で上書き |

テンプレートに使えるプロパティは `map` ブロックと同じ（`lane`, `start`, `end`, `time`, `label`, `tags`, `filter`）。`apply` 内では `lane` の上書きのみ可能。

完全なサンプル: [`examples/template_apply_example.tdsl`](../examples/template_apply_example.tdsl)

### 式（Expression）

#### claim 式

Wikidataのプロパティ値を取得する。

```
claim(P571).year    // P571 (inception) の時刻値を年に変換
claim(P569).year    // P569 (date of birth) の時刻値を年に変換
```

`??` 演算子で claim チェーンまたはリテラルへのフォールバックを記述できる。
左辺が解決できない場合にのみ右辺を評価する（短絡評価）。

```
// claim フォールバック: P580 がなければ P571 を使用
start claim(P580).year ?? claim(P571).year;

// リテラルフォールバック: P570 がなければ 9999 を使用
end claim(P570).year ?? 9999;

// チェーン + リテラル: P580 → P571 → 0 の順に試みる
time claim(P580).year ?? claim(P571).year ?? 0;
```

#### qualifier アクセス

Statement の qualifier（修飾子）プロパティにアクセスする。

```
claim(P39).qualifier(P580).year   // P39 ステートメントの qualifier P580（開始時点）の年
claim(P39).qualifier(P582).year   // P39 ステートメントの qualifier P582（終了時点）の年
```

qualifier が存在しない場合は値なし（silent fallback しない）。

#### expand — 複数 statement から複数アイテムを生成

`expand claim(P)` を map ブロック内に記述すると、そのエンティティのプロパティ P の
non-deprecated な Statement を全件ループし、各 Statement につき 1 アイテムを生成する。
`expand` がない場合は従来どおり最初の Statement のみを参照する。

```tdsl
import wikidata as w {
    entity Q9682 as elizabeth_ii;  // 例: エリザベス2世
}

// 在任した役職（P39）をすべてスパンとして展開する例
map w.elizabeth_ii to span {
    lane offices;
    expand claim(P39);
    start claim(P39).qualifier(P580).year;
    end   claim(P39).qualifier(P582).year ?? 9999;
    label label@ja;
}
```

P39 Statement が複数あれば複数のスパンが生成される。
qualifier（P580/P582）が存在しないステートメントはそのアイテムをスキップする（`start`/`end` が解決できないため）。

#### label 式

Wikidataエンティティのラベルを言語フォールバック付きで取得する。

```
label@ja             // 日本語ラベル
label@ja ?? label@en // 日本語がなければ英語にフォールバック
```

## コメント

行コメントとブロックコメントをサポート。

```
// これは行コメント

/* これは
   ブロックコメント */
```

> **コメントの扱い**: コメントはパース時に AST（`File.comments`）に byte span 付きで保持されます。
>
> - **`tdsl fmt`**: トップレベルのコメント（文の前後・同一行末尾）は位置を保ったまま保持されます。ブロック内部のコメントは内容を失わずにブロック境界（直前文の末尾または次文の直前）に移動します。整形は冪等（idempotent）です。
> - **lowering / IR**: コメントは IR には一切持ち込まれず、IR はコメントの有無で不変です（IR を単一の真実とする設計）。
> - **`tdsl decompile`**: IR 起点の逆変換のため、コメントは復元できません。

## Wikidataプロパティリファレンス

### 人物

| プロパティ | 意味 | 用途 |
|---|---|---|
| P569 | date of birth | 誕生年。`claim(P569).year` |
| P570 | date of death | 死亡年。`claim(P570).year` |
| P39 | position held | 役職。P580/P582で期間を取得 |

### 組織・国・王朝

| プロパティ | 意味 | 用途 |
|---|---|---|
| P571 | inception | 成立年。`claim(P571).year` |
| P576 | dissolved/abolished | 消滅年。`claim(P576).year` |

### 時間

| プロパティ | 意味 | 用途 |
|---|---|---|
| P580 | start time | 開始時点 |
| P582 | end time | 終了時点 |
| P585 | point in time | 特定時点の出来事 |

## 時間の表現

| 形式 | 例 | 精度 |
|---|---|---|
| `YYYY` | `1969`, `-206` | 年 |
| `YYYY-MM` | `1969-07` | 月 |
| `YYYY-MM-DD` | `1969-07-20` | 日 |
| `YYYY-MM-DDTHH:MM` | `1969-07-20T20:17` | 分 |
| `YYYY-MM-DDTHH:MM:SS` | `1969-07-20T20:17:40` | 秒（#612〜#616、ADR 0003） |
| `YYYY-MM-DDTHH:MM[:SS]Z` | `1969-07-20T20:17:40Z` | 分/秒 + UTCオフセット（#612〜#616、ADR 0003） |
| `YYYY-MM-DDTHH:MM[:SS]±HH:MM` | `1969-07-20T20:17:40+09:00` | 分/秒 + タイムゾーンオフセット（#612〜#616、ADR 0003） |

- 範囲: `開始..終了`（例: `-0206-01-15..-0206-02-20`, `1939-09-01..1945-09-02`, `1969-07-20T20:17..1969-07-20T21:00`, `1969-07-20T20:17:00Z..1969-07-20T20:18:40Z`）
- 紀元前（負の年）の月日・時分精度は、符号付き4桁年（例: `-0206-01-15`）で表す
- Wikidataの時刻値は `.year` / `.month` / `.day` / `.hour` / `.minute` / `.second` accessor で各精度の値を取得可能。たとえば `claim(P585).second` は Wikidata precision 14 の値だけを解決し、秒まで保持した時刻を返す。要求した精度を元データが持たない場合は未解決となり、`??` の次候補を評価する。必須フィールドが最後まで未解決なら、そのアイテムは生成されず warning が報告される（silent fallback しない）

### 秒・UTCオフセットの詳細仕様（ADR 0003）

- **秒**: `HH:MM` の後ろに `:SS`（0〜59）を任意で追加できる。うるう秒（leap second, `60`）はサポートしない（常にパースエラー）
- **オフセット**: `Z`（UTC）または `[+-]HH:MM`（例: `+09:00`, `-05:00`, `+05:45` のような15分単位も許容）を、秒の有無にかかわらず `HH:MM` の後ろに任意で追加できる。許容範囲は `-14:00`〜`+14:00`（実在するUTCオフセットの範囲）。範囲外・書式不正はパースエラー（[error-catalog の E007/E008](./error-catalog.md#e007-不正な秒)）
- **比較セマンティクス（D2）**: offset付き値同士はUTCに正規化してより比較し、offsetなし値同士は暦時刻の値そのままで比較する。**offset付き値とoffsetなし値を同一の比較コンテキスト（同一 `span`/`event_range` の `start..end` 等）で混在させると、`MixedOffsetComparison` エラーになる**（silent fallbackしない。[error-catalog の E113](./error-catalog.md#e113-utcオフセット付き値となし値の比較) 参照）
- **Wikidataインポート**: 常にoffsetなし（`DateTime` / `DateTimeSecond`）として格納される。静的データにoffsetを付けた場合、Wikidataデータと同一比較コンテキストで混在させると上記エラーになりうる（意図した挙動。移行手順は [docs/migration-second-precision.md](./migration-second-precision.md) を参照）
- **既存 minute-level ファイルとの後方互換**: 秒・offsetはいずれも既存構文（`YYYY-MM-DDTHH:MM`）の後ろに任意で追加されるオプション部なので、既存の minute-level（秒・offsetなし）`.tdsl` ファイルは一切変更なく引き続きパース・buildできる（非破壊変更）。移行の必要はない
- 使用例: [`examples/iss_docking_second_precision.tdsl`](../examples/iss_docking_second_precision.tdsl)（秒精度 + UTC `Z`）、[`examples/global_conference_timezones.tdsl`](../examples/global_conference_timezones.tdsl)（複数タイムゾーンオフセット）

## CLI

### サブコマンド一覧

全サブコマンドの正準な一覧・詳細は [`docs/cli-spec.md`](cli-spec.md#サブコマンド一覧) を参照。以下は代表的なものの抜粋。

| コマンド | 目的 |
|---|---|
| `tdsl build <file>` | `.tdsl` をJSON IRに変換 |
| `tdsl check <file>` | 構文・意味チェック |
| `tdsl ast <file>` | ASTダンプ |
| `tdsl render <file>` | HTML / SVG / PDF / PNG を生成（`--format html\|svg\|pdf\|png`、`--interactive`） |
| `tdsl decompile <json>` | JSON IRを `.tdsl` ソースに逆変換 |
| `tdsl merge <files...>` | 複数 `.tdsl` を統合してIR JSONを出力 |
| `tdsl fetch <QID>` | Wikidataエンティティ確認 |
| `tdsl search <query>` | Wikidata候補検索 |
| `tdsl inspect <QID>` | 年表化適性の診断 |
| `tdsl resolve <wikipedia-url>` | Wikipedia URL から QID を解決 |
| `tdsl scaffold wikidata ...` | QID群から `.tdsl` 雛形生成 |
| `tdsl init ...` | 手作業向け `.tdsl` テンプレ生成 |
| `tdsl import-csv <csv>` | CSVから `span/event/event_range` 生成 |
| `tdsl export-csv <json>` | IRをCSVに書き出す（`import-csv` と対称） |
| `tdsl fmt <file>` | `.tdsl` ファイルを正準フォーマット |
| `tdsl lint <file> [--fix]` | 品質チェックと安全な自動補正 |
| `tdsl cache status` | ローカルキャッシュの状態を表示 |
| `tdsl cache clear [--older-than <days>]` | キャッシュエントリを削除 |
| `tdsl completions <shell>` | シェル補完スクリプトを生成 |
| `tdsl lsp` | LSP サーバを stdio 経由で起動 |

### 最短フロー（Wikidata起点）

```bash
tdsl search "漢王朝" --lang ja -n 5
tdsl resolve "https://ja.wikipedia.org/wiki/漢"
tdsl inspect Q7209 --lang ja,en
tdsl scaffold wikidata --qids Q7183,Q7209 --timeline "中国王朝(生成)" --lang ja,en --target auto --lane-mode per-entity --output /tmp/china_scaffold.tdsl
tdsl render /tmp/china_scaffold.tdsl --output /tmp/china_scaffold.html
```

> `search / inspect / resolve / scaffold wikidata` はネットワークが必要。

### 最短フロー（手作業起点）

```bash
tdsl init --output /tmp/manual.tdsl --timeline "架空世界年表" --range-start 1000 --range-end 1300 --lanes "王国:kingdom,事件:incidents"
tdsl import-csv examples/fictional_empire_items.csv --append /tmp/manual.tdsl
tdsl lint /tmp/manual.tdsl --fix
tdsl render /tmp/manual.tdsl --output /tmp/manual.html
```

### `tdsl render`

`tdsl render` はタイムラインを HTML / SVG / PDF / PNG で出力する。

```bash
tdsl render input.tdsl --output timeline.html [--format html|svg|pdf|png] [--interactive] [--scale N] [--offline]
```

| オプション | 説明 |
|---|---|
| `--output` | 出力パス。省略時は標準出力 |
| `--format` | 出力形式。`html`（デフォルト）/ `svg` / `pdf` / `png` |
| `--interactive` | ズーム・パン・検索・凡例・詳細パネル付きインタラクティブモード（JavaScript使用）。`--format html` のみ有効 |
| `--show-legend` | レーン色とタグ色（`color_map`）の静的凡例パネルを表示 |
| `--scale` | 1年あたりのピクセル幅（デフォルト 2） |
| `--lane-height` | 各レーンの高さ（px、デフォルト 60）。縦密度を制御し、バーの太さも追従する |
| `--layout-style` | 高レベルな視覚レイアウト。`timeline`（デフォルト）/ `group-bands`（連続する lane group の背景帯） |
| `--dpi` | PNG 出力の DPI（デフォルト 96）。`--format png` のみ有効 |
| `--offline` | Wikidata fetch を省略 |
| `--pdf-pagination` | `--show-table` のアイテムテーブルを用紙サイズ・余白に収まる行数ごとに複数ページへ分割する（ADR-0004）。デフォルトは無効（既存の単一ページ縮小描画のまま）。`--show-table` なしで指定するとエラー。`--format pdf` のみ有効。`--chart-pagination` 併用時のページ構成は下記参照 |
| `--chart-pagination <N>` | タイムライン本体（チャート）を lane グループ単位（1 ページ N レーン）で複数ページに分割する（issue #660/#661, ADR-0005 D2）。`--output` 必須。`--format svg`（`<stem>.pageN.<ext>` の複数ファイル）と `--format pdf`（単一 PDF 内の複数ページ）の両方で有効。`--chart-pagination-range` とは併用不可 |
| `--chart-pagination-range <N>` | タイムライン本体（チャート）を時間範囲軸で `N` ページ（連続する非空の整数年区間）に分割する（issue #733/#736, ADR-0005 D3）。`--output` 必須。`--format svg`（`<stem>.pageN.<ext>` の複数ファイル）と `--format pdf`（単一 PDF 内の複数ページ）の両方で有効。`--chart-pagination` とは併用不可 |

### 出力仕様

- **形式**:
  - `html`: 単一 HTML ファイル。インライン SVG + CSS 埋め込み。デフォルトは JavaScript 非依存（`--interactive` を付けると JS 有効）
  - `svg`: スタンドアロン SVG ファイル
  - `pdf`: PDF ファイル（`svg2pdf` / `usvg` 経由、CJK フォント対応）
  - `png`: PNG ラスタ画像（`--dpi` で解像度を調整可能）
- **インタラクティブモード**（`--interactive`）: ズーム・パン・全文検索・凡例・詳細パネルを追加。`color_map` で定義した色が自動適用される
  - 凡例パネルには lane 表示トグル（チェックボックス）に加え、`timeline.color_map` に登録されたタグの絞り込みトグルが表示される（issue #755）。タグトグルは OR セマンティクス（チェックしたタグを1つも持たない item を非表示にする）で、lane トグルとは AND で合成される（どちらかで非表示なら item は非表示）。全タグのチェックを外した場合は絞り込み自体を無効化する（全 item を表示）。`color_map` が空の場合はタグ凡例セクション自体が出力されない。
- **レイアウト**:
  - 横軸: 時間（`timeline.range` を使用）
  - 縦軸: lane を `order` 昇順に縦積み
  - 時間軸の目盛りは範囲に応じて自動選択（10年/20年/50年/100年/…）
  - `--layout-style group-bands` を指定すると、連続する同一 `lane.group` を背景帯として描画する（#543）。`Orientation` とは直交するレンダリング専用オプションで、IR/DSL フィールドは追加しない。
- **要素の描画**:
  - `span` → 角丸矩形（レーン帯中央）
  - `event_range` → 細めの矩形（レーン帯下段）
  - `event` → 縦線 + 小さい円マーカー
- **色**: `timeline { color_map { タグ名: "#hex"; } }` で定義したタグ色が適用される
- **ツールチップ**: 各要素にマウスを乗せると `<title>` 要素でラベル・期間・タグ・ソース・ID が表示される
- **全 item 一覧表（`--show-table`）**：有効にすると、全 item（時期・ラベル・レーン・タグ）を時系列順に一覧する表がタイムライン本体の下に追加される（#536）。
  - `html`: リチ HTML `<table>` 要素（CSS で自由にカスタマイズ可能）。
  - `svg` / `png` / `pdf`: 同じ列構成（時期/ラベル/レーン/タグ）を SVG `<rect>`/`<text>` で描画し、タイムライン本体の高さ（`viewBox`/`height`）に自動で含める。
  - `pdf` はデフォルト（`--pdf-pagination` / `--chart-pagination` いずれも未指定）では従来と同じ単一ページベクトル方式のままであり、表を含めた全体をページに収まるように拡大縮小する。
  - `--pdf-pagination` を指定すると、`pdf` 出力はタイムライン本体（1ページ目、既存どおり単一ページ）とアイテムテーブル（2ページ目以降、用紙サイズ・余白から計算した行数ごとに分割）に分かれる（ADR-0004）。各テーブルページの先頭に列見出しを再描画し、フッタに `i / N` 形式のページ番号を付与する（`N` はテーブルページ数のみを数えたもので、タイムラインチャートページは含まない）。タイムライン本体（チャート部分）自体のページ分割は `--pdf-pagination` 単体のスコープ外（ADR-0004 D1）。`--chart-pagination` と併用した場合のページ構成は次項参照（issue #661）。
  - `--show-table` のデフォルトは `false`（非表示）で、従来の出力には影響しない。
- **タイムライン本体（チャート）の複数ページ分割（`--chart-pagination`）**：`--chart-pagination <N>`（1 ページあたりの lane 数）を指定すると、lane グループ単位でチャートを複数ページに分割する（issue #660/#661, ADR-0005 D2）。時間軸（`meta.range`）は全ページ共通で、`Item::lane` が単一 lane を持つため span/event_range のページ境界クリッピングは発生しない。`--show-legend` は各チャートページに個別描画される。lane の `group` がページ境界をまたいで分断される場合は stderr に警告を出す（silent no-op にはしない）。`--output` は必須。
  - `--format svg`: `<stem>.pageN.<ext>` ごとに別ファイルとして分割出力される（stdout 非対応）。`--show-table` を併用すると、チャートページ群の後ろに専用のテーブルページを 1 枚追加し、IR 全体（最後のチャートページの lane に限らない）の item を一覧表示する（このテーブルページは常に `1 / 1`、複数ページへの分割は未対応）。
  - `--format pdf`（issue #661）: 別ファイルには分割されず、単一の PDF ファイル内で「チャートページ群（lane グループ順）→ テーブルページ群」の順に複数ページとして出力される。`--show-table` なしならテーブルページなし。`--show-table` のみ（`--pdf-pagination` なし）なら IR 全体を 1 枚の未分割テーブルページとして末尾に追加する。`--show-table --pdf-pagination` を併用すると `--pdf-pagination` の行分割ロジックでテーブルページ群を生成し、その `i / N` フッタはテーブルページ数のみを数える（先行するチャートページ数は含めない）。`--chart-pagination` を指定しない既存の `--format pdf` 出力は本機能の追加後も完全に不変（ADR-0004 D3）。
  - `--format html` / `--format png` との併用はエラー。
- **タイムライン本体（チャート）の時間範囲軸による複数ページ分割（`--chart-pagination-range`）**：`--chart-pagination-range <N>` を指定すると、`meta.range` を `N` 個の連続する非空の整数年区間へ均等分割し、区間ごとに1ページを描画する（issue #733/#736, ADR-0005 D3）。lane グループ軸（`--chart-pagination`）と異なり `lanes`/`items` はページごとにフィルタされず、各ページの `TimelineIr` は全 item を保持したまま `meta.range`（およびサブ年精度フィールド、クリアされる）だけが書き換わる。
  - 区間境界をまたぐ `span`/`event_range` は既存の `primary_axis_segment` クランプでページごとにクリップされ（新規ジオメトリは不要）、クリップされた辺に継続マーカー（三角形の `<polygon>`、`aria-label`/`<title>` 付き、issue #734, ADR-0005 §2 戦略1）が描画される。境界をまたぐ item がある場合は `stderr` に `Warning: item "..." (...) is clipped at chart page boundary year(s) [...]; ...` も出力する（silent no-op にはしない）。マーカーは `RenderOptions::show_boundary_clip_markers`（`--chart-pagination-range` の内部レンダリングでのみ有効化）による opt-in のため、通常の（ページ分割と無関係な）狭い `range` 指定のレンダリングは従来どおりマーカーなしでクリップされる。
  - group band / gantt / zigzag / open-ended range の4機能はいずれもこの軸では追加の分岐処理が不要（band は lane フィルタが無いため常に全幅描画、gantt/zigzag は全ページ共通の item 集合から計算されるため parity が全ページで一致する）。
  - `--output` は必須。`--format svg`（`<stem>.pageN.<ext>` ごとに別ファイルとして分割出力、stdout 非対応）と `--format pdf`（issue #736、単一 PDF 内の複数ページ。ページ構成は `--chart-pagination` の PDF 出力と同じ規則：チャートページ群 → テーブルページ群の順、footer はテーブルページ数のみを数える）の両方で有効。
  - `--watch`、`--chart-pagination` との併用はいずれもエラー。
- **静的凡例（`--show-legend`）**：有効にすると、レーンごとのパレット色と `timeline.color_map` のタグ色を凡例パネルとして表示する（#544）。
  - `html`: インライン SVG 内の凡例パネルとして表示されるため、JavaScript 非依存の静的HTMLでも色対応を確認できる。
  - `svg` / `png` / `pdf`: SVG `<rect>`/`<text>` で描画し、タイムライン本体の高さ（`viewBox`/`height`）に自動で含める。
  - `--show-legend` のデフォルトは `false`（非表示）で、既存の `--interactive` 凡例とは独立した静的出力用オプションである。

## サンプルと WebUI ギャラリー

`examples/*.tdsl` は WebUI テンプレートギャラリーのソースでもあり、ギャラリー側は `.tdsl` 本文を埋め込まず raw import で参照する。
各サンプルの description は、例示している DSL 機能を明示する。

| 機能 | 代表サンプル |
|---|---|
| `group { ... }` | `examples/grouped_dynasties.tdsl` |
| `color_map { ... }` | `examples/fictional_empire.tdsl` |
| 月・日精度の日付 | `examples/world_wars.tdsl`, `examples/apollo_11.tdsl` |
| 時刻精度・sub-day 軸 | `examples/apollo_11_hourly.tdsl` |
| 秒精度・UTCオフセット（ADR 0003） | `examples/iss_docking_second_precision.tdsl` |
| 複数タイムゾーンオフセット（ADR 0003 D2） | `examples/global_conference_timezones.tdsl` |
| `policy field_priority { ... }` | `examples/template_apply_example.tdsl` |
| `claim(P39).qualifier(P580/P582)` | `examples/officeholder_wikidata.tdsl` |
| CSV 取り込み導線 | `examples/fictional_empire.tdsl`, `examples/fictional_empire_items.csv` |
| `note` / `link` / `color`（block_options）・open-ended `now` | `examples/feature_showcase.tdsl` |
| `filter` 式（map ブロック） | `examples/china_dynasties_filtered.tdsl` |

Wikidata を必要とするサンプルは WebUI では「CLI 専用・構文リファレンス」として表示する。ブラウザ/WASM 実行では `import wikidata` を解決しないため、オンライン取得は `tdsl build` / `tdsl render` を CLI で実行する。

## ライセンスとデータ利用

- **Wikidataの構造化データ**: CC0ライセンス。出典表示なしで自由に利用可能
- **Wikipediaの文章・図表**: CC BY-SA 4.0。引用時は出典表示と同ライセンス適用が必須
