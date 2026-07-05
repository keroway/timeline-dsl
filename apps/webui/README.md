# Timeline DSL WebUI

Timeline DSL の Web エディタ。ブラウザ上で `.tdsl` ファイルを編集し、リアルタイムで SVG 年表プレビューを確認できます。

## 機能

- CodeMirror 6 によるテキストエディタ（シンタックスハイライト）
- 500ms debounce によるリアルタイム SVG プレビュー
- エラー・警告の診断パネル
- `.tdsl` ファイルのダウンロード / 開く
- File System Access API 対応ブラウザ（Chrome/Edge 等）ではローカル `.tdsl` ファイルを直接開き、上書き保存できる（`hooks/useFileHandle.ts`）。非対応ブラウザ（Safari/Firefox）では従来の `<input type="file">` 選択 + ダウンロード方式にフォールバックし、UI 上で非対応を明示する
- SVG のダウンロード
- サンプル切り替え
- 設定パネルからイベントラベル常時表示（`show_event_labels`）・向き・グリッド・SVGテーマなどのプレビューオプションを切り替え可能。一覧確認や印刷資料作成時にホバー無しで全イベントの文字を表示できる
- PWA としてインストール可能（初回ロード後は静的 DSL 編集・プレビューをオフライン起動可能）

## 開発

### 初回セットアップ

```bash
# リポジトリルートから
cd apps/webui
npm install
```

### WASM

WASM バインディングは `crates/tdsl-wasm` からビルドして公開されている npm パッケージ [`@keroway/tdsl-wasm`](https://www.npmjs.com/package/@keroway/tdsl-wasm) への依存として取得する（ADR 0001 / #580）。`npm install` だけで `node_modules/@keroway/tdsl-wasm` に展開され、追加の手動ビルド・コミットは不要。

`crates/tdsl-wasm` をローカルで変更して WebUI から動作確認したい場合は、リリース前の変更を試すため一時的に `file:` 依存へ切り替える:

```bash
# 別ディレクトリにローカルビルドを出力
wasm-pack build ../../crates/tdsl-wasm --target web --out-dir /tmp/tdsl-wasm-local --no-opt

# package.json の "@keroway/tdsl-wasm" を一時的に file: 参照に書き換えて npm install
#   "@keroway/tdsl-wasm": "file:/tmp/tdsl-wasm-local"

# 確認が終わったら package.json / package-lock.json を元の ^x.y.z に戻して npm install し直す
```

### 開発サーバー起動

```bash
npm run dev
```

### プロダクションビルド

```bash
npm run build
```

ビルド成果物には `manifest.webmanifest` と Service Worker が含まれます。Service Worker は JS/CSS/WASM などのアプリシェルを事前キャッシュし、初回ロード後のオフライン起動に対応します。新バージョンが利用可能になった場合は画面上部の更新通知から再読み込みできます。Wikidata インポートはブラウザでは実行されないため、オフライン時も UI 通知と診断で明示し、CLI での利用を案内します。

### PWA 手動チェックリスト

- Chrome DevTools / Lighthouse の PWA 監査で installable であることを確認する
- 初回ロード後に DevTools の Network を Offline にしてリロードし、エディタとプレビューが起動することを確認する
- 新しいビルドをデプロイした際に更新通知が表示され、再読み込みで更新されることを確認する

## アーキテクチャ

- `src/wasmLoader.ts` — WASM 初期化と関数ラッパー
- `src/gallery-meta.ts` — テンプレートギャラリーのメタ情報。本文は `examples/*.tdsl` を raw import して単一の真実源にする
- `src/examples.ts` — 初期表示用のオフラインテンプレート一覧（`gallery-meta.ts` から派生）
- `src/App.tsx` — メインアプリコンポーネント
- `src/hooks/useFileHandle.ts` — File System Access API（`showOpenFilePicker` / `showSaveFilePicker`）のラップ。開いた `FileSystemFileHandle` を保持し、保存時に同一ファイルへの上書き（`createWritable()`）を行う。非対応ブラウザではダウンロードにフォールバック
- `src/types/file-system-access.d.ts` — `Window.showOpenFilePicker` / `showSaveFilePicker` の ambient 型定義（TypeScript の標準 DOM lib には未収録）

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
