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
                 | "source" <expr> ";"

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
| `merge_by_source` | 同一ソースの項目を同一視。手動修正を優先 |
| `overwrite_imported` | インポート済みデータを常にWikidata最新で上書き |
| `keep_manual` | インポート済み部分は変更せず、手動追加のみ許可 |

### map

インポートしたエンティティを年表要素に変換するルール。

```
map wd.han_dynasty to span {
    lane han;
    start claim(P571).year;
    end claim(P576).year;
    label label@ja ?? label@en;
    tags ["dynasty", "imported"];
    source claim(P571).year;
}
```

| プロパティ | 説明 |
|---|---|
| `lane` | 対象レーンのID |
| `start` | 開始時点を計算する式 |
| `end` | 終了時点を計算する式 |
| `time` | 点イベントの時点を計算する式（event用） |
| `label` | ラベルを計算する式 |
| `tags` | タグのリスト |
| `source` | ソースを計算する式 |

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
