# WebUI 技術選定

作成日: 2026-05-05

## 結論

WebUI MVP は **静的サイト + client-side WASM** を採用する。

- ホスティング: GitHub Pages または Cloudflare Pages
- フロントエンド: Vite + TypeScript + React
- コンパイラ連携: `wasm-bindgen` で公開する薄い WASM facade
- MVP の処理範囲: `.tdsl` テキストの parse/check/render と SVG/HTML プレビュー
- Wikidata 連携: 初期 MVP では `origin=*` を付けたブラウザ fetch で検索/取得を扱い、必要なら後続で Pages Functions などの軽量 proxy を追加する

この方針は、サーバー運用なしでエディタとリアルタイムプレビューを提供しつつ、IR を単一の境界として Rust 実装を再利用できるため、Issue #62 の MVP に最短で進める。

## 評価マトリクス

スコアは 5 が最良。

| 評価軸 | 重み | Option A: WASM UI (Leptos / Yew) | Option B: HTTP API (axum) | Option C: 静的サイト + client-side WASM |
|---|---:|---:|---:|---:|
| 実装コスト | 高 | 2 | 3 | 4 |
| ユーザー体験 | 高 | 4 | 4 | 4 |
| ホスティングコスト | 中 | 5 | 2 | 5 |
| メンテナンス性 | 中 | 3 | 3 | 4 |
| Wikidata 連携 | 中 | 3 | 5 | 3 |
| 配布の単純さ | 中 | 4 | 2 | 5 |
| 総合判断 |  | 非採用 | 保留 | 採用 |

## Option A: WASM UI (Leptos / Yew)

Rust で UI まで実装する案。

利点:

- Rust の型と既存 crate を UI 側にも寄せられる
- ビルド成果物は静的ファイルとして配布できる

課題:

- MVP では UI フレームワーク習熟と WASM build の両方が必要になる
- 既存の `tdsl-core` は `tdsl-wikidata` に直接依存しており、`reqwest`、`tokio`、ローカル cache、`dirs` を含むため、そのまま browser WASM に寄せにくい
- UI と compiler facade を同時に WASM 化すると切り分けが難しくなる

判断:

- MVP では採用しない。Rust UI は、WASM facade が安定してから再評価する。

## Option B: HTTP API (axum)

`tdsl serve` のようなローカル/リモート HTTP API を追加し、ブラウザ UI から呼ぶ案。

利点:

- `tdsl-wikidata` の既存実装、cache、retry、rate limit をそのまま使いやすい
- CORS や API key などの制御をサーバー側に集約できる
- 将来の共有・保存・アカウント機能に拡張しやすい

課題:

- MVP のために `tdsl serve`、API schema、CORS、deployment、observability を追加する必要がある
- CLI 配布と WebUI 配布の運用経路が分かれる
- サーバーを持つとホスティング費用と保守責務が発生する

判断:

- 初期 MVP では採用しない。Wikidata proxy、共有 URL の永続化、ユーザー保存が必要になった段階で追加する。

## Option C: 静的サイト + client-side WASM

UI は通常の Web stack で構築し、`.tdsl` の parse/check/render だけを Rust WASM facade に閉じ込める案。

利点:

- 静的ホスティングで配布でき、サーバー運用が不要
- 既存 renderer が生成する SVG/HTML をプレビューに再利用できる
- Web UI の操作性は React/TypeScript の標準的な構成で作りやすい
- WASM 境界を compiler facade に限定できるため、問題の切り分けがしやすい

課題:

- `tdsl-core` と `tdsl-wikidata` の依存分離が必要
- ブラウザから Wikidata API を呼ぶ場合は CORS、rate limit、失敗時 UI を明示的に扱う必要がある
- 大きな `.tdsl` では main thread blocking を避けるため、後続で Web Worker 化を検討する

判断:

- MVP の採用案とする。

## 採用スタック

### Rust/WASM

新しい facade crate を追加する。

```text
crates/tdsl-wasm
```

公開 API の最小案:

```rust
compile_to_ir(source: &str) -> Result<JsValue, JsValue>
render_svg_from_source(source: &str, options: JsValue) -> Result<String, JsValue>
check_source(source: &str) -> JsValue
```

実装方針:

- facade は `tdsl-parser`、`tdsl-core`、`tdsl-render` を呼ぶだけにする
- WebUI は parser AST ではなく IR/renderer 結果だけに依存する
- `tdsl-wikidata` は初期 facade に含めない
- `tdsl-core` の Wikidata 依存を optional feature に分離する
- `wasm-bindgen` の `--target web` または bundler 経由で読み込む

既存 crate への影響:

