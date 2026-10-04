---
paths:
  - "crates/**"
  - "apps/webui/**"
  - "editors/vscode/**"
---

# アーキテクチャ詳細

CLAUDE.md を補完するパス限定ルール。矛盾があれば CLAUDE.md を正とする。

## クレート構成

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

## コンパイルパイプライン

1. **パース**: `.tdsl` → `Vec<Statement>`（AST）
2. **Lowering Pass 1**: timeline/lane 宣言を収集
3. **Lowering Pass 2**: 静的アイテム（span/event/event_range）を変換
4. **Lowering Pass 3**: import ブロックを解決（Wikidata fetch）
5. **Lowering Pass 4**: map ブロックを適用してアイテム生成
6. **バリデーション**（`tdsl_core::validate`）: range整合性、lane kind、item→lane 参照、start>end
7. **lint**（`tdsl_core::lint`、`tdsl lint` から呼ばれる）: 未使用 lane、ID 重複、空ラベル、タグの重複など

## IR構造（`tdsl_core::ir::TimelineIr`）

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
