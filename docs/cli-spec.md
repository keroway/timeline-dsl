# CLI サブコマンドリファレンス

`tdsl` は Timeline DSL ファイル（`.tdsl`）のコンパイル・編集・レンダリングを行うコマンドラインツールです。

```
tdsl [OPTIONS] <COMMAND>
```

## グローバルオプション

| オプション | 説明 | デフォルト |
|---|---|---|
| `--wikidata-timeout <SECONDS>` | Wikidata HTTP リクエストのタイムアウト秒数 | `30` |
| `-h, --help` | ヘルプを表示 | — |
| `-V, --version` | バージョンを表示 | — |

## 終了コード

| コード | 意味 |
|---|---|
| `0` | 正常終了 |
| `1` | エラー（パース失敗、バリデーション失敗、IO エラーなど） |

---

## サブコマンド一覧

| サブコマンド | 概要 |
|---|---|
| [`build`](#build) | `.tdsl` → IR JSON にコンパイル |
| [`merge`](#merge) | 複数 `.tdsl` ファイルを統合して IR JSON を出力 |
| [`check`](#check) | 構文・意味エラーチェック |
| [`ast`](#ast) | パース済み AST をダンプ（デバッグ用） |
| [`fetch`](#fetch) | Wikidata エンティティのデータを取得・表示 |
| [`search`](#search) | キーワードで Wikidata エンティティを検索 |
| [`inspect`](#inspect) | Wikidata エンティティを詳細解析してマッピング戦略を提案 |
| [`resolve`](#resolve) | Wikipedia 記事 URL を Wikidata QID に変換 |
| [`scaffold`](#scaffold) | Wikidata エンティティから `.tdsl` テンプレートを自動生成 |
| [`render`](#render) | `.tdsl` をスタンドアロン HTML/SVG/PNG/PDF 年表にレンダリング |
| [`init`](#init) | 手動編集用の最小 `.tdsl` テンプレートを生成 |
| [`import-csv`](#import-csv) | CSV から年表アイテムを取り込む |
| [`export-csv`](#export-csv) | IR を CSV に書き出す（`import-csv` と対称） |
| [`fmt`](#fmt) | `.tdsl` ファイルを正準フォーマット |
| [`lint`](#lint) | `.tdsl` ファイルのリントと自動修正 |
| [`cache`](#cache) | Wikidata ローカルキャッシュの管理 |
| [`decompile`](#decompile) | JSON IR を `.tdsl` ソースに逆変換 |
| [`completions`](#completions) | シェル補完スクリプトを生成 |
| [`lsp`](#lsp) | LSP サーバを stdio 経由で起動（Diagnostics + Completion + Hover + Goto Definition + Code Action + Document Symbols + Find References + Rename + Formatting） |

---

## `build`

`.tdsl` ファイルを IR JSON にコンパイルします。複数ファイルを指定するとマージして出力します。

```
tdsl build [OPTIONS] [FILE]...
```

### 引数

| 引数 | 説明 |
|---|---|
| `[FILE]...` | 入力 `.tdsl` ファイルのパス（複数指定時は順番にマージ）。`--json-schema` を指定する場合は省略可能 |

### オプション

| オプション | 説明 | デフォルト |
|---|---|---|
| `-o, --output <OUTPUT>` | 出力 JSON ファイルのパス | 標準出力 |
| `--pretty` | JSON を整形出力 | — |
| `--offline` | Wikidata フェッチをスキップし静的アイテムのみ処理 | — |
| `--no-cache` | ローカルキャッシュをバイパスして API を直接呼び出す | — |
| `--cache-ttl <CACHE_TTL>` | キャッシュ有効期限（秒）、0 で無効化 | `86400`（24h）|
| `--json-schema` | `TimelineIr` の JSON Schema を標準出力する（入力ファイル不要） | — |

### 実行例

```bash
# オフラインでコンパイルし整形 JSON を表示
tdsl build examples/china_dynasties.tdsl --pretty

# Wikidata 連携ありでコンパイルしファイルに保存
tdsl build examples/china_with_import.tdsl --output out.json --pretty

# オフラインビルド（開発時に推奨）
tdsl build examples/china_with_import.tdsl --offline --pretty

# 複数ファイルをマージしてコンパイル
tdsl build part1.tdsl part2.tdsl --output merged.json --pretty

# TimelineIr の JSON Schema を標準出力（入力ファイル不要）
tdsl build --json-schema

# JSON Schema を整形してファイルに保存
tdsl build --json-schema --pretty --output timeline-ir.schema.json
```

---

## `merge`

複数の `.tdsl` ファイルを読み込み、統合した IR JSON を出力します。最初のファイルのメタ情報（タイトル・単位・範囲）が優先されます。

```
tdsl merge [OPTIONS] <FILE> <FILE>...
```

### 引数

| 引数 | 説明 |
|---|---|
| `<FILE> <FILE>...` | 入力 `.tdsl` ファイルのパス（2 ファイル以上必須、順番にマージ） |

### オプション

`build` と同じオプションセット（`--output`, `--pretty`, `--offline`, `--no-cache`, `--cache-ttl`）。

### 実行例

```bash
# 2 ファイルをマージして整形出力
tdsl merge china_dynasties.tdsl world_wars.tdsl --pretty

# ファイルに保存
tdsl merge base.tdsl extension.tdsl --output combined.json --pretty
```

---

## `check`

`.tdsl` ファイルの構文エラーおよび意味エラー（lane 未定義参照、date 範囲矛盾など）を確認します。エラーがなければ終了コード 0 を返します。

```
tdsl check [OPTIONS] <FILE>...
```

### 引数

| 引数 | 説明 |
|---|---|
| `<FILE>...` | 入力 `.tdsl` ファイル、またはディレクトリ（**再帰的に `*.tdsl` を探索**）。複数指定可 |
| `--offline` | Wikidata 解決を行わないことを明示する（現時点では唯一の動作。付けても付けなくても挙動は同じ） |
| `--format {text,json}` | 出力形式（既定 `text`）。`json` は `code` / `severity` / `line` / `message` を含む機械可読な診断を標準出力へ出す |
| `--deny-warnings` | 警告があれば非ゼロ終了する。既定では警告のみなら成功（CI で「警告ゼロ」を強制したい場合に使う） |

**診断コード**

警告には `docs/error-catalog.md` に対応する安定したコードが付く。CI で特定の警告だけを許容/禁止する判断に使える。

```
Warning [W204] line 2: Lane "a" uses unknown kind: unknown_kind (...)
Warning [W202] line 3: Span "s1" has start (2005) > end (2001)
```

`--format json` は `lint --format json` と同じ形（`file` / `ok` / カウント + 診断の配列）に寄せてある。複数入力時は配列、単一入力時はオブジェクト。

```json
{
  "file": "timeline.tdsl",
  "lanes": 1,
  "items": 2,
  "unresolved_blocks": 0,
  "warning_count": 3,
  "ok": false,
  "diagnostics": [
    { "code": "W202", "severity": "warning", "line": 3, "message": "Span \"s1\" has start (2005) > end (2001)" }
  ]
}
```

**複数入力の扱い（`check` / `lint` / `fmt` 共通）**

- ディレクトリを渡すと配下の `*.tdsl` を再帰的に処理する
- **1 件でも失敗すれば非ゼロ終了する。** ただし最初の失敗で打ち切らず全件処理してから要約を出す（CI では「どのファイルが落ちたか」を一度に知りたいため）
- 対象が 2 件以上のときは `=== <path> ===` の見出しを付ける。単一ファイル指定時の出力は従来どおり
- 処理順はパス名でソートする（ファイルシステムの列挙順に依存させない）
- **対象が 0 件ならエラー。** パスの打ち間違いが「問題なし」として通らないようにする
- glob 展開はシェルに任せる。**`**` の再帰展開はシェル依存**で、POSIX sh では展開されず、bash も `shopt -s globstar` が必要（zsh は既定で有効）。シェルに依存せず再帰したい場合は**ディレクトリを直接渡す**のが確実:

```sh
# 確実（ディレクトリを渡して再帰探索させる）
tdsl check src

# シェル依存（zsh は可、bash は shopt -s globstar が必要、POSIX sh は不可）
tdsl check src/**/*.tdsl

# find を使う場合
find src -name '*.tdsl' -exec tdsl check {} +
```

`check` は lowering の Pass 1/2 のみを実行し、**import 解決（Pass 3）と map 適用（Pass 4）は行わない**。
`import` / `map` / `apply` を含むファイルではアイテムが生成されないため、その旨を警告と完了行の両方に出す。

```
Warning: 1 import block(s) and 1 map block(s) were not resolved (offline lowering); ...
OK: 1 lanes, 0 items (2 block(s) unresolved: offline lowering does not run import/map)
```

### 実行例

```bash
# 構文・意味チェック
tdsl check examples/china_dynasties.tdsl

# CI で使う（エラー時にゼロ以外の終了コードを返す）
tdsl check my_timeline.tdsl && echo "OK"
```

---

## `ast`

`.tdsl` ファイルをパースして AST（抽象構文木）を標準出力にダンプします。文法デバッグや Lowering の調査に使います。

```
tdsl ast [OPTIONS] <FILE>
```

### 引数

| 引数 | 説明 |
|---|---|
| `<FILE>` | 入力 `.tdsl` ファイルのパス |

### 実行例

```bash
# AST をダンプ
tdsl ast examples/china_dynasties.tdsl

# ページャで確認
tdsl ast examples/china_with_import.tdsl | less
```

---

## `fetch`

Wikidata エンティティ（QID 指定）のラベル・説明・プロパティを取得して表示します。`import` ブロックを書く前に、対象エンティティのデータ確認に使います。

```
tdsl fetch [OPTIONS] <QID>
```

### 引数

| 引数 | 説明 |
|---|---|
| `<QID>` | Wikidata QID（例: `Q7209`） |

### オプション

| オプション | 説明 | デフォルト |
|---|---|---|
| `-l, --lang <LANG>` | ラベルを取得する言語（カンマ区切り） | `ja,en` |

### 実行例

```bash
# 漢（前漢）の情報を取得
tdsl fetch Q7209

# 英語・フランス語ラベルで取得
tdsl fetch Q7209 --lang en,fr
```

---

## `search`

キーワードで Wikidata エンティティを検索し、候補 QID の一覧を表示します。`import` に使う QID を探す際に利用します。

```
tdsl search [OPTIONS] <QUERY>
```

### 引数

| 引数 | 説明 |
|---|---|
| `<QUERY>` | 検索クエリ（例: `"漢王朝"`） |

### オプション

| オプション | 説明 | デフォルト |
|---|---|---|
| `-l, --lang <LANG>` | Wikidata 検索に使う言語 | `ja` |
| `-n, --limit <LIMIT>` | 最大取得件数（1〜50） | `10` |
| `--json` | JSON 形式で出力 | — |

### 実行例

```bash
# 日本語で「漢王朝」を検索
tdsl search "漢王朝"

# 英語で検索し件数を増やす
tdsl search "Han dynasty" --lang en --limit 20

# JSON で取得してスクリプトに渡す
tdsl search "samurai" --json | jq '.[] | .id'
```

---

## `inspect`

Wikidata エンティティを詳細解析し、年表へのマッピング戦略（どのプロパティを `start`/`end` に使うか等）を提案します。`scaffold` 実行前の事前調査に有効です。

```
tdsl inspect [OPTIONS] <QID>
```

### 引数

| 引数 | 説明 |
|---|---|
| `<QID>` | Wikidata QID（例: `Q7209`） |

### オプション

| オプション | 説明 | デフォルト |
|---|---|---|
| `-l, --lang <LANG>` | ラベル取得のフォールバック言語（カンマ区切り） | `ja,en` |
| `--json` | JSON 形式で出力 | — |

### 実行例

```bash
# 徳川家康のエンティティを解析
tdsl inspect Q7243

# JSON で出力してスクリプト処理
tdsl inspect Q7243 --json | jq '.suggestions'
```

---

## `resolve`

Wikipedia 記事の URL を Wikidata QID に変換します。記事を見つけたが QID が不明な場合に使います。

```
tdsl resolve [OPTIONS] <URL>
```

### 引数

| 引数 | 説明 |
|---|---|
| `<URL>` | Wikipedia 記事 URL |

### オプション

| オプション | 説明 | デフォルト |
|---|---|---|
| `-l, --lang <LANG>` | ラベル取得のフォールバック言語（カンマ区切り） | `ja,en` |
| `--json` | JSON 形式で出力 | — |

### 実行例

```bash
# 記事 URL から QID を取得
tdsl resolve "https://ja.wikipedia.org/wiki/%E6%BC%A2"

# JSON で出力
tdsl resolve "https://en.wikipedia.org/wiki/Han_dynasty" --json
```

---

## `scaffold`

Wikidata エンティティから `.tdsl` テンプレートを自動生成します。サブコマンド `wikidata` を指定します。

```
tdsl scaffold wikidata [OPTIONS] --qids <QIDS> --timeline <TIMELINE>
```

### `scaffold wikidata` オプション

| オプション | 説明 | デフォルト |
|---|---|---|
| `--qids <QIDS>` | カンマ区切りの QID リスト（例: `Q7183,Q7209`）【必須】 | — |
| `--timeline <TIMELINE>` | 年表の表示タイトル【必須】 | — |
| `-o, --output <OUTPUT>` | 出力 `.tdsl` ファイルのパス | 標準出力 |
| `-l, --lang <LANG>` | ラベル取得のフォールバック言語（カンマ区切り） | `ja,en` |
| `--target <TARGET>` | マッピングターゲット戦略 | `auto` |
| `--lane-mode <LANE_MODE>` | レーン割り当て戦略 | `per-entity` |
| `--single-lane-label <LABEL>` | `lane-mode=single` 時の共有レーン名 | `項目` |

**`--target` の選択肢:** `auto` / `span` / `event` / `event-range`

**`--lane-mode` の選択肢:** `single` / `per-entity` / `by-kind`

### 実行例

```bash
# 前漢・後漢を自動マッピングでスキャフォールド（Wikidata 連携）
tdsl scaffold wikidata \
  --qids "Q7209,Q8209" \
  --timeline "漢王朝年表" \
  --output han_dynasties.tdsl

# 全エンティティを単一レーンにまとめる
tdsl scaffold wikidata \
  --qids "Q7209,Q8209" \
  --timeline "漢王朝年表" \
  --lane-mode single \
  --single-lane-label "王朝"

# span として強制マッピング
tdsl scaffold wikidata \
  --qids "Q7183,Q7209,Q8209" \
  --timeline "漢・新・後漢" \
  --target span
```

---

## `render`

`.tdsl` ファイルをスタンドアロンな HTML / SVG / PNG / PDF 年表にレンダリングします。

```
tdsl render [OPTIONS] <FILE>
```

### 引数

| 引数 | 説明 |
|---|---|
| `<FILE>` | 入力 `.tdsl` ファイルのパス |

### オプション

| オプション | 説明 | デフォルト |
|---|---|---|
| `-o, --output <OUTPUT>` | 出力ファイルのパス | 標準出力 |
| `--format <FORMAT>` | 出力フォーマット（`html` / `svg` / `png` / `pdf`） | `html` |
| `--scale <SCALE>` | 横軸の 1 年あたりピクセル数 | `2` |
| `--lane-height <LANE_HEIGHT>` | 各レーンの高さ（px） | `60` |
| `--left-gutter <LEFT_GUTTER>` | レーンラベル用の左ガター幅 | `120` |
| `--top-margin <TOP_MARGIN>` | 時間軸のトップマージン | `40` |
| `--theme <THEME>` | 配色テーマ（`default` / `dark` / `print` / `pastel`） | `default` |
| `--custom-css <CUSTOM_CSS>` | テーマ CSS の後に注入するカスタム CSS ファイルのパス | — |
| `--dpi <DPI>` | PNG 出力の解像度（dpi/96 倍のピクセルサイズ）。`--format png` のみ有効。`--png-scale` と排他 | `96` |
| `--png-scale <PNG_SCALE>` | PNG 出力の固定ピクセル倍率（例: `2.0` で 2 倍）。`--format png` のみ有効。`--dpi` と排他 | — |
| `--interactive` | ズーム・パン・検索・凡例・詳細パネルを有効化 | — |
| `--offline` | Wikidata フェッチをスキップ | — |
| `--no-cache` | ローカルキャッシュをバイパス | — |
| `--cache-ttl <CACHE_TTL>` | キャッシュ有効期限（秒） | `86400`（24h）|
| `--color-map <COLOR_MAP>` | タグ→色マッピング（例: `war=#cc0000,dynasty=#3366cc`） | — |
| `--orientation <ORIENTATION>` | 時間軸方向（`horizontal` / `vertical`） | `horizontal` |
| `--grid <GRID>` | 補助グリッド線（`none` / `decade` / `year` / `month`）。`none` でグリッド無効（デフォルト）。`decade` = 10年ごと、`year` = 1年ごと、`month` = 月ごとの薄い補助線を描画 | `none` |
| `--layout-style <LAYOUT_STYLE>` | 高レベルレイアウトスタイル（`timeline` / `group-bands` / `gantt` / `zigzag`）。`--orientation` と直交。`timeline` = 通常レイアウト（デフォルト）。`group-bands` = レーンの `group` が連続する箇所に背景帯を描画（#543）。`gantt` = 月単位グリッドを強調表示（`--grid` 未指定時は自動で月グリッド相当を強制）し、`span` / `event_range` のバーに開始〜終了日の期間ラベルを常時表示するプロジェクト管理向けスタイル。同一レーン内で近接するラベルは自動的にスタッキングされ重なりを回避する（#564）。`zigzag` = レーン内アイテムを開始時刻順に上下交互に配置（#565）。レーン数が 2 以下のときのみ有効。それ以上の場合は描画前に明示的なエラーを返す（フォールバックしない）。`gantt` / `zigzag` はいずれも #549（バーの重なり回避サブ行スタッキング）とは異なるレイアウト軸であり、`zigzag` は特に #549 と互いに排他 | `timeline` |
| `--watch` | 入力ファイルの変更を監視し、変更検出のたびに自動再レンダリングする。`--output` が必須。`html` / `svg` のみ対応（`png` / `pdf` は非対応） | — |
| `--show-table` | 内容一覧の表を追加する（時期・ラベル・レーン・タグ列、時系列順）。`--format html` では HTML `<table>` として追加され、`svg` / `png` / `pdf` ではタイムライン本体の下に SVG `<rect>`/`<text>` で同等の表が描画される（#536）。デフォルトの PDF 出力（`--pdf-pagination` 未指定）は従来と同じ単一ページベクトル出力のままで、表を含む全体をページに収まるように拡大縮小する（ADR-0002 の単一ベクトルPDF方式）。`--pdf-pagination` と併用するとテーブルのみ複数ページに分割される（ADR-0004） | — |
| `--show-event-labels` | イベント（`event` / `event_range`）のドット・バー近傍にラベルテキストを常時描画する。デフォルト無効（ホバー時のツールチップのみ） | — |
| `--pdf-size <SIZE>` | PDF 用紙サイズ（`a4` / `a3` / `letter`）。`--format pdf` のみ有効 | `a4` |
| `--pdf-landscape` | PDF を横向き（landscape）で出力する。`--format pdf` のみ有効 | — |
| `--pdf-margin <MM>` | PDF の用紙マージン（mm）。`--format pdf` のみ有効 | `10` |
| `--pdf-title <TITLE>` | PDF ドキュメントの Title メタデータを上書きする（未指定時は年表タイトルを使用）。`--format pdf` のみ有効 | — |
| `--pdf-pagination` | アイテムテーブルを用紙サイズ・余白に収まる行数ごとに複数ページへ分割する（ADR-0004）。`--chart-pagination` 未指定時は 1 ページ目が従来どおりタイムライン本体（縮小描画）のみで、2 ページ目以降にテーブルを分割描画する。`--chart-pagination` 併用時のページ構成は `--chart-pagination` の説明を参照（issue #661）。各テーブルページの先頭に列見出しを再描画し、フッタに `i / N` 形式のページ番号を付与する（`N` はテーブルページ数のみを数えたもので、先行するチャートページは含まない）。opt-in（デフォルト無効）で、既存の単一ページ出力は変更されない。`--show-table` が指定されていない場合はエラー（分割対象のテーブルが存在しないため、silent no-op にはしない）。`--format pdf` のみ有効。既存のタイムライン描画オプションとの相互作用（ADR-0004 D5）: `--show-legend` はタイムラインページのみに描画されテーブルページには影響しない。`--layout-style group-bands` / `gantt` / `zigzag`、および open-ended range（`now` 終了）はいずれもタイムライン本体の描画にのみ関わり、本フラグの有効/無効によってタイムラインページの描画内容が変わることはない（`--chart-pagination` 未指定時はページ分割はテーブルのみが対象） | — |
| `--chart-pagination <N>` | タイムライン本体（チャート部分）を lane グループ単位で複数ページに分割する（issue #660/#661, ADR-0005 D2）。`N` は 1 ページあたりの lane 数。時間軸（`meta.range`）は全ページ共通のため、`Item::lane` が単一 lane を持つ構造上、span/event_range のページ境界クリッピングは発生しない。`--show-legend` は各チャートページに個別描画される。`--format svg` と `--format pdf` の両方で有効（`html`/`png` との併用は明示エラー）。`--output <path>` は必須。`0` はエラー。`--watch` との併用も明示エラー。`--chart-pagination-range` との併用も明示エラー（両軸の直積分割は非対応）。lane の `group` がページ境界をまたいで分断される場合は `stderr` に `Warning: group band "..." is split across chart pages; ...` を出力し、出力自体は生成する（silent no-op にはしない）。<br>**`--format svg` の場合**: `<stem>.pageN.<ext>`（`N` は総ページ数の桁数に0埋め、例: 10ページ以上なら `page01`）ごとに別ファイルとして書き出される。`--show-table` を指定した場合、チャートページ群の**後ろに専用のテーブルページを 1 枚**追加し、**IR 全体**（最後のチャートページの lane に限らない）の item を一覧表示する（このテーブルページの `i / N` フッタは `1 / 1` 固定。複数テーブルページへの分割は SVG では未対応）。<br>**`--format pdf` の場合**（issue #661）: `<stem>.pageN.svg` のような複数ファイルには分割されず、1 つの PDF ファイル内に複数ページとして出力される。ページ構成は「チャートページ群（lane グループ順）→ テーブルページ群」の順で固定。`--show-table` を指定しない場合はテーブルページなし。`--show-table` のみ指定（`--pdf-pagination` なし）の場合は IR 全体を 1 枚の未分割テーブルページとしてチャートページ群の末尾に追加する。`--show-table --pdf-pagination` を併用した場合は `--pdf-pagination` の行分割ロジックでテーブルページ群を生成し、その `i / N` フッタはテーブルページ数のみを数える（先行するチャートページ数を含めない）。ADR-0004 D3 の後方互換制約により、`--chart-pagination` を指定しない既存の `--format pdf` 出力（単体 / `--show-table` / `--pdf-pagination` のいずれも）は本フラグ追加後も完全に不変 | 無効（単一ページ） |
| `--chart-pagination-range <N>` | タイムライン本体（チャート部分）を時間範囲軸で `N` ページに分割する（issue #733/#736, ADR-0005 D3）。`meta.range` を `N` 個の連続する非空の整数年区間へ均等分割し、区間ごとに1ページを描画する（各ページの `meta` はサブ年精度フィールド〈`range_start_month` 等12個〉をクリアするため、月日/秒精度の `range` を持つ `.tdsl` でもページの目盛は整数年表示になる。元ファイルの宣言精度自体は変更されない）。区間境界をまたぐ `span`/`event_range` は既存のクランプ処理で各ページ側にクリップされ、クリップされた辺に継続マーカー（三角形の `<polygon>`。`role="img"` の `aria-label`・`<title>` 付き。CSS フック `tdsl-item-continues-from-previous-page` / `tdsl-item-continues-to-next-page` / `tdsl-continuation-marker-*` で上書き可能。issue #734, ADR-0005 §2 戦略1「クリップ + 継続マーカー」）を描画し、`stderr` に `Warning: item "..." (...) is clipped at chart page boundary year(s) [...]; ...` を出力する（silent no-op にはしない）。`--format svg` と `--format pdf` の両方で有効（`html`/`png` との併用は明示エラー）。`--output <path>` は必須。`0` はエラー。`--watch` との併用、`--chart-pagination` との併用はいずれも明示エラー。<br>**`--format svg` の場合**: `<stem>.pageN.<ext>` ごとに別ファイルとして書き出される点は `--chart-pagination` の SVG 出力と同じ。<br>**`--format pdf` の場合**（issue #736）: `--chart-pagination` の PDF 出力と同じページ構成規則に従う（チャートページ群 → テーブルページ群の順、`--show-table` のみなら未分割テーブルページ1枚、`--show-table --pdf-pagination` なら行分割されたテーブルページ群、footer はテーブルページ数のみを数える）。`--chart-pagination-range` を指定しない既存の `--format pdf` 出力は本フラグ追加後も完全に不変 | 無効（単一ページ） |

### 実行例

```bash
# HTML にレンダリング（オフライン）
tdsl render examples/china_dynasties.tdsl --output china.html

# ダークテーマで SVG に出力
tdsl render examples/china_dynasties.tdsl --format svg --theme dark --output china.svg

# PNG にラスタライズして出力（resvg ベース、デフォルト DPI 96）
tdsl render examples/china_dynasties.tdsl --format png --output china.png

# 300 DPI の高解像度 PNG（印刷向け）
tdsl render examples/china_dynasties.tdsl --format png --dpi 300 --output china_hires.png

# 2 倍の固定スケールで PNG 出力（--dpi と排他）
tdsl render examples/china_dynasties.tdsl --format png --png-scale 2.0 --output china_2x.png

# ベクター PDF として出力（svg2pdf 経由）
tdsl render examples/china_dynasties.tdsl --format pdf --output china.pdf

# A3 横向き・マージン 15mm で PDF を出力
tdsl render examples/china_dynasties.tdsl --format pdf --pdf-size a3 --pdf-landscape --pdf-margin 15 --output china_a3.pdf

# PDF タイトルメタデータを明示指定
tdsl render examples/china_dynasties.tdsl --format pdf --pdf-title "中国王朝年表" --output china.pdf

# アイテムテーブルを複数ページに分割（ADR-0004、--show-table 必須。1ページ目はタイムライン本体のみ）
tdsl render examples/china_dynasties.tdsl --format pdf --show-table --pdf-pagination --output china_paginated.pdf

# タイムライン本体を lane グループ単位で複数の SVG ページに分割（issue #660, ADR-0005 D2）
# china.page1.svg / china.page2.svg ... が生成される
tdsl render examples/china_dynasties.tdsl --format svg --chart-pagination 2 --output china.svg

# タイムライン本体を lane グループ単位で複数の PDF ページに分割（issue #661）
# 単一の china_chart.pdf 内に複数ページとして出力される（テーブルなし）
tdsl render examples/china_dynasties.tdsl --format pdf --chart-pagination 2 --output china_chart.pdf

# チャートページ分割 + テーブルページ分割を併用（チャートページ群 → テーブルページ群の順）
tdsl render examples/china_dynasties.tdsl --format pdf --chart-pagination 2 --show-table --pdf-pagination --output china_full_paginated.pdf

# インタラクティブモードで HTML を生成
tdsl render examples/china_dynasties.tdsl --interactive --output china_interactive.html

# カスタム CSS を注入
tdsl render examples/china_dynasties.tdsl --custom-css my_style.css --output china.html

# タグ→色マッピングを指定
tdsl render examples/china_dynasties.tdsl \
  --color-map "dynasty=#4b7bec,war=#e74c3c" \
  --output china.html

# Wikidata 連携ありでレンダリング
tdsl render examples/china_with_import.tdsl --output china_wd.html

# 10年グリッド線付きで SVG に出力
tdsl render examples/china_dynasties.tdsl --format svg --grid decade --output china_grid.svg

# 年単位グリッド線付きで HTML に出力（短期年表向け）
tdsl render examples/world_wars.tdsl --grid year --output wars_grid.html

# レーン group 単位の背景帯付きで HTML に出力
tdsl render examples/china_dynasties.tdsl --layout-style group-bands --output china_bands.html

# Gantt スタイル（月グリッド強調 + 期間ラベル常時表示）で HTML に出力
tdsl render examples/china_dynasties.tdsl --layout-style gantt --output china_gantt.html

# Zigzag スタイル（レーン内アイテムを上下交互に配置、単一/少数レーン向け）
tdsl render examples/apollo_11.tdsl --layout-style zigzag --output apollo_zigzag.html

# ファイル変更を監視して自動再レンダリング（Ctrl+C で終了）
tdsl render examples/china_dynasties.tdsl --watch --output china.html

# SVG でウォッチモード
tdsl render examples/china_dynasties.tdsl --watch --format svg --output china.svg
```

---

## `init`

手動編集用の最小 `.tdsl` テンプレートを生成します。Wikidata 接続は不要です。

```
tdsl init [OPTIONS]
```

### オプション

| オプション | 説明 | デフォルト |
|---|---|---|
| `-o, --output <OUTPUT>` | 出力 `.tdsl` ファイルのパス | 標準出力 |
| `--timeline <TIMELINE>` | 年表の表示タイトル | `新しい年表` |
| `--range-start <RANGE_START>` | 範囲開始年 | `0` |
| `--range-end <RANGE_END>` | 範囲終了年 | `2000` |
| `--lanes <LANES>` | レーンラベル（カンマ区切り、例: `"王朝,事件,人物"`） | `""` |

### 実行例

```bash
# 最小テンプレートを生成（標準出力）
tdsl init

# ファイルに保存してレーンを指定
tdsl init \
  --output my_timeline.tdsl \
  --timeline "架空世界年表" \
  --range-start 1000 \
  --range-end 1500 \
  --lanes "王国,事件,人物"
```

---

## `import-csv`

CSV ファイルから年表アイテムを読み込み、`.tdsl` スニペットに変換します。CSV のヘッダ行に `lane,type,start,end,time,label,tags,id` を含める必要があります。
任意で `source` / `origin` 列も受理し、`export-csv` が出力する 10 列の CSV をそのまま往復できます（#608）。

```
tdsl import-csv [OPTIONS] <CSV>
```

### 引数

| 引数 | 説明 |
|---|---|
| `<CSV>` | 入力 CSV ファイルのパス（UTF-8、ヘッダ行あり） |

### CSV 列仕様

| 列名 | 必須 | 説明 |
|---|---|---|
| `lane` | ○ | アイテムを配置するレーンの ID |
| `type` | ○ | アイテム種別（`span` / `event` / `event_range`） |
| `start` | `span`/`event_range` | 開始時刻 |
| `end` | `span`/`event_range` | 終了時刻 |
| `time` | `event` | 発生時刻 |
| `label` | ○ | 表示ラベル |
| `tags` | — | タグ（カンマ区切り） |
| `id` | — | アイテム ID（省略時は自動採番） |
| `source` | —（任意） | 出典参照。`<ident>:<QID>` 形式（例 `wd:Q7209`）。空欄可（#608） |
| `origin` | —（任意） | 由来。DSL の `ident` 文法を満たす値（例 `wikidata`）。空欄可（#608） |

`source` / `origin` は両列とも独立して任意（旧8列形式の CSV も引き続き受理される）。
`origin=wikidata` の行は `source` が `wd:Q<id>` 形式であることを必須とし（#608 provenance 契約）、
不整合は CSV 行番号付きのエラーで拒否されます（silent に破棄しない）。
`wd:Q…` ソースで `origin` が `wikidata` 以外（または空欄）の場合は、static provenance としてそのまま保持されます（書き換えない）。

### 時刻リテラル

`start` / `end` / `time` 列は、年・月・日・時分精度を判別パースします。`.tdsl` 本文と同じ表記が利用できます。

| 表記 | 例 | 精度 |
|---|---|---|
| `YYYY-MM-DDTHH:MM` | `1969-07-20T20:17` | 分 |
| `YYYY-MM-DD` | `1969-07-20` | 日 |
| `YYYY-MM` | `1939-09` | 月 |
| `YYYY` | `1180` / `-206` | 年 |

- 紀元前の月日・時分精度も許可する（例: `-0206-01`, `-0206-01-15`）。
- 月の範囲は 1〜12、日の範囲は 1〜31（カレンダー妥当性の細かな検証は lowering 側で行う）。
- 不正フォーマット時は CSV 行番号付きで「`time must be YYYY-MM-DDTHH:MM, YYYY-MM-DD, YYYY-MM, or YYYY (got`2020-13-01`): ...`」のように原因が表示される。

### `source` / `origin` リテラル

任意列 `source` は `<ident>:<QID>`（例 `wd:Q7209`）、`origin` は DSL の `ident` 文法（先頭英字/`_`、
以降英数字/`_`/`-`）を満たす値を受理する。不正フォーマット時は CSV 行番号付きで拒否される。

```bash
# source 列の形式不正
$ tdsl import-csv items.csv
Error: CSV row 3: source must be `<ident>:<QID>` (e.g. `wd:Q7209`), got `badvalue`: ...

# origin=wikidata なのに source が wd:Q<id> 形式でない
$ tdsl import-csv items.csv
Error: CSV row 5: origin=wikidata requires a source column value in the form `wd:Q<id>`
```

### オプション

| オプション | 説明 | デフォルト |
|---|---|---|
| `-o, --output <OUTPUT>` | 出力 `.tdsl` スニペットのパス | 標準出力 |
| `--append <APPEND>` | 生成アイテムを既存 `.tdsl` ファイルに追記 | — |

### 実行例

```bash
# CSV を .tdsl スニペットに変換（標準出力）
tdsl import-csv items.csv

# ファイルに保存
tdsl import-csv items.csv --output items_snippet.tdsl

# 既存ファイルに追記
tdsl import-csv new_items.csv --append my_timeline.tdsl
```

**CSV 例:**

```csv
lane,type,start,end,time,label,tags,id,source,origin
dynasty,span,-206,9,,"前漢",dynasty,han_early,,
events,event,,,221,"秦の統一",unification,qin_unify,,
war,event_range,-206,-202,,"楚漢戦争",war,chuhan_war,,
mission,event,,,1969-07-20,"アポロ11号着陸",space,event:apollo,wd:Q43653,wikidata
ww2,span,1939-09-01,1945-09-02,,"第二次世界大戦",war,span:ww2,,
```

（`source` / `origin` 列は省略可能。旧8列のみの CSVも引き続き受理されます）。

---

## `export-csv`

IR を CSV に書き出します。`import-csv` と対称な往復（ラウンドトリップ）を目的とし、出力は
**IR を単一の真実源**として生成されます（パーサ出力には直接依存しません）。

```
tdsl export-csv [OPTIONS] <FILE>
```

### 引数

| 引数 | 説明 |
|---|---|
| `<FILE>` | 入力ファイル。`.tdsl` ソース（lowering して IR 化）または `.json`（IR を直接読み込み） |

### CSV 列仕様

ヘッダは `lane,type,start,end,time,label,tags,id,source,origin` の 10 列です。`source` / `origin`
を含めて全 10 列が `import-csv` でラウンドトリップされます（#608）。

| 列名 | 説明 |
|---|---|
| `lane` | レーン ID |
| `type` | アイテム種別（`span` / `event` / `event_range`） |
| `start` / `end` | `span`/`event_range` の開始・終了時刻（`event` では空欄） |
| `time` | `event` の発生時刻（`span`/`event_range` では空欄） |
| `label` | 表示ラベル |
| `tags` | タグ（`|`区切り。`import-csv` は `|` と `,` の両方を受理） |
| `id` | アイテム ID |
| `source` | 出典（例 `wd:Q1`）。空欄可。`import-csv` で往復保持（#608） |
| `origin` | 由来（例 `wikidata`）。空欄可。`import-csv` で往復保持（#608） |

時刻は `YYYY` / `YYYY-MM` / `YYYY-MM-DD` / `YYYY-MM-DDTHH:MM` で出力されます（紀元前の月日・時分精度も保持し、`import-csv` と整合）。

### オプション

| オプション | 説明 | デフォルト |
|---|---|---|
| `-o, --output <OUTPUT>` | 出力 CSV ファイルのパス | 標準出力 |
| `--offline` | Wikidata 取得をスキップ（静的アイテムのみ）。`.json` 入力では無視 | `false` |
| `--no-cache` | ローカルキャッシュをバイパスして再取得 | `false` |
| `--cache-ttl <SECONDS>` | キャッシュ TTL（秒。0 で無効） | `86400` |

### 実行例

```bash
# .tdsl を CSV に書き出し（標準出力、静的のみ）
tdsl export-csv my_timeline.tdsl --offline

# ファイルに保存
tdsl export-csv my_timeline.tdsl --offline --output items.csv

# IR JSON から書き出し
tdsl build my_timeline.tdsl --offline --output ir.json
tdsl export-csv ir.json --output items.csv

# import-csv との往復（10 列全て（source/origin 含む）が意味的に同値、#608）
tdsl export-csv my_timeline.tdsl --offline --output items.csv
tdsl import-csv items.csv
```

---

## `lint`

`.tdsl` ファイルの品質チェックを実施し、自動修正可能な問題を `--fix` で修正します。

```
tdsl lint [OPTIONS] <FILE>
```

### 引数

| 引数 | 説明 |
|---|---|
| `<FILE>` | 入力 `.tdsl` ファイルのパス |

### オプション

| オプション | 説明 | デフォルト |
|---|---|---|
| `--fix` | 安全な修正をファイルに直接適用 | — |
| `--format <FORMAT>` | 出力フォーマット（`text` / `json`） | `text` |

### 実行例

```bash
# リントチェックのみ
tdsl lint examples/china_dynasties.tdsl

# 自動修正を適用
tdsl lint examples/china_dynasties.tdsl --fix

# CI 向けに JSON 出力
tdsl lint examples/china_dynasties.tdsl --format json
```

### 終了コード

下表は**リント判定まで到達した場合**の終了コード。入力の読み取り失敗・パースエラー・
`--fix` の書き込み失敗はリント判定の前に失敗するため、いずれも 1 を返す。

| 条件 | 終了コード |
|---|---|
| 入力の読み取り / パース / `--fix` の書き込みに失敗 | 1 |
| リント判定に到達し、issue なし | 0 |
| リント判定に到達し、WARN のみ | 0 |
| リント判定に到達し、**ERROR が 1 件以上** | **1** |

`--fix` を指定した場合は**修正適用後に残った** issue で判定する。
`--format json` でも同じ（JSON は標準出力に出したうえで終了コードを返す）。

ERROR で非ゼロを返すため、**JSON をパースしなくても CI のゲートにできる**。
`fmt --check` が未整形時に 1 を返すのと同じ扱い（issue #766 以前は ERROR でも 0 を返しており、
両者が非一貫だった）。

WARN で落とすかどうかは `check` の `--deny-warnings` 提案（issue #748）と揃えて別途決める。

---

## `fmt`

`.tdsl` ファイルを正準スタイル（2 スペースインデント・ブロック間空行 1 行）にフォーマットします。
デフォルトでは整形結果を標準出力に出力します。`--write` でファイルを上書き、`--check` で CI 用の差分チェックができます。

> **コメント**: `tdsl fmt` はコメント（`//`・`/* */`）を保持します。トップレベルの leading / trailing コメントは位置を保ち、ブロック内部コメントは内容を保持したまま正準位置へ移動される場合があります。

```
tdsl fmt [OPTIONS] <FILE>
```

### 引数

| 引数 | 説明 |
|---|---|
| `<FILE>` | 入力 `.tdsl` ファイルのパス |

### オプション

| オプション | 説明 | デフォルト |
|---|---|---|
| `--check` | フォーマットが必要な場合に非ゼロ終了する（ファイルは変更しない）。CI 向け | — |
| `--write` | 整形結果でファイルを上書きする | — |

`--check` と `--write` は同時に指定できません。

### 実行例

```bash
# 整形結果を標準出力に表示
tdsl fmt examples/china_dynasties.tdsl

# ファイルを上書き
tdsl fmt examples/china_dynasties.tdsl --write

# CI でフォーマット差分チェック（差分があれば exit 1）
tdsl fmt examples/china_dynasties.tdsl --check
```

---

## `cache`

Wikidata 取得結果のローカルキャッシュ（`~/.cache/tdsl/`）を管理します。サブコマンド `status` または `clear` を指定します。

```
tdsl cache <COMMAND>
```

### `cache status`

キャッシュの統計情報（ファイル数・合計サイズ・最古/最新エントリ）を表示します。

```bash
tdsl cache status
```

### `cache clear`

キャッシュエントリを削除します。

```
tdsl cache clear [OPTIONS]
```

| オプション | 説明 | デフォルト |
|---|---|---|
| `--older-than <DAYS>` | 指定日数より古いエントリのみ削除 | —（全件削除） |

### 実行例

```bash
# キャッシュ統計を表示
tdsl cache status

# 全キャッシュを削除
tdsl cache clear

# 7 日より古いキャッシュを削除
tdsl cache clear --older-than 7
```

---

## `decompile`

JSON IR ファイルを `.tdsl` ソースコードに逆変換します。JSON を他ツールで生成した場合や、IR からソースを復元したい場合に使います。

```
tdsl decompile [OPTIONS] [INPUT]
```

### 引数

| 引数 | 説明 |
|---|---|
| `[INPUT]` | 入力 JSON ファイルのパス（省略時は標準入力） |

### オプション

| オプション | 説明 | デフォルト |
|---|---|---|
| `-o, --output <OUTPUT>` | 出力 `.tdsl` ファイルのパス | 標準出力 |

### 実行例

```bash
# JSON IR を .tdsl に逆変換
tdsl decompile out.json

# ファイルに保存
tdsl decompile out.json --output recovered.tdsl

# パイプライン経由（標準入力から）
tdsl build examples/china_dynasties.tdsl --pretty | tdsl decompile --output recovered.tdsl
```

> **制約（コメント非対応）**: `decompile` は JSON IR を起点とするため、元の `.tdsl` に書かれていたコメント（`//`・`/* */`）は復元できません。コメントは parser の AST には保持されますが、lowering では参照されず IR には現れないため、これは IR を単一の真実とする設計上の恒久的な制約です。

---

## `completions`

指定シェル向けの補完スクリプトを生成します。生成したスクリプトをシェルの設定ファイルに追加することで、`tdsl` のサブコマンドやオプションを Tab 補完できるようになります。

```
tdsl completions [OPTIONS] <SHELL>
```

### 引数

| 引数 | 説明 |
|---|---|
| `<SHELL>` | 対象シェル（`bash` / `elvish` / `fish` / `powershell` / `zsh`） |

### 実行例

```bash
# bash 補完スクリプトを生成・インストール
tdsl completions bash >> ~/.bashrc
source ~/.bashrc

# fish 補完スクリプトをインストール
tdsl completions fish > ~/.config/fish/completions/tdsl.fish

# zsh 補完スクリプトをインストール
tdsl completions zsh > ~/.zfunc/_tdsl
echo 'fpath=(~/.zfunc $fpath)' >> ~/.zshrc
echo 'autoload -Uz compinit && compinit' >> ~/.zshrc
source ~/.zshrc
```

---

## Wikidata 連携コマンドの注意事項

`build`, `merge`, `render`, `scaffold`, `fetch`, `search`, `inspect`, `resolve` は Wikidata API を呼び出す可能性があります。

- **レート制限**: Wikidata API にはレート制限があります。大量フェッチが必要な場合は `--offline` で開発し、最終確認時にオンラインビルドを実施してください。
- **キャッシュ**: 取得結果はデフォルトで `~/.cache/tdsl/` に 24 時間キャッシュされます。`--no-cache` で強制リフレッシュ、`--cache-ttl 0` でキャッシュを無効化できます。
- **タイムアウト**: ネットワーク環境が遅い場合は `--wikidata-timeout` を増やしてください（例: `--wikidata-timeout 60`）。

---

---

## `lsp`

LSP（Language Server Protocol）サーバを stdio 経由で起動します。

```
tdsl lsp
```

### 説明

標準入力（stdin）から JSON-RPC 2.0 メッセージを読み込み、標準出力（stdout）に応答を書き出す LSP サーバとして動作します。

**現バージョンで対応している機能:**

| 機能 | 説明 |
|---|---|
| `textDocument/publishDiagnostics` | パースエラー・検証警告・静的参照エラーをリアルタイムで通知 |
| `textDocument/didOpen` | ドキュメントを開いたときに診断を実行 |
| `textDocument/didChange` | ドキュメント変更時に診断を再実行（FULL sync） |
| `textDocument/didClose` | ドキュメントを閉じたときに診断をクリア |
| `textDocument/completion` | カーソル位置のブロック構造に応じた補完候補を返す。ブロックごとのキーワード（`timeline` / `lane` / `group` / `map` / `template` / `import` / `apply` / item オプション）に加え、**値の位置では宣言済み ID を補完する**（`span` / `event` / `event_range` の直後は lane ID、`map` の直後と `apply <template> to` の直後は import alias、`apply` の直後は template ID）。値補完は offline のみで、Wikidata の entity key は対象外（補完のために暗黙にネットワークへ出ない）。`as` を省略した宣言は ID を推測しない |
| `textDocument/hover` | lane ID → lane 情報（ラベル・kind・order）/ QID → キャッシュ済みエンティティ情報（offline） |
| `textDocument/definition` | lane 参照位置 → lane 宣言位置へのジャンプ |
| `textDocument/codeAction` | `tdsl lint --fix` 相当の自動修正を quick fix として提示（全文置換・offline） |
| `textDocument/documentSymbol` | timeline / lane / アイテムの階層シンボルを返す（アウトライン・ブレッドクラム表示） |
| `textDocument/references` | lane ID の全参照位置を返す（`includeDeclaration` で宣言含む／含まないを制御） |
| `textDocument/rename` | lane ID の宣言＋全参照を新名称に一括置換（明示的 `as <alias>` 限定） |
| `textDocument/prepareRename` | リネーム対象の妥当性検証（`as` 省略 lane は拒否） |
| `textDocument/formatting` | DSL ソースを正準形（2 スペースインデント・ブロック間空行 1 行）に整形する全文置換 TextEdit を返す |

> **`import` / `map` / `apply` ブロックについて**: LSP の診断はネットワークアクセスを伴わない静的解析（offline）で行うため、Wikidata 取得が前提のエンティティ解決は行いません。ただし、ネットワーク不要で判定できる **静的な参照エラー** は offline でも検出します:
>
> - `map <alias>.<key>` の `alias` が未宣言（`import ... as <alias>` が存在しない）→ **Error**
> - `apply <template> to <import>` の `template` / `import` が未宣言 → **Error**
>
> エンティティ解決に依存するブロック（参照は正しいが Wikidata 取得が必要なもの）は黙って無視せず、各ブロック位置に **Information レベルの診断**（「offline 診断では未解決」）を表示します。生成されるアイテムの完全な検証は `tdsl build` / `tdsl check` を使用してください。

> **フォーマットとコメントについて**: `textDocument/formatting` は `tdsl_parser::format_source` を使って全文置換 TextEdit を返します。コメント（`//` 行コメント・`/* */` ブロックコメント）は保持されますが、ブロック内部コメントは正準位置へ移動される場合があります。

### エディタ連携

VS Code では [Timeline DSL 拡張](https://marketplace.visualstudio.com/items?itemName=keroway.timeline-dsl) が `tdsl lsp` を自動起動します。`timelineDsl.serverPath` に `tdsl` バイナリの絶対パスを指定でき、未指定時は PATH から解決します。

Neovim / Helix などの汎用 LSP クライアントでは以下のように手動設定が可能です（設定方法はエディタのドキュメントを参照）:

```
command: ["tdsl", "lsp"]
root_markers: ["*.tdsl"]
filetypes: ["tdsl"]
```

### 実行例

```bash
# LSP サーバを起動（stdin 待機でブロックする）
tdsl lsp

# 最小 JSON-RPC リクエストで動作確認（Content-Length ヘッダ必須）
echo -e 'Content-Length: 2\r\n\r\n{}' | tdsl lsp
```

---

## 関連ドキュメント

- [Getting Started チュートリアル](tutorial.md) — ステップバイステップのハンズオン
- [DSL 言語仕様](dsl-spec.md) — 文法リファレンス
- [スタイルカスタマイズガイド](styling.md) — `--theme` / `--custom-css` によるカスタマイズ
- [エラーコードカタログ](error-catalog.md) — エラーメッセージの原因と修正方法
- [CI 連携](../docs/ci-integration.md) — GitHub Actions での利用方法
