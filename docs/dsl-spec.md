# Timeline DSL 言語仕様

## 概要

Timeline DSL（`.tdsl`）は年表データを宣言的に記述するためのドメイン固有言語。C風の波括弧+セミコロン構文を採用し、可読性とGit差分管理のしやすさを重視している。

## 文法（EBNF）

```ebnf
<document>     ::= { <statement> }

<statement>    ::= <timeline>
                 | <lane>
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
                 | "range" <number> ".." <number> ";"
                 | "calendar" <identifier> ";"

<lane>         ::= "lane" <string> ["as" <identifier>] "{" { <lane_prop> } "}"
<lane_prop>    ::= "kind" <identifier> ";"
                 | "order" <number> ";"

<span>         ::= "span" <identifier> <number> ".." <number> <string>
                   <block_options> ";"
<event>        ::= "event" <identifier> <number> <string>
                   <block_options> ";"
<event_range>  ::= "event_range" <identifier> <number> ".." <number> <string>
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

<map_block>    ::= "map" <import_ref> "to" <mapping_target>
                   "{" { <mapping_rule> } "}"
<mapping_target> ::= "span" | "event" | "event_range"
<mapping_rule> ::= "lane" <identifier> ";"
                 | "start" <expr> ";"
                 | "end" <expr> ";"
                 | "time" <expr> ";"
                 | "label" <expr> ";"
                 | "tags" "[" <string_list> "]" ";"

<template_block> ::= "template" <string> ["as" <identifier>]
                   "to" <mapping_target> "{" { <mapping_rule> } "}"

<apply_block>  ::= "apply" <identifier> "to" <identifier>
                   "{" { <apply_override> } "}"
<apply_override> ::= "lane" <identifier> ";"

<expr>         ::= <claim_expr> | <lang_expr> | <literal>
<claim_expr>   ::= "claim(" <property_id> ")" ["." <function>]
<lang_expr>    ::= "label@" <lang_code> ["??" <lang_expr>]

<source_ref>   ::= <identifier> ":" <qid>
<string_list>  ::= <string> { "," <string> }
<qid>          ::= "Q" <digits>
<property_id>  ::= "P" <digits>
<identifier>   ::= /[A-Za-z_][A-Za-z0-9_-]*/
<number>       ::= /"-"? [0-9]+/
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
}
```

| プロパティ | 必須 | 説明 |
|---|---|---|
| `title` | 任意 | 年表の表示タイトル |
| `unit` | 任意 | 時間単位（`year`） |
| `range` | 任意 | 表示範囲。`開始..終了` の形式。負の値は紀元前 |
| `calendar` | 任意 | 暦法。`proleptic_gregorian` 等 |

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

### span

存続期間を表す。王朝の在位期間や人物の生没年など。

```
span han -206..220 "漢" { tags ["dynasty"]; source wd:Q7209; id "span:han"; };
```

- 第1引数: レーンID
- 第2引数: `開始..終了`（整数範囲）
- 第3引数: ラベル（文字列）

### event

特定の時点に起きたイベント。

```
event han -209 "陳勝・呉広の乱" {};
```

- 第1引数: レーンID
- 第2引数: 時点（整数）
- 第3引数: ラベル（文字列）

### event_range

一定期間のイベント。戦争・災害・プロジェクトなど。

```
event_range han 184..204 "黄巾の乱" { tags ["war"]; };
```

