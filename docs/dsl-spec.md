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
                 | "color_map" "{" { <identifier> ":" <string> ";" } "}"

<lane>         ::= "lane" <string> ["as" <identifier>] "{" { <lane_prop> } "}"
<lane_prop>    ::= "kind" <identifier> ";"
                 | "order" <number> ";"

<group>        ::= "group" <string> "{" <lane> { <lane> } "}"

<span>         ::= "span" <identifier> <time_value> ".." <time_value> <string>
                   <block_options> ";"
<event>        ::= "event" <identifier> <time_value> <string>
                   <block_options> ";"
<event_range>  ::= "event_range" <identifier> <time_value> ".." <time_value> <string>
                   <block_options> ";"

<block_options> ::= "{" { <option> } "}"
<option>       ::= "tags" "[" <string_list> "]" ";"
                 | "source" <source_ref> ";"
                 | "origin" <identifier> ";"
                 | "id" <string> ";"

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

<claim_expr>   ::= "claim(" <property_id> ")" ["." "qualifier(" <property_id> ")"] ["." <function>] [<claim_offset>]
<map_expr>     ::= (<claim_expr> | <number>) { "??" (<claim_expr> | <number>) }
<lang_expr>    ::= <label_ref> { "??" <label_ref> }
<label_ref>    ::= "label@" <lang_code>

<source_ref>   ::= <identifier> ":" <qid>
<string_list>  ::= <string> { "," <string> }
<qid>          ::= "Q" <digits>
<property_id>  ::= "P" <digits>
<identifier>   ::= /[A-Za-z_][A-Za-z0-9_-]*/
<number>       ::= /"-"? [0-9]+/
<time_value>   ::= <date_time> | <date> | <year_month> | <year>
<year>         ::= /"-"? [0-9]+/
<year_month>   ::= /"-"? [0-9]{1,4} "-" [0-9]{2}/
<date>         ::= /"-"? [0-9]{1,4} "-" [0-9]{2} "-" [0-9]{2}/
<date_time>    ::= /"-"? [0-9]{1,4} "-" [0-9]{2} "-" [0-9]{2} "T" [0-9]{2} ":" [0-9]{2}/
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
| `unit` | 任意 | 時間単位（`year`） |
| `range` | 任意 | 表示範囲。`開始..終了` の形式。負の値は紀元前 |
| `calendar` | 任意 | 暦法。`proleptic_gregorian` 等 |
| `color_map` | 任意 | タグ→色のマッピング。`タグ名: "#16進数カラーコード";` の形式で複数定義可能 |

`color_map` で定義した色は `tdsl render` 時に自動適用される。`color_map` は hex 色（`#RGB`, `#RGBA`, `#RRGGBB`, `#RRGGBBAA`）と単純な CSS 色キーワードを受け付ける。複雑な CSS 値は安全のため renderer が無視する。高度な装飾は CLI の `--custom-css` を使う。`--color-map "war=#cc0000"` CLIフラグで上書きも可能。

### lane

年表の縦軸カテゴリ（レーン）を定義する。王朝・人物・国・組織などを表す。

```
lane "漢" as han { kind dynasty; order 20; }
```

| プロパティ | 必須 | 説明 |
|---|---|---|
| `as <id>` | 任意 | 内部識別子。省略時はラベルからスラッグを自動生成 |
| `kind` | 任意 | 分類（`dynasty`, `person`, `nation` 等） |
| `order` | 任意 | 初期表示順（整数） |

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
```

- 第1引数: レーンID
- 第2引数: `開始..終了`（時刻値の範囲。`1939-09-01..1945-09-02` のように月・日精度も指定可）
- 第3引数: ラベル（文字列）

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
```

- 第1引数: レーンID
- 第2引数: `開始..終了`（時刻値の範囲）
- 第3引数: ラベル（文字列）

### block_options（共通オプション）

span / event / event_range に付与できるオプション。

| オプション | 説明 | 例 |
|---|---|---|
| `tags` | タグのリスト | `tags ["war", "major"];` |
| `source` | データソース（Wikidata等） | `source wd:Q7209;` |
| `id` | 要素の安定識別子 | `id "span:han";` |
| `origin` | 由来の識別子 | `origin imported;` |

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

- 範囲: `開始..終了`（例: `-0206-01-15..-0206-02-20`, `1939-09-01..1945-09-02`, `1969-07-20T20:17..1969-07-20T21:00`）
- 紀元前（負の年）の月日・時分精度は、符号付き4桁年（例: `-0206-01-15`）で表す
- Wikidataの時刻値は `.year` / `.month` / `.day` / `.hour` / `.minute` 関数で各精度の値を取得可能

## CLI

### サブコマンド一覧

