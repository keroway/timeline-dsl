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

### 方針: 手動同期（真実源 = `src/lang-tdsl/index.ts`）

WebUI 側の CodeMirror ハイライトと VS Code 拡張のシンタックスハイライトでは、
キーワード集合が**手動で二重管理**されています。

| ファイル | 役割 |
|---|---|
| `src/lang-tdsl/index.ts:8-22` | CodeMirror StreamLanguage 用キーワード集合（`BLOCK_KEYWORDS` / `ITEM_KEYWORDS` / `MISC_KEYWORDS`）|
| `editors/vscode/syntaxes/tdsl.tmLanguage.json` | VS Code TextMate grammar キーワードパターン |

ビルド時自動生成は将来の改善課題（フォローアップ issue 参照）とし、現時点では**両ファイルを手動で同時に更新する運用**としています。

### 文法ステートメントを追加するときの更新手順

1. `crates/tdsl-parser/src/grammar.pest` に文法規則を追加
2. `crates/tdsl-parser/src/builder.rs` / `crates/tdsl-core/src/lower.rs` を更新
3. **`apps/webui/src/lang-tdsl/index.ts`** の `BLOCK_KEYWORDS` / `ITEM_KEYWORDS` / `MISC_KEYWORDS` に追加
4. **`editors/vscode/syntaxes/tdsl.tmLanguage.json`** の該当パターン文字列に追加
5. `cargo test --workspace` と `npm run build` がパスすることを確認

> ⚠️ 手順 3 と 4 を同時に更新しないとシンタックスハイライトが VSCode と WebUI で不一致になります。

### 将来の改善（フォローアップ issue: #204 以降で追跡）

`src/lang-tdsl/index.ts` を単一の真実源とし、tmLanguage の一部をビルド時に自動生成する仕組みを導入する予定です。
