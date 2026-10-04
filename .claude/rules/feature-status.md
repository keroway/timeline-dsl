---
paths:
  - "crates/**"
  - "apps/webui/**"
  - "editors/vscode/**"
  - "examples/**"
---

# 現在のMVP実装状況

CLAUDE.md を補完するパス限定ルール。矛盾があれば CLAUDE.md を正とする。
（「未実装 / 意図的に対応しない機能」は CLAUDE.md 本体を参照。NO-GO 判定の絶対基準のため全ターン共通で載せている）

## 実装済み

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