| コマンド | 目的 |
|---|---|
| `tdsl build <file>` | `.tdsl` をJSON IRに変換 |
| `tdsl check <file>` | 構文・意味チェック |
| `tdsl ast <file>` | ASTダンプ |
| `tdsl render <file>` | HTML / SVG / PDF / PNG を生成（`--format html\|svg\|pdf\|png`、`--interactive`） |
| `tdsl decompile <json>` | JSON IRを `.tdsl` ソースに逆変換 |
| `tdsl fetch <QID>` | Wikidataエンティティ確認 |
| `tdsl search <query>` | Wikidata候補検索 |
| `tdsl inspect <QID>` | 年表化適性の診断 |
| `tdsl resolve <wikipedia-url>` | Wikipedia URL から QID を解決 |
| `tdsl scaffold wikidata ...` | QID群から `.tdsl` 雛形生成 |
| `tdsl init ...` | 手作業向け `.tdsl` テンプレ生成 |
| `tdsl import-csv <csv>` | CSVから `span/event/event_range` 生成 |
| `tdsl lint <file> [--fix]` | 品質チェックと安全な自動補正 |
| `tdsl cache status` | ローカルキャッシュの状態を表示 |
| `tdsl cache clear [--older-than <days>]` | キャッシュエントリを削除 |

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
| `--scale` | 1年あたりのピクセル幅（デフォルト 2） |
| `--lane-height` | 各レーンの高さ（px、デフォルト 60）。縦密度を制御し、バーの太さも追従する |
| `--dpi` | PNG 出力の DPI（デフォルト 96）。`--format png` のみ有効 |
| `--offline` | Wikidata fetch を省略 |

### 出力仕様

- **形式**:
  - `html`: 単一 HTML ファイル。インライン SVG + CSS 埋め込み。デフォルトは JavaScript 非依存（`--interactive` を付けると JS 有効）
  - `svg`: スタンドアロン SVG ファイル
  - `pdf`: PDF ファイル（`svg2pdf` / `usvg` 経由、CJK フォント対応）
  - `png`: PNG ラスタ画像（`--dpi` で解像度を調整可能）
- **インタラクティブモード**（`--interactive`）: ズーム・パン・全文検索・凡例・詳細パネルを追加。`color_map` で定義した色が自動適用される
- **レイアウト**:
  - 横軸: 時間（`timeline.range` を使用）
  - 縦軸: lane を `order` 昇順に縦積み
  - 時間軸の目盛りは範囲に応じて自動選択（10年/20年/50年/100年/…）
- **要素の描画**:
  - `span` → 角丸矩形（レーン帯中央）
  - `event_range` → 細めの矩形（レーン帯下段）
  - `event` → 縦線 + 小さい円マーカー
- **色**: `timeline { color_map { タグ名: "#hex"; } }` で定義したタグ色が適用される
- **ツールチップ**: 各要素にマウスを乗せると `<title>` 要素でラベル・期間・タグ・ソース・ID が表示される
- **全 item 一覧表（`--show-table`）**：有効にすると、全 item（時期・ラベル・レーン・タグ）を時系列順に一覧する表がタイムライン本体の下に追加される（#536）。
  - `html`: リチ HTML `<table>` 要素（CSS で自由にカスタマイズ可能）。
  - `svg` / `png` / `pdf`: 同じ列構成（時期/ラベル/レーン/タグ）を SVG `<rect>`/`<text>` で描画し、タイムライン本体の高さ（`viewBox`/`height`）に自動で含める。
  - `pdf` は従来と同じ単一ページベクトル方式のままであり、表を含めた全体をページに収まるように拡大縮小する。本体と表のページ分割は未実装（将来拡張）である。
  - `--show-table` のデフォルトは `false`（非表示）で、従来の出力には影響しない。

## サンプルと WebUI ギャラリー

`examples/*.tdsl` は WebUI テンプレートギャラリーのソースでもあり、ギャラリー側は `.tdsl` 本文を埋め込まず raw import で参照する。
各サンプルの description は、例示している DSL 機能を明示する。

| 機能 | 代表サンプル |
|---|---|
| `group { ... }` | `examples/grouped_dynasties.tdsl` |
| `color_map { ... }` | `examples/fictional_empire.tdsl` |
| 月・日精度の日付 | `examples/world_wars.tdsl`, `examples/apollo_11.tdsl` |
| `policy field_priority { ... }` | `examples/template_apply_example.tdsl` |
| `claim(P39).qualifier(P580/P582)` | `examples/officeholder_wikidata.tdsl` |
| CSV 取り込み導線 | `examples/fictional_empire.tdsl`, `examples/fictional_empire_items.csv` |

Wikidata を必要とするサンプルは WebUI では「CLI 専用・構文リファレンス」として表示する。ブラウザ/WASM 実行では `import wikidata` を解決しないため、オンライン取得は `tdsl build` / `tdsl render` を CLI で実行する。

## ライセンスとデータ利用

- **Wikidataの構造化データ**: CC0ライセンス。出典表示なしで自由に利用可能
- **Wikipediaの文章・図表**: CC BY-SA 4.0。引用時は出典表示と同ライセンス適用が必須
