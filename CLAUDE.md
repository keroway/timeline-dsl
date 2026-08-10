# CLAUDE.md -- Timeline DSL プロジェクト指示書

## プロジェクト概要

年表特化のDSLコンパイラ。`.tdsl` ファイルをパースし、WikidataからデータをインポートしてJSON IRに変換する。Rustで実装。

## ビルド・テスト

```bash
# ビルド
cargo build --workspace

# 全テスト実行
cargo test --workspace

# 特定クレートのテスト
cargo test -p tdsl-parser
cargo test -p tdsl-core
cargo test -p tdsl-wikidata

# CLIの実行（例）
cargo run -p tdsl-cli -- build examples/china_dynasties.tdsl --pretty
cargo run -p tdsl-cli -- check examples/china_dynasties.tdsl
cargo run -p tdsl-cli -- build examples/china_with_import.tdsl --pretty
cargo run -p tdsl-cli -- build examples/china_with_import.tdsl --offline --pretty
cargo run -p tdsl-cli -- fetch Q7209 --lang ja,en
```

## アーキテクチャ

### クレート構成

依存方向（`cargo metadata` が正。矢印の先が依存される側）:

```
tdsl-parser ← tdsl-core ← tdsl-render ← tdsl-wasm
tdsl-wikidata ↗         ↖ tdsl-lsp ← tdsl-cli
```

`tdsl-parser` と `tdsl-wikidata` は他のワークスペースクレートに依存しない基底。
`tdsl-cli` は core / parser / wikidata / render / lsp すべてに依存する最上位。
`tdsl-wasm` は CLI からは参照されず、WebUI 向けのビルドターゲットとして独立している。

```
crates/
├── tdsl-parser/    # PEG文法(pest) → AST
│   ├── grammar.pest   # PEG文法定義
│   ├── ast.rs         # AST型定義
│   ├── builder.rs     # pest解析木 → AST変換
│   ├── format.rs      # AST → 整形済みソース（tdsl fmt）
│   ├── comments.rs    # コメントの保持
│   ├── now.rs         # `now` キーワードの解決
│   └── error.rs       # パースエラー
├── tdsl-core/      # AST → IR変換・バリデーション
│   ├── ir.rs          # IR型定義（JSON直列化対象）
│   ├── lower/         # 4パスlowering（静的 / Wikidata連携）。mod.rs がパスを束ねる
│   │   ├── declarations.rs   # Pass 1: timeline/lane 宣言の収集
│   │   ├── static_items.rs   # Pass 2: 静的アイテムの変換
│   │   ├── imports.rs        # Pass 3: import ブロックの解決
│   │   ├── mapping.rs        # map / template / color_map の適用
│   │   └── context.rs        # パス間で共有する状態
│   ├── validate.rs    # 意味検証
│   ├── lint.rs        # tdsl lint（品質チェックと --fix）
│   ├── merge.rs       # tdsl merge（複数 IR のマージ）
│   ├── decompile.rs   # tdsl decompile（JSON IR → .tdsl 逆変換）
│   └── error.rs       # lowering エラー
├── tdsl-wikidata/  # Wikidata APIクライアント
│   ├── client.rs      # WikidataClient trait + HTTP実装
│   ├── entity.rs      # エンティティ型 + 時間パース
│   ├── cache.rs       # 取得キャッシュ（TTL、~/.cache/tdsl/）
│   └── error.rs       # Wikidataエラー
├── tdsl-render/    # IR → SVG / HTML / PDF / PNG
│   ├── layout.rs      # LayoutModel の算出（描画の中核）
│   ├── svg.rs         # SVG 直列化
│   ├── html.rs        # SVG を埋め込んだスタンドアロン HTML
│   ├── pdf.rs / png.rs        # ラスタ・PDF 出力
│   └── pagination.rs / time_range_pagination.rs  # ページ分割（ADR-0005 D2）
├── tdsl-lsp/       # Language Server（tdsl lsp から起動）
│   ├── backend.rs     # LSP サーバ本体
│   ├── completion.rs / hover.rs / diagnostics.rs / formatting.rs
│   └── goto_definition.rs / find_references.rs / rename.rs / code_action.rs
├── tdsl-wasm/      # WebUI 向け wasm バインディング（CLI からは参照されない）
│   └── lib.rs
└── tdsl-cli/       # CLIバイナリ
    ├── main.rs        # 引数パースとディスパッチ
    └── commands/      # サブコマンド1つにつき1ファイル
```

