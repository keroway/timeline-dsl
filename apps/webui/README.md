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