- `tdsl-parser` と `tdsl-render` は比較的 WASM 化しやすい
- `tdsl-render` は `TimelineIr` 入力だけを要求しており、IR authoritative の境界を保てる
- 最大の制約は `tdsl-core` が常に `tdsl-wikidata` と `tokio` に依存している点
- `tdsl-wikidata` は `reqwest`、`tokio::time::sleep`、`dirs`、`std::fs`、`tempfile` を含むため、初期 browser WASM の対象から外す
- MVP の最小対応は `tdsl-core` の `wikidata` feature gate 化
- より明確に分離する場合は、Wikidata import 解決を後続で `tdsl-import` または `tdsl-wikidata-lower` 相当の crate に切り出す

### Frontend

```text
apps/webui
```

最小構成:

- Vite
- TypeScript
- React
- CodeMirror 6
- SVG preview pane

MVP 画面:

- 左: `.tdsl` editor
- 右: realtime preview
- 下部または右パネル: diagnostics
- ファイル操作: open local file, download `.tdsl`, download rendered SVG/HTML
- サンプル切り替え: static timeline と import を含む timeline を最初から選べる

UX 方針:

- プレビュー更新は短い debounce を入れ、入力中のちらつきと過剰な WASM 呼び出しを避ける
- syntax/semantic error はプレビュー領域を壊さず、行番号、エラー内容、修正ヒントを diagnostics に表示する
- unknown lane、unknown import reference、unknown map target は警告ではなくブロッキングエラーとして表示する
- ビジュアル編集ではなく DSL editor を主操作面にする

### Hosting

第一候補は GitHub Pages。GitHub Pages は repository の HTML/CSS/JavaScript を静的サイトとして公開できるため、この MVP と相性がよい。

Cloudflare Pages は preview deployments、Pages Functions、rollbacks を使いたい段階で候補にする。Cloudflare Pages は静的/フルスタックアプリを Git provider または direct upload から配布でき、後続で proxy が必要になった場合の逃げ道がある。

## Wikidata 連携方針

初期 MVP:

- `.tdsl` 内の static items は完全にブラウザ内で parse/check/render する
- import を含む `.tdsl` はまず「未解決 import がある」diagnostic として表示する
- 検索/候補表示はブラウザ fetch で Wikidata API を呼ぶ
- MediaWiki Action API の unauthenticated CORS では `origin=*` を使う

後続:

- `tdsl-wikidata` の browser 対応を検討する
- rate limit や cache の UX が問題になった場合は Cloudflare Pages Functions などで proxy/cache を追加する
- server-side import が必要になったら Option B の `tdsl serve` を再評価する

## 実装計画

### Sprint 1: WASM facade の土台

- `tdsl-core` から `tdsl-wikidata` 依存を optional feature に分離する
- `tdsl-wasm` crate を追加する
- `compile_to_ir` と `render_svg_from_source` を公開する
- browser WASM build を CI で確認する

### Sprint 2: WebUI MVP

- `apps/webui` を Vite + TypeScript + React で追加する
- CodeMirror editor と preview pane を実装する
- 入力変更時に WASM facade を呼び、diagnostics と preview を更新する
- local file open/download を実装する

### Sprint 3: Wikidata 支援

- `search` / `inspect` 相当の UX を WebUI に追加する
- CORS 失敗、rate limit、offline 時の diagnostic を明示する
- 必要なら Pages Functions proxy/cache を追加する

## MVP では実装しないもの

- 共有 URL
- クラウド保存
- 認証
- 複数プロジェクト管理
- ビジュアル編集
- 共同編集
- 完全な Wikidata re-import / merge policy UI

## Issue #36 への反映内容

Issue #36 は次の方針で更新する。

- 技術スタックは「Vite + TypeScript + React + CodeMirror 6 + Rust WASM facade」
- ホスティングは「GitHub Pages を第一候補、Cloudflare Pages は proxy/cache が必要になった場合の候補」
- MVP は「エディタ + リアルタイム SVG プレビュー + diagnostics + local file open/download」
- import/Wikidata は初期 MVP では限定対応し、未解決 import は diagnostic として扱う

## 参照

- MediaWiki Action API cross-site requests: https://www.mediawiki.org/wiki/API:Cross-site_requests
- wasm-bindgen deployment targets: https://wasm-bindgen.github.io/wasm-bindgen/reference/deployment.html
- GitHub Pages overview: https://docs.github.com/en/pages/getting-started-with-github-pages/what-is-github-pages
- Cloudflare Pages overview: https://developers.cloudflare.com/pages/
- axum crate docs: https://docs.rs/axum/latest/axum/