- 第1引数: レーンID
- 第2引数: `開始..終了`（整数範囲）
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
}
```

| 要素 | 説明 |
|---|---|
| `entity <QID>` | 特定のWikidataエンティティを指定 |
| `query <SPARQL>` | SPARQLクエリで複数エンティティを取得 |
| `policy <name>` | 再インポート時のマージ戦略 |
| `as <alias>` | インポートブロック/エンティティの別名 |

#### 再インポートポリシー

| ポリシー | 動作 |
|---|---|
| `merge_by_source` | ID衝突をエラーとして扱う（デフォルト） |
| `overwrite_imported` | 既存のインポート済み項目のみ上書き。手動定義との衝突はエラー |
| `keep_manual` | ID衝突時はインポート側をスキップして既存項目を保持 |

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

> `source` はインポートされたアイテムに `wd:<entity_id>` として自動付与されます。`map` ブロック内での明示指定は廃止されています。

| プロパティ | 説明 |
|---|---|
| `lane` | 対象レーンのID |
| `start` | 開始時点を計算する式 |
| `end` | 終了時点を計算する式 |
| `time` | 点イベントの時点を計算する式（event用） |
| `label` | ラベルを計算する式 |
| `tags` | タグのリスト |

### 式（Expression）

#### claim 式

Wikidataのプロパティ値を取得する。

```
claim(P571).year    // P571 (inception) の時刻値を年に変換
claim(P569).year    // P569 (date of birth) の時刻値を年に変換
```

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

- 正の整数: 西暦年（例: `220` = 220年）
- 負の整数: 紀元前（例: `-206` = 紀元前206年）
- 範囲: `開始..終了`（例: `-206..220`）
- Wikidataの時刻値は `.year` 関数で整数年に変換

## CLI

### サブコマンド一覧

| コマンド | 目的 |
|---|---|
| `tdsl build <file>` | `.tdsl` をJSON IRに変換 |
| `tdsl check <file>` | 構文・意味チェック |
| `tdsl ast <file>` | ASTダンプ |
| `tdsl render <file>` | スタンドアロンHTMLを生成 |
| `tdsl fetch <QID>` | Wikidataエンティティ確認 |
| `tdsl search <query>` | Wikidata候補検索 |
| `tdsl inspect <QID>` | 年表化適性の診断 |
| `tdsl resolve <wikipedia-url>` | Wikipedia URL から QID を解決 |
| `tdsl scaffold wikidata ...` | QID群から `.tdsl` 雛形生成 |
| `tdsl init ...` | 手作業向け `.tdsl` テンプレ生成 |
| `tdsl import-csv <csv>` | CSVから `span/event/event_range` 生成 |
| `tdsl lint <file> [--fix]` | 品質チェックと安全な自動補正 |

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

`tdsl render` はJSON IRではなくスタンドアロンHTMLとしてタイムラインを可視化する。

```bash
tdsl render input.tdsl --output timeline.html [--scale N] [--offline]
```

| オプション | 説明 |
|---|---|
| `--output` | 出力パス。省略時は標準出力 |
| `--scale` | 1年あたりのピクセル幅（デフォルト 2） |
| `--offline` | Wikidata fetch を省略 |

### 出力仕様

- **形式**: 単一のHTMLファイル。インラインSVG + CSS 埋め込み。JavaScript非依存
- **レイアウト**:
  - 横軸: 時間（`timeline.range` を使用）
  - 縦軸: lane を `order` 昇順に縦積み
  - 時間軸の目盛りは範囲に応じて自動選択（10年/20年/50年/100年/…）
- **要素の描画**:
  - `span` → 角丸矩形（レーン帯中央）
  - `event_range` → 細めの矩形（レーン帯下段）
  - `event` → 縦線 + 小さい円マーカー
- **ツールチップ**: 各要素にマウスを乗せると `<title>` 要素でラベル・期間・タグ・ソース・ID が表示される

### 制約（MVP）

- ズーム・パン・検索なし（JSなし）
- タグ→色のカラーマップ未対応（`span` は青、`event_range` は赤、固定配色）
- PNG/PDF 等のラスタ出力は未対応（HTMLをブラウザ経由で印刷・スクリーンショットで代替）

## ライセンスとデータ利用

- **Wikidataの構造化データ**: CC0ライセンス。出典表示なしで自由に利用可能
- **Wikipediaの文章・図表**: CC BY-SA 4.0。引用時は出典表示と同ライセンス適用が必須

## 将来の拡張（未実装）

### template / apply

マッピングをテンプレート化し、複数エンティティに適用する構文。

```
template PersonLife(entity) {
    lane entity.label@ja ?? entity.label@en as entity.qid;
    let birth = entity.claim(P569).year;
    let death = entity.claim(P570).year;
    if birth != null && death != null {
        span entity.qid birth..death "生没" {
            tags ["person"];
            source wd:entity.qid;
        };
    }
}

apply PersonLife to wd.SengokuSamurai;
```

### フィールド別優先度

再インポート時にフィールドごとの優先度を設定:
- `label`: 手動優先
- `time`: Wikidata優先
- `tags`: 統合（マージ）