### コンパイルパイプライン

1. **パース**: `.tdsl` → `Vec<Statement>`（AST）
2. **Lowering Pass 1**: timeline/lane 宣言を収集
3. **Lowering Pass 2**: 静的アイテム（span/event/event_range）を変換
4. **Lowering Pass 3**: import ブロックを解決（Wikidata fetch）
5. **Lowering Pass 4**: map ブロックを適用してアイテム生成
6. **バリデーション**: range整合性、未使用lane等の警告

### IR構造（`tdsl_core::ir::TimelineIr`）

- `meta`: title, unit, range, calendar
- `lanes`: id, label, kind, order
- `items`: Span / Event / EventRange（tagged enum）
  - 各 item の共通フィールド: `id`, `lane`, `label`, `tags`, `source`, `origin`
  - `source_span?: { line, col_start, col_end }` — ソーステキストを渡した場合のみ付与（1-based 行番号・列番号）。JSON では `None` のとき省略
- `imports`: インポート記録
- `sources`: 出典・ライセンス情報

### `source_span` の付与条件

`lower_static_with_source(file, Some(src))` または `lower_with_wikidata_and_source(file, client, Some(src))` にソーステキストを渡した場合のみ付与。
`lower_static(file)` / `lower_with_wikidata(file, client)` では常に `None`（JSON に出力されない）。
WebUI と WASM バインディングはソーステキストを渡しているため `source_span` が含まれる。CLI の `build` サブコマンドはソースを渡さないため含まれない（将来拡張可能）。

## コーディング規約

- **Edition**: Rust 2024
- **エラー処理**: `thiserror` でエラー型定義、`miette` で整形出力
- **非同期**: `tokio`（Wikidata fetch に使用）
- **パーサ**: `pest` PEGパーサジェネレータ。文法変更は `grammar.pest` を編集後 `builder.rs` も更新すること
- **テスト**: 各クレートの `lib.rs` 末尾に `#[cfg(test)]` で統合テスト
- **シリアライズ**: IR型は `serde::Serialize + Deserialize` を derive

## DSL文法の変更手順

1. `crates/tdsl-parser/src/grammar.pest` を編集
2. `crates/tdsl-parser/src/ast.rs` にAST型を追加/変更
3. `crates/tdsl-parser/src/builder.rs` に変換ロジックを実装
4. `crates/tdsl-core/src/lower/` にloweringロジックを追加（宣言なら `declarations.rs`、静的アイテムなら `static_items.rs`、import 解決なら `imports.rs`）
5. 必要に応じて `crates/tdsl-core/src/ir.rs` のIR型を更新
6. `cargo test --workspace` で全テスト通過を確認
7. **シンタックスハイライトのキーワードを更新すること**（手順下記参照）

### シンタックスハイライトのキーワード管理

キーワードの**単一真実源**は `apps/webui/src/lang-tdsl/keywords.json` です。
VS Code 拡張の `editors/vscode/syntaxes/tdsl.tmLanguage.json` は `npm run build` 時に自動生成されます。

- `apps/webui/src/lang-tdsl/keywords.json` — `BLOCK_KEYWORDS` / `ITEM_KEYWORDS` / `MISC_KEYWORDS` を編集する。`apps/webui/src/lang-tdsl/keywords.ts` は `keywords.json` を型付きで re-export するだけの生成物寄りファイルであり、手編集しない
- `npm run build`（または `node editors/vscode/scripts/gen-grammar-keywords.mjs`）を実行すると `tdsl.tmLanguage.json` が自動更新される

詳細は `apps/webui/README.md` の「シンタックスハイライトのキーワード管理」セクションを参照。

Rust LSP（`crates/tdsl-lsp/src/keywords.rs`）も `keywords.json` をミラーし、ドリフト防止テストで同期を保証する。

`README.md` / `README.ja.md` / `editors/vscode/README.md` の「Syntax Highlighting」節では、キーワードをハードコード列挙**しない**（列挙すると `keywords.json` 更新時にドリフトする。実例: #665）。代わりに `apps/webui/src/lang-tdsl/keywords.json` へのリンクで済ませる。

## Wikidataプロパティ（頻用）

| プロパティ | 意味 | DSL式 |
|---|---|---|
| P569 | 誕生年 | `claim(P569).year` |
| P570 | 死亡年 | `claim(P570).year` |
| P571 | 成立年 | `claim(P571).year` |
| P576 | 消滅年 | `claim(P576).year` |
| P580 | 開始時点 | `claim(P580).year` |
| P582 | 終了時点 | `claim(P582).year` |

