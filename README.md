# Timeline DSL

年表特化のドメイン固有言語（DSL）コンパイラ。テキストベースで年表を定義し、Wikidataから構造化データを自動インポートできる。

## 特徴

- **宣言型DSL** -- C風の構文で年表をテキスト定義。Git管理・差分レビューに最適
- **Wikidata連携** -- QIDやSPARQLクエリでWikidataからデータを自動取得
- **JSON IR出力** -- パース結果を正規化JSONに変換。エディタや描画エンジンで利用可能
- **レーン構造** -- 王朝・人物・国などをレーン（縦軸カテゴリ）で整理
- **3種の時間要素** -- `span`（存続期間）、`event`（点イベント）、`event_range`（期間イベント）
- **ライセンス追跡** -- Wikidataデータ(CC0)の出典を自動記録

## クイックスタート

### ビルド

```bash
cargo build --release
```

### 基本的な使い方

```bash
# DSLファイルをJSONにコンパイル
cargo run --release -p tdsl-cli -- build examples/china_dynasties.tdsl --pretty

# 構文・意味チェック
cargo run --release -p tdsl-cli -- check examples/china_dynasties.tdsl

# Wikidata連携つきコンパイル
cargo run --release -p tdsl-cli -- build examples/china_with_import.tdsl --pretty

# オフラインモード（Wikidataアクセスなし）
cargo run --release -p tdsl-cli -- build examples/china_with_import.tdsl --offline --pretty

# ASTダンプ（デバッグ用）
cargo run --release -p tdsl-cli -- ast examples/china_dynasties.tdsl

# Wikidataエンティティの確認
cargo run --release -p tdsl-cli -- fetch Q7209 --lang ja,en
```

## DSL文法

### timeline ブロック

タイトル・単位・表示範囲・暦法を宣言する。

```
timeline "中国王朝年表" {
    title "中国王朝年表";
    unit year;
    range -500..2000;
    calendar proleptic_gregorian;
}
```

### lane 宣言

年表の縦軸カテゴリを定義する。`as` で内部IDを指定。

```
lane "漢" as han { kind dynasty; order 20; }
```

### span / event / event_range

年表の時間要素。レーンに紐付ける。

```
// 存続期間
span han -206..220 "漢" { tags ["dynasty"]; source wd:Q7209; id "span:han"; };

// 点イベント
event han -209 "陳勝・呉広の乱" {};

// 期間イベント（戦争・災害など）
event_range han 184..204 "黄巾の乱" { tags ["war"]; };
```

### import ブロック

Wikidataからのデータ取り込みを宣言する。

```
import wikidata as wd {
    entity Q7183 as qin_dynasty;
    entity Q7209 as han_dynasty;
    policy merge_by_source;
}
```

### map ブロック

インポートしたエンティティを年表要素に変換するルール。

```
map wd.han_dynasty to span {
    lane han;
    start claim(P571).year;      // inception
    end claim(P576).year;        // dissolved
    label label@ja ?? label@en;  // 日本語優先、英語フォールバック
    tags ["dynasty", "imported"];
    source claim(P571).year;
}
```

## サンプルファイル

| ファイル | 内容 |
|---|---|
| `examples/china_dynasties.tdsl` | 静的定義のみ。秦・漢・三国の年表 |
| `examples/china_with_import.tdsl` | Wikidata連携つき。秦・漢をQIDからインポート |

## アーキテクチャ

```
.tdsl ファイル
    |
    v
[tdsl-parser]  PEG文法(pest) → AST
    |
    v
[tdsl-core]    AST → IR変換（4パスパイプライン）
    |               Pass 1: timeline/lane 宣言の収集
    |               Pass 2: 静的アイテムの変換
    |               Pass 3: Wikidataインポート解決
    |               Pass 4: mapブロックの適用
    v
[tdsl-wikidata] Wikidata API / SPARQLクライアント
    |
    v
JSON IR 出力
```

### クレート構成

| クレート | 役割 |
|---|---|
| `tdsl-parser` | PEG文法定義とAST構築 |
| `tdsl-core` | IR変換（lowering）・バリデーション |
| `tdsl-wikidata` | Wikidata HTTPクライアント・エンティティモデル |
| `tdsl-cli` | CLIバイナリ（build / check / ast / fetch） |

## IR (中間表現) の構造

コンパイル結果のJSON IRは以下の構造を持つ。

```json
{
  "meta": {
    "title": "中国王朝年表",
    "unit": "year",
    "range": [-500, 2000],
    "calendar": "proleptic_gregorian"
  },
  "lanes": [
    {"id": "han", "label": "漢", "kind": "dynasty", "order": 20}
  ],
  "items": [
    {"type": "span", "id": "span:han", "lane": "han", "start": -206, "end": 220, "label": "漢", "tags": ["dynasty"], "source": "wd:Q7209"}
  ],
  "imports": [
    {"source": "wikidata", "entity_id": "Q7209", "mapped_to": ["span:han"]}
  ],
  "sources": [
    {"id": "wd:Q7209", "provider": "wikidata", "license": "CC0"}
  ]
}
```

## Wikidataプロパティ

年表構築で頻用するプロパティ:

| プロパティ | 用途 | DSLでの使い方 |
|---|---|---|
| P569 | 人物の誕生年 | `claim(P569).year` |
| P570 | 人物の死亡年 | `claim(P570).year` |
| P571 | 組織・国の成立年 | `claim(P571).year` |
| P576 | 組織・国の消滅年 | `claim(P576).year` |
| P580 | 開始時点 | `claim(P580).year` |
| P582 | 終了時点 | `claim(P582).year` |

## テスト

```bash
cargo test --workspace
```

## ライセンス

MIT License

Wikidataの構造化データは [CC0](https://www.wikidata.org/wiki/Wikidata:Licensing) で提供されており、出典表示なしで自由に利用可能。
