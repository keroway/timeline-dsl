# Timeline DSL

[![CI](https://github.com/keroway/timeline-dsl/actions/workflows/ci.yml/badge.svg)](https://github.com/keroway/timeline-dsl/actions/workflows/ci.yml)
[![Release](https://github.com/keroway/timeline-dsl/actions/workflows/release.yml/badge.svg)](https://github.com/keroway/timeline-dsl/actions/workflows/release.yml)

年表特化のドメイン固有言語（DSL）コンパイラ。テキストベースで年表を定義し、Wikidataから構造化データを自動インポートできる。

## 特徴

- **宣言型DSL** -- C風の構文で年表をテキスト定義。Git管理・差分レビューに最適
- **Wikidata連携** -- QIDでWikidataからデータを自動取得
- **JSON IR出力** -- パース結果を正規化JSONに変換。エディタや描画エンジンで利用可能
- **HTMLレンダリング** -- スタンドアロンHTML + インラインSVGでタイムラインを可視化。ホバーで詳細表示
- **レーン構造** -- 王朝・人物・国などをレーン（縦軸カテゴリ）で整理
- **3種の時間要素** -- `span`（存続期間）、`event`（点イベント）、`event_range`（期間イベント）
- **ライセンス追跡** -- Wikidataデータ(CC0)の出典を自動記録

## インストール

### ワンラインインストール（macOS / Linux）

```sh
curl -sSfL https://raw.githubusercontent.com/keroway/timeline-dsl/main/install.sh | sh
```

### Homebrew（macOS / Linux）

```sh
brew tap keroway/tap
brew install tdsl
```

### cargo でインストール（Rust開発者向け）

```sh
cargo install --git https://github.com/keroway/timeline-dsl tdsl-cli
```

## クイックスタート

インストール後は `tdsl` コマンドが直接使えます。

### 基本的な使い方

```bash
# DSLファイルをJSONにコンパイル
tdsl build examples/china_dynasties.tdsl --pretty

# 構文・意味チェック
tdsl check examples/china_dynasties.tdsl

# Wikidata連携つきコンパイル
tdsl build examples/china_with_import.tdsl --pretty

# オフラインモード（Wikidataアクセスなし）
tdsl build examples/china_with_import.tdsl --offline --pretty

# ASTダンプ（デバッグ用）
tdsl ast examples/china_dynasties.tdsl

# Wikidataエンティティの確認
tdsl fetch Q7209 --lang ja,en

# Wikipedia URL から QID を解決
tdsl resolve "https://ja.wikipedia.org/wiki/漢" --lang ja,en

# スタンドアロンHTMLにレンダリング（ブラウザで開くだけ）
tdsl render examples/china_dynasties.tdsl --output china.html
open china.html

# スケールを大きくして描画
tdsl render examples/china_dynasties.tdsl --scale 5 --output china.html

# 手作業向けテンプレートを生成
tdsl init --output /tmp/manual.tdsl --timeline "架空世界年表" --range-start 1000 --range-end 1300 --lanes "王国:kingdom,事件:incidents"

# CSVから item 定義を生成（stdout）
tdsl import-csv examples/fictional_empire_items.csv

# CSVから item 定義を既存ファイルへ追記
tdsl import-csv examples/fictional_empire_items.csv --append /tmp/manual.tdsl

# 品質チェック（テキスト表示）
tdsl lint /tmp/manual.tdsl

# 自動修正付き品質チェック（JSON表示）
tdsl lint /tmp/manual.tdsl --fix --format json
```

### 最短フロー（Wikidata起点）

```bash
# 1) 候補を探す
tdsl search "漢王朝" --lang ja -n 5

# (任意) Wikipedia URL からQIDを解決
tdsl resolve "https://ja.wikipedia.org/wiki/漢"

# 2) QIDの年表化適性を確認
tdsl inspect Q7209 --lang ja,en

# 3) .tdsl 雛形を生成
tdsl scaffold wikidata \
  --qids Q7183,Q7209 \
  --timeline "中国王朝(生成)" \
  --lang ja,en \
  --target auto \
  --lane-mode per-entity \
  --output /tmp/china_scaffold.tdsl

# 4) HTMLに描画
tdsl render /tmp/china_scaffold.tdsl --output /tmp/china_scaffold.html
```

> `search / inspect / resolve / scaffold wikidata` はネットワーク接続が必要です。

### 最短フロー（手作業起点）

```bash
# 1) 年表テンプレート生成
tdsl init \
  --output /tmp/manual.tdsl \
  --timeline "架空世界年表" \
  --range-start 1000 \
  --range-end 1300 \
  --lanes "王国:kingdom,事件:incidents"

# 2) CSVから項目を追記
tdsl import-csv examples/fictional_empire_items.csv --append /tmp/manual.tdsl

# 3) 品質補正
tdsl lint /tmp/manual.tdsl --fix

# 4) HTMLに描画
tdsl render /tmp/manual.tdsl --output /tmp/manual.html
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
}
```

> `source` はMVP では自動付与（`wd:<entity_id>`）。`map` ブロック内での明示指定は不要。
> `policy` はID衝突時の挙動を切り替える:
> `merge_by_source` は衝突をエラー扱い、`overwrite_imported` は既存 imported 項目のみ置換、
> `keep_manual` は既存項目（主に手動定義）を優先して imported 側をスキップ。

## サンプルファイル

| ファイル | 内容 |
|---|---|
| `examples/china_dynasties.tdsl` | 静的定義のみ。秦・漢・三国の年表 |
| `examples/china_with_import.tdsl` | Wikidata連携つき。秦・漢をQIDからインポート |
| `examples/fictional_empire.tdsl` | 架空世界向けの手作業年表サンプル |
| `examples/fictional_empire_items.csv` | `import-csv` 用の入力CSVサンプル |
| `examples/japanese_history.tdsl` | 日本史（奈良〜江戸）。複数lane・静的定義 |
| `examples/samurai_wikidata.tdsl` | 戦国武将の生没年。Wikidata連携（P569/P570）サンプル |
| `examples/world_wars.tdsl` | 近代戦争年表。event_range 中心の年表 |
| `examples/sci_tech_timeline.tdsl` | 科学技術の発明・発見年表。event 中心の年表 |

## エディタサポート

### VS Code 構文ハイライト

`editors/vscode/` に VS Code 拡張があります。`.tdsl` ファイルのキーワード・文字列・コメント・QID などを色分けします。

**インストール方法（手動）:**

```bash
# プロジェクトルートから
cp -r editors/vscode ~/.vscode/extensions/timeline-dsl
# VS Code を再起動
```

ハイライト対象:
- キーワード: `timeline`, `lane`, `span`, `event`, `event_range`, `import`, `map`, `template`, `apply`
- 文字列リテラル（ダブルクォート）
- コメント（`//` と `/* */`）
- Wikidata QID（`Q123`）・プロパティID（`P569`）・参照（`wd:Q123`）
- `claim(P571).year` 式、`label@ja` 式

## Lint

`tdsl lint <file> [--fix] [--format text|json]`

- 初期ルール: 未定義lane参照 / 重複id / `start > end` / 空label / タグの空要素・重複
- `--fix` 対応: タグ重複除去・空タグ除去 / `start,end` 入れ替え / `id` 未設定時の安定ID生成
- `--format json` はCI連携向けに issue 一覧と `ok` フラグを出力

## ドキュメント

- [Getting Started チュートリアル](docs/tutorial.md) — ステップバイステップのハンズオン
- [DSL 言語仕様](docs/dsl-spec.md) — 文法リファレンス
- [エラーコードカタログ](docs/error-catalog.md) — エラーメッセージの原因と修正方法
- [v0→v1 移行ガイド](docs/migration-v0-to-v1.md) — バージョンアップ時の変更点

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
[tdsl-wikidata] Wikidata APIクライアント
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
| `tdsl-render` | IR → スタンドアロンHTML（インラインSVG）レンダラ |
| `tdsl-cli` | CLIバイナリ（build / check / ast / fetch / search / inspect / resolve / scaffold / init / import-csv / render / lint） |

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
    {"type": "span", "id": "span:han", "lane": "han", "start": -206, "end": 220, "label": "漢", "tags": ["dynasty"], "source": "wd:Q7209", "origin": "wikidata"}
  ],
  "imports": [
    {"source": "wikidata", "qid": "Q7209", "mapped_to": "span:han"}
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

# E2Eスモーク（CIと同じ）
bash scripts/e2e-smoke.sh
```

## ライセンス

### このソフトウェア

MIT License — 詳細は [LICENSE](./LICENSE) を参照。

### Wikidataから取得したデータ

Wikidataの構造化データは [CC0 1.0](https://creativecommons.org/publicdomain/zero/1.0/) で提供されます。
`tdsl` でWikidataデータをインポートした場合、そのデータ自体は出典表示なしで自由に利用できます。
これはこのソフトウェア（MIT）とは独立した条件です。
