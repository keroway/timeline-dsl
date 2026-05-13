# Timeline DSL

[![CI](https://github.com/keroway/timeline-dsl/actions/workflows/ci.yml/badge.svg)](https://github.com/keroway/timeline-dsl/actions/workflows/ci.yml)
[![Release](https://github.com/keroway/timeline-dsl/actions/workflows/release.yml/badge.svg)](https://github.com/keroway/timeline-dsl/actions/workflows/release.yml)

年表特化のドメイン固有言語（DSL）コンパイラ。テキストベースで年表を定義し、WikidataからデータをインポートしてHTML/SVGで可視化できる。

**[ランディングページ →](https://timeline-dsl-lp.pages.dev/)** | **[WebUI で今すぐ試す →](https://keroway.github.io/timeline-dsl/)**

> English version: [README.md](./README.md)

## 特徴

- **宣言型DSL** — C風の構文で年表をテキスト定義。Git管理・差分レビューに最適
- **Wikidata連携** — QIDを指定するだけで歴史データを自動取得。ローカルキャッシュ（24時間TTL）でオフライン利用も可能
- **インタラクティブHTML出力** — ズーム・パン・検索・凡例・詳細パネルを内蔵したスタンドアロンHTMLを生成
- **SVG出力** — ベクター形式で書き出し。論文・スライドへの組み込みに
- **カラーマッピング** — タグ→色の対応を DSL 内または CLI フラグで指定
- **逆コンパイル** — JSON IRから`.tdsl`ソースを再生成
- **WebUI** — ブラウザ上でリアルタイム編集・プレビュー（WASM駆動）。フォントサイズ・ライト/ダークテーマ選択対応
- **レーン構造** — 王朝・人物・国などをレーン（縦軸カテゴリ）で整理
- **3種の時間要素** — `span`（存続期間）、`event`（点イベント）、`event_range`（期間イベント）
- **ライセンス追跡** — Wikidataデータ（CC0）の出典を自動記録

## インストール

### ワンラインインストール（macOS / Linux）

```sh
curl -sSfL https://raw.githubusercontent.com/keroway/timeline-dsl/main/install.sh | sh
```

### ワンラインインストール（Windows）

PowerShell で実行します。

```powershell
irm https://raw.githubusercontent.com/keroway/timeline-dsl/main/install.ps1 | iex
```

### Homebrew（macOS / Linux）

```sh
brew tap keroway/tap
brew install tdsl
```

### cargo-binstall（高速）

```sh
cargo binstall tdsl-cli
```

[cargo-binstall](https://github.com/cargo-bins/cargo-binstall) を事前にインストールしておく必要があります。プリビルドバイナリを直接ダウンロードするため、ソースコンパイル不要です。

### cargo でインストール

```sh
cargo install --git https://github.com/keroway/timeline-dsl tdsl-cli
```

## クイックスタート

### 基本的な使い方

```bash
# DSLファイルをJSONにコンパイル
tdsl build examples/china_dynasties.tdsl --pretty

# 構文・意味チェック
tdsl check examples/china_dynasties.tdsl

# スタンドアロンHTMLにレンダリング（ブラウザで開くだけ）
tdsl render examples/china_dynasties.tdsl --output china.html
open china.html

# インタラクティブHTML（ズーム・パン・検索・詳細パネル付き）
tdsl render examples/china_dynasties.tdsl --interactive --output china.html

# SVG形式で出力
tdsl render examples/china_dynasties.tdsl --format svg --output china.svg

# Wikidata連携つきコンパイル
tdsl build examples/china_with_import.tdsl --pretty

# オフラインモード（キャッシュのみ使用）
tdsl build examples/china_with_import.tdsl --offline --pretty

# ASTダンプ（デバッグ用）
tdsl ast examples/china_dynasties.tdsl

# Wikidataエンティティの確認
tdsl fetch Q7209 --lang ja,en

# Wikipedia URL から QID を解決
tdsl resolve "https://ja.wikipedia.org/wiki/漢" --lang ja,en

# JSON IRから.tdslソースを逆コンパイル
tdsl decompile output.json --output restored.tdsl

# 複数ファイルをまとめてコンパイル（ファイル順にマージ）
tdsl build part1.tdsl part2.tdsl --pretty

# 複数ファイルを専用マージコマンドで結合
tdsl merge base.tdsl extensions.tdsl --output merged.json --pretty

# キャッシュの状態を確認
tdsl cache status

# 古いキャッシュエントリを削除（7日以上）
tdsl cache clear --older-than 7
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

タイトル・単位・表示範囲・暦法・カラーマッピングを宣言する。

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
    // フィールド単位のマージ戦略（任意）
    policy field_priority {
        label: manual;    // ラベルは手動定義を優先
        time:  wikidata;  // 時刻はWikidataを優先
        tags:  merge;     // タグは両方をマージ
    }
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

### template / apply 構文

共通のマッピングパターンをテンプレート化して複数のインポートに再利用できる。

```
template "王朝スパン" as dynasty_span to span {
    start claim(P571).year;
    end claim(P576).year;
    label label@ja ?? label@en;
}

apply dynasty_span to wd {
    lane han;
}
```

> `source` はインポートされたアイテムに `wd:<entity_id>` として自動付与されます。`map` ブロック内での明示指定は不要です。
> `policy` はID衝突時の挙動を切り替えます:
> `merge_by_source` は衝突をエラー扱い、`overwrite_imported` は既存のインポート済み項目のみ置換、
> `keep_manual` は既存項目（手動定義）を優先してインポート側をスキップします。

## サンプルファイル

| ファイル | 内容 |
|---|---|
| `examples/china_dynasties.tdsl` | 静的定義のみ。秦・漢・三国の年表 |
| `examples/china_with_import.tdsl` | Wikidata連携つき。秦・漢をQIDからインポート |
| `examples/template_apply_example.tdsl` | template / apply 構文のサンプル |
| `examples/fictional_empire.tdsl` | 架空世界向けの手作業年表サンプル |
| `examples/fictional_empire_items.csv` | `import-csv` 用の入力CSVサンプル |
| `examples/japanese_history.tdsl` | 日本史（奈良〜江戸）。複数lane・静的定義 |
| `examples/samurai_wikidata.tdsl` | 戦国武将の生没年。Wikidata連携（P569/P570）サンプル |
| `examples/world_wars.tdsl` | 近代戦争年表。event_range 中心の年表 |
| `examples/sci_tech_timeline.tdsl` | 科学技術の発明・発見年表。event 中心の年表 |

## GitHub Actions 連携

`uses: keroway/timeline-dsl@v1` で `.tdsl` ファイルを SVG / HTML にレンダリングできます。

```yaml
- uses: keroway/timeline-dsl@v1
  with:
    file: examples/china_dynasties.tdsl
    format: svg
    output: china.svg
    offline: 'true'
```

主なインプット:

| インプット | デフォルト | 説明 |
|---|---|---|
| `file` | — | レンダリングする `.tdsl` ファイルのパス（必須） |
| `format` | `svg` | 出力フォーマット: `svg` または `html` |
| `output` | `<basename>.<format>` | 出力ファイルパス |
| `offline` | `false` | Wikidata フェッチをスキップ（CI推奨） |
| `interactive` | `false` | インタラクティブ HTML 出力（`format: html` 時） |
| `theme` | — | テーマ: `default` / `dark` / `print` / `pastel` |
| `version` | `latest` | 使用する tdsl バージョン（例: `v1.5.0`） |

アウトプット `output_path` には生成ファイルの絶対パスが入ります。

詳細な使い方は [docs/ci-integration.md](docs/ci-integration.md) を参照してください。

## エディタサポート

### VS Code 構文ハイライト

`editors/vscode/` に VS Code 拡張があります。`.tdsl` ファイルのキーワード・文字列・コメント・QIDなどを色分けします。

**インストール方法（Marketplace）:**

VS Code 上で `Ctrl+P`（macOS: `Cmd+P`）を押して以下を実行:

```
ext install keroway.timeline-dsl
```

または [VS Code Marketplace](https://marketplace.visualstudio.com/items?itemName=keroway.timeline-dsl) から直接インストール。

**インストール方法（手動）:**

```bash
cp -r editors/vscode ~/.vscode/extensions/timeline-dsl
# VS Code を再起動
```

ハイライト対象:
- キーワード: `timeline`, `lane`, `span`, `event`, `event_range`, `import`, `map`, `template`, `apply`, `color_map`
- 文字列リテラル（ダブルクォート）
- コメント（`//` と `/* */`）
- Wikidata QID（`Q123`）・プロパティID（`P569`）・参照（`wd:Q123`）
- `claim(P571).year` 式、`label@ja` 式

## ファイルのマージ

複数の `.tdsl` ファイルを 1 つの IR に統合できます。

```bash
# tdsl build で複数ファイルを指定（ファイル順にマージ）
tdsl build base.tdsl additions.tdsl --pretty

# tdsl merge コマンドで明示的にマージ
tdsl merge base.tdsl additions.tdsl --output merged.json --pretty
```

最初のファイルの `timeline` メタ（title / range / calendar）が優先されます。`lane` は全ファイルから収集され、`item` は重複 ID を検出しながら順に追加されます。

## Lint

`tdsl lint <file> [--fix] [--format text|json]`

- 検出ルール: 未定義lane参照 / 重複id / `start > end` / 空label / タグの空要素・重複
- `--fix` 対応: タグ重複除去・空タグ除去 / `start,end` 入れ替え / `id` 未設定時の安定ID生成
- `--format json` はCI連携向けに issue 一覧と `ok` フラグを出力

## WebUI

**[今すぐブラウザで試す →](https://keroway.github.io/timeline-dsl/)**

WASM 駆動のブラウザ内エディタです。インストール不要でタイムラインの作成・プレビューができます。

### 主な機能

- **リアルタイムプレビュー**: `.tdsl` を編集するたびに SVG が即時更新される（500ms debounce）
- **診断パネル**: 構文エラー・意味エラーを行番号付きで表示
- **フォントサイズ選択**: エディタのフォントサイズを 12px〜18px から選択
- **ライト/ダークテーマ**: エディタとUIのカラースキームをワンクリックで切替
- **ファイル操作**: ローカルの `.tdsl` ファイルを開く / `.tdsl` ・ SVG ・ スタンドアロン HTML としてダウンロード
- **サンプル切替**: 複数の例文を選択して即試せる
- **ツールチップ**: SVG上にマウスを重ねるとアイテム詳細を表示

> **制限**: ブラウザ内では Wikidata インポート（`import wikidata`）は解決されません。静的な `span`・`event`・`event_range` のみプレビューされます。

## ドキュメント

- [Getting Started チュートリアル](docs/tutorial.md) — ステップバイステップのハンズオン
- [DSL 言語仕様](docs/dsl-spec.md) — 文法リファレンス
- [スタイルカスタマイズガイド](docs/styling.md) — `--theme` / `--custom-css` によるCSSカスタマイズのリファレンス
- [エラーコードカタログ](docs/error-catalog.md) — エラーメッセージの原因と修正方法
- [v0→v1 移行ガイド](docs/migration-v0-to-v1.md) — バージョンアップ時の変更点
- [WebUI 技術選定](docs/webui-design.md) — WASM + 静的サイト構成の設計記録

## アーキテクチャ

```
.tdsl ファイル
    |
    v
[tdsl-parser]   PEG文法(pest) → AST
    |
    v
[tdsl-core]     AST → IR変換（4パスパイプライン）
    |               Pass 1: timeline/lane 宣言の収集
    |               Pass 2: 静的アイテムの変換
    |               Pass 3: Wikidataインポート解決
    |               Pass 4: mapブロックの適用
    |
[tdsl-wikidata] Wikidata APIクライアント（キャッシュ付き）
    |
    v
JSON IR 出力
    |
    +--[tdsl-render]--> HTML / SVG
    +--[tdsl-wasm]  --> WebUI (WASM facade)
```

### クレート構成

| クレート | 役割 |
|---|---|
| `tdsl-parser` | PEG文法定義とAST構築 |
| `tdsl-core` | IR変換（lowering）・バリデーション・逆コンパイル |
| `tdsl-wikidata` | Wikidata HTTPクライアント・エンティティモデル・キャッシュ |
| `tdsl-render` | IR → HTML（静的・インタラクティブ）/ SVG レンダラ |
| `tdsl-wasm` | WebUI向けWASM facade（`wasm-bindgen`） |
| `tdsl-cli` | CLIバイナリ（全サブコマンド） |

## IR (中間表現) の構造

コンパイル結果のJSON IRは以下の構造を持つ。

```json
{
  "meta": {
    "title": "中国王朝年表",
    "unit": "year",
    "range": [-500, 2000],
    "calendar": "proleptic_gregorian",
    "color_map": { "dynasty": "#3366cc", "war": "#cc0000" }
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

# ベンチマーク
cargo bench --workspace
```

## ライセンス

### このソフトウェア

MIT License — 詳細は [LICENSE](./LICENSE) を参照。

### Wikidataから取得したデータ

Wikidataの構造化データは [CC0 1.0](https://creativecommons.org/publicdomain/zero/1.0/) で提供されます。
`tdsl` でWikidataデータをインポートした場合、そのデータ自体は出典表示なしで自由に利用できます。
これはこのソフトウェア（MIT）とは独立した条件です。