## 現在のMVP実装状況

### 実装済み

- PEG文法 + パーサ（7種のstatement: timeline, lane, span, event, event_range, import, map）
- AST → IR変換（静的 / Wikidata連携 両方）
- Wikidata HTTPクライアント（wbgetentities API, wbsearchentities, SPARQL）
- CLI サブコマンド: `build` / `check` / `ast` / `fetch` / `search` / `inspect` / `resolve` / `scaffold` / `render` / `init` / `import-csv` / `export-csv` / `lint` / `decompile` / `merge` / `cache` / `lsp` / `fmt` / `completions`（`main.rs` の `enum Commands` が正）
- JSON IR出力（`origin` フィールドを含む）
- コメント（行 `//` / ブロック `/* */`）
- `map` の `target_type` は enum 型（span / event / event_range のみ許可）
- imported item の `source` は `wd:<entity_id>` で自動付与（map 内での手動指定は廃止）
- 日本語 lane 名で `as` 省略時、ASCII slug が空なら `lane_N` を自動採番
- 静的アイテム（event / event_range）の `source` も `sources[]` に登録
- 再インポートポリシー（merge_by_source / overwrite_imported / keep_manual）を lowering で実装済み
- `query "SPARQL" as alias` による複数エンティティの一括インポートを実装済み
- HTMLレンダリング（`tdsl-render` クレート、インラインSVG）
- `tdsl render --interactive` によるズーム・パン・検索・凡例・詳細パネル付きインタラクティブHTML
- `tdsl render --format svg` によるスタンドアロンSVG出力
- `tdsl lint` による品質チェックと自動修正（`--fix`）
- `template` / `apply` 構文（共通フォーマットのテンプレート再利用）
- `color_map` ブロック（タグ→色マッピングの宣言的定義）
- `tdsl decompile`（JSON IR → `.tdsl` 逆変換）
- `tdsl export-csv`（IR → CSV。`import-csv` と対称。`source`/`origin` を含む 10 列全てが往復で保持される）
- Wikidata取得キャッシュ（TTL管理、`~/.cache/tdsl/` に保存）
- Wikidata APIリトライ（HTTP 429・5xx に対するexponential backoff、最大5回 / `DEFAULT_MAX_RETRIES`）
- `tdsl cache status` / `tdsl cache clear` によるキャッシュ管理
- フィールド別インポート優先度（`policy field_priority { ... }`）
- WebUI（WASM + Vite/React）: CodeMirror 6 シンタックスハイライト・SVGプレビュー・スケール制御・診断パネル
- VS Code 拡張（TextMate grammar ベース構文ハイライト、Marketplace 公開済み）
- Homebrew formula（`brew tap keroway/tap && brew install tdsl`）
- Windows バイナリ対応
- Criterion ベンチマーク（パーサ・lowering・レンダリング）
- `tdsl merge`（複数 `.tdsl` ファイルのIRマージ）
- GitHub Actions composite action（`action.yml`）: `uses: keroway/timeline-dsl@v1` で `.tdsl` → SVG/HTML レンダリングを CI から呼び出せる（詳細: `docs/ci-integration.md`）
- `tdsl render --chart-pagination <N>`: タイムライン本体（チャート部分）を lane グループ単位で複数ページに分割出力（ADR-0005 D2 / #660, #661）。`--format svg` では `<stem>.pageN.svg` の複数ファイル、`--format pdf` では単一 PDF 内の複数ページ（チャートページ群 → テーブルページ群の順、`--pdf-pagination` 併用時はテーブルページ番号がテーブルページ数のみを数える）として出力される。`--show-table` 併用時は IR 全体の item を一覧する専用テーブルページを末尾に追加

## 未実装 / 意図的に対応しない機能

- `map source` -- `map` ブロック内の `source:` プロパティ指定。`MapProp` に `Source` バリアントが存在せず、pest 文法（`grammar.pest` の `map_prop`）がそもそも受理しないためパース時点で拒否される（item レベルの `source wd:<QID>` のみ有効）
- サブ秒（ミリ秒未満）精度
- IANA タイムゾーン名（例: `Asia/Tokyo`）による DST 自動解決 -- 意図的に非対応と確定済み（ADR-0007、2026-07-26 決定）。固定の数値 UTC オフセット（`+09:00` 等）のみサポート

