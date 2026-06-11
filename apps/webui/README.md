# Timeline DSL WebUI

Timeline DSL の Web エディタ。ブラウザ上で `.tdsl` ファイルを編集し、リアルタイムで SVG 年表プレビューを確認できます。

## 機能

- CodeMirror 6 によるテキストエディタ（シンタックスハイライト）
- 500ms debounce によるリアルタイム SVG プレビュー
- エラー・警告の診断パネル
- `.tdsl` ファイルのダウンロード / 開く
- SVG のダウンロード
- サンプル切り替え

## 開発

### 初回セットアップ

```bash
# リポジトリルートから
cd apps/webui
npm install
```

### WASM ビルド（初回・crates/tdsl-wasm 変更時）

```bash
# wasm-pack が未インストールの場合
cargo install wasm-pack

# WASM をビルド（apps/webui ディレクトリから実行）
wasm-pack build ../../crates/tdsl-wasm --target web --out-dir src/wasm --no-opt
```

### 開発サーバー起動

```bash
npm run dev
```

### プロダクションビルド

```bash
npm run build
```

## アーキテクチャ

- `src/wasmLoader.ts` — WASM 初期化と関数ラッパー
- `src/examples.ts` — サンプル .tdsl コンテンツ
- `src/App.tsx` — メインアプリコンポーネント
- `src/wasm/` — wasm-pack ビルド成果物（.gitignore 対象）

## WASM facade

`crates/tdsl-wasm` が提供する 3 関数:

| 関数 | 説明 |
|---|---|
| `compile_to_ir(source)` | .tdsl を IR（JSON）にコンパイル |
| `render_svg_from_source(source)` | SVG 文字列を生成（静的アイテムのみ） |
| `check_source(source)` | 診断結果を JSON 配列で返す |

## シンタックスハイライトのキーワード管理

### 方針: ビルド時自動生成（真実源 = `src/lang-tdsl/keywords.ts`）

キーワード集合の**単一真実源**は `src/lang-tdsl/keywords.ts` です。
VS Code 拡張の `editors/vscode/syntaxes/tdsl.tmLanguage.json` のキーワードパターンは、
`npm run build` の `prebuild` フックで自動生成されます（手動同期不要）。

| ファイル | 役割 |
|---|---|
| `src/lang-tdsl/keywords.ts` | キーワード配列の単一真実源（`BLOCK_KEYWORDS` / `ITEM_KEYWORDS` / `MISC_KEYWORDS`）|
| `src/lang-tdsl/index.ts` | CodeMirror StreamLanguage 定義（`keywords.ts` をインポート）|
| `editors/vscode/syntaxes/tdsl.tmLanguage.json` | VS Code TextMate grammar（ビルド時に自動更新）|
| `editors/vscode/scripts/gen-grammar-keywords.mjs` | 生成スクリプト |

### 文法ステートメントを追加するときの更新手順

1. `crates/tdsl-parser/src/grammar.pest` に文法規則を追加
2. `crates/tdsl-parser/src/builder.rs` / `crates/tdsl-core/src/lower.rs` を更新
3. **`apps/webui/src/lang-tdsl/keywords.ts`** の `BLOCK_KEYWORDS` / `ITEM_KEYWORDS` / `MISC_KEYWORDS` に追加
4. `cargo test --workspace` と `npm run build` がパスすることを確認（`npm run build` で `tdsl.tmLanguage.json` が自動更新される）
5. 再生成された `editors/vscode/syntaxes/tdsl.tmLanguage.json` を **必ずコミット**する。コミット忘れは CI の `Build WebUI` ジョブ内の "Check tmLanguage.json drift" ステップで検出され失敗する