これらに遭遇した場合は silent fallback ではなく必ずパース/lowering エラーで拒否する（「No silent fallback」原則、`.claude/rules/implementation-strict.md` §2）。秒精度（`DateTimeSecond`）と UTC オフセット（`DateTimeOffset` / `DateTimeSecondOffset`）自体は #612〜#616（ADR-0003）で実装済みなので、上記の未実装リストに含めない。

## サンプルファイル

- `examples/china_dynasties.tdsl` -- 静的定義のみ（インポートなし）
- `examples/china_with_import.tdsl` -- Wikidata連携つき
- `examples/japanese_history.tdsl` -- 日本史
- `examples/samurai_wikidata.tdsl` -- 武将（Wikidata連携）
- `examples/world_wars.tdsl` -- 世界大戦
- `examples/sci_tech_timeline.tdsl` -- 科学技術史
- `examples/fictional_empire.tdsl` -- 架空の帝国（CSV連携例付き）
- `examples/template_apply_example.tdsl` -- `template` / `apply` 構文の使用例
- `examples/grouped_dynasties.tdsl` -- `group` ブロックの使用例（静的定義のみ）
- `examples/officeholder_wikidata.tdsl` -- `expand claim(P39)` / `qualifier(P580/P582)` の使用例（Wikidata連携）
- `examples/iss_docking_second_precision.tdsl` -- 秒精度 + UTC(`Z`)オフセットの使用例（#612〜#616、ADR 0003、静的定義のみ）
- `examples/global_conference_timezones.tdsl` -- 複数タイムゾーン（`+09:00`/`-05:00`/`Z`）の使用例とoffset付き値同士のUTC正規化比較（#612〜#616、ADR 0003 D2、静的定義のみ）
- `examples/feature_showcase.tdsl` -- `note` / `link` / `color`（block_options）・open-ended `now` の使用例（#663、静的定義のみ）
- `examples/china_dynasties_filtered.tdsl` -- `filter` 句によるインポートエンティティの絞り込み例（#142、Wikidata連携）

## 注意点

- Wikidata APIにはレート制限あり。大量fetchする場合は `--offline` で開発し、最終確認時にオンラインビルド
- 負の年（紀元前）は整数で表現: `-206` = 紀元前206年
- lane IDの `as` 省略時はラベルからASCIIスラッグを自動生成。日本語のみの場合は `lane_N` に自動採番
- `source wd:QXXX` はWikidata出典を表し、IR の sources に CC0 ライセンスとして記録
- map ブロックの `source` プロパティは廃止済み。imported item の source は `wd:<entity_id>` で自動付与
- map の `target_type` は `span` / `event` / `event_range` のみ。不正値はパースエラー
- `wd.xxx` の entity_key が import に存在しない場合はエラー（全件フォールバックしない）
- imported item の `origin` は lowering で常に `"wikidata"` に固定される（`crates/tdsl-core/src/lower/mapping.rs`）。静的アイテムの `origin` は DSL の `origin` オプションで宣言した値がそのまま使われ、lowering は上書きしない（`crates/tdsl-core/src/lower/static_items.rs`）

## Claude Code 用セットアップ（このリポジトリ）

このリポジトリには Claude Code 用の補助設定が `.claude/` 配下にコミットされている。実装時は以下を参照・利用すること。

- **`.claude/rules/implementation-strict.md`** -- 実装方針の strict ルール。本ファイル（`AGENTS.md` は本ファイルへの symlink）に加えて必ず参照する。NO-GO パターン、コードレベルの規約、テスト最低ライン、PR 提出前ゲートを定義。
- **`.claude/agents/rust-app-developer.md`** -- Rust 実装用サブエージェント。文法・lowering・Wikidata 連携の実装はこれに委譲する。
- **`.claude/agents/app-dev-director.md`** -- 設計判断・スコープ整理・仕様整合性レビュー用サブエージェント。実装着手前のレビュー、実装後の整合性チェックに使う。
- **`.claude/commands/fix-pr.md`** -- `/fix-pr [PR番号]` で自分の PR の CI 失敗を自動修正する。
- **`.claude/hooks/post-stop-check.sh`** -- Stop hook。応答完了時に変更ファイルを見て `cargo fmt --check` / `cargo clippy -D warnings` / `cargo test --workspace` を実行（WebUI 変更時は `npm run lint` も）。スキップは `TDSL_SKIP_STOP_HOOK=1`。

実装着手時は `.claude/rules/implementation-strict.md` の「§3 着手前チェックリスト」を埋めてから書き始めること。
