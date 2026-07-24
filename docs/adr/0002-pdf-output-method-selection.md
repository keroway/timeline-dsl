# ADR 0002: PDF 出力の方式選定

- **Status**: Accepted
- **Date**: 2026-05-23
- **Deciders**: keroway
- **Related issues**: #265（本 ADR + 実装）, #65（親: PNG/PDF 出力スプリント）
- **Supersedes**: なし

## コンテキスト

`tdsl render` は現在 HTML / SVG / PNG の 3 形式を出力できる。年表を論文・スライド・印刷物に貼り込む用途では PDF 出力の需要があり、親 issue #65 のスコープに含まれる。

PDF は SVG → PDF への変換方式が複数あり、選定によって実装難度・依存サイズ・配布制約・出力品質（ベクタ維持 / ラスタ化）が大きく変わる。先に方式を決定として記録し、最小実装（#265 の残作業）の前提を固めることが本 ADR の目的である。

判断の前提として、すでに実装済みの PNG 出力パイプラインの設計を踏襲できることが望ましい。現状は次のとおり。

- **PNG はすでに純 Rust スタックで実装済み**（`crates/tdsl-render/src/png.rs`）。`resvg` / `usvg` / `tiny-skia` を用い、IR から計算したレイアウトを SVG 文字列に直列化したうえで `usvg` でパースし `resvg` でラスタライズする。`fontdb` でシステムフォントを読み込み、CJK レーンラベル（Noto Sans JP / Hiragino Sans / Yu Gothic 等）を正しくシェイプする。
- PNG は opt-in の `png` Cargo feature でゲートされている（`crates/tdsl-render/Cargo.toml`: `[features] png = ["dep:resvg", "dep:thiserror"]`、`resvg = { version = "0.45", optional = true }`）。WASM クレット（`tdsl-wasm`）は `tdsl-render` をこの feature 無しで依存し、ブラウザ向けビルドを slim に保つ。CLI（`tdsl-cli`）は `tdsl-render = { features = ["png"] }` で依存する。
- 現在の依存バージョンは `resvg 0.45.1` / `usvg 0.45.1` / `tiny-skia 0.11.4`（`Cargo.lock`）。
- CLI の出力形式は `enum RenderFormat { Html, Svg, Png }` で表現し、`--format` フラグ（`cmd_render` 内の `match format`）で分岐する。
- レンダリングの公開 API は `crate::render_svg_only(ir, opts)` が SVG 文字列を返し、PNG はその SVG を `svg_to_png` で変換する 2 段構成になっている。

つまり「IR → SVG 文字列 → 変換」という既存パイプラインの最終段に PDF 変換器を 1 つ追加できれば、PNG と対称な構造で実装できる。

## 決定事項

### D1. 変換方式: 純 Rust の `svg2pdf` によるベクタ PDF

SVG → PDF 変換に [`svg2pdf`](https://crates.io/crates/svg2pdf)（typst プロジェクト製）を採用し、**ベクタ PDF** を出力する。

- 採用理由:
  - **既存依存ツリーと完全整合**: `svg2pdf 0.13.0`（最新安定）は `usvg ^0.45` / `pdf-writer ^0.12` に依存し、本プロジェクトの `usvg 0.45.1`（`resvg 0.45.1` 経由）と一致する。`usvg` の `Tree` をそのまま PDF 化できるため、PNG と同じ「SVG 文字列 → `usvg::Tree`」の前処理を共有できる。
    - **【訂正】** ここで述べた「PNG と usvg の前処理を共有できる／usvg 世代の一致が統合上の要件」という認識は誤り。実装上 `png.rs` は `resvg::usvg`、`pdf.rs` は `svg2pdf::usvg` と**各々の re-export を用いて別個に `Tree` を構築**しており、単一の `Tree` を共有しない。resvg と svg2pdf の usvg 世代が偶然一致しても correctness 上の要件ではなく、resvg は独立に更新してよい。実際に固定が必要なのは `svg2pdf` ↔ `pdf-writer` のみ。詳細は後述の補遺「svg2pdf / pdf-writer のバージョン結合」を参照。
  - **純 Rust・システム依存なし**: 実行時に外部バイナリ（ブラウザ等）やシステムライブラリ（cairo / librsvg 等）を要求しない。CI の offline 前提、cross-compile（Windows / `x86_64-unknown-linux-musl`）、Homebrew / `cargo binstall` でのバイナリ配布と整合する。
  - **ベクタ品質**: 受け入れ条件「ベクタ PDF が望ましい」を満たす。テキスト・図形がベクタのまま埋め込まれ、拡大しても劣化しない。
  - **既存 `png` feature と同型に実装可能**: モジュール構成・feature ゲート・CLI 配線を PNG にミラーでき、保守負荷が小さい。

### D2. feature・依存設計

- `crates/tdsl-render/Cargo.toml` に opt-in feature を追加する:
  - `[features] pdf = ["dep:svg2pdf", "dep:thiserror"]`
  - `svg2pdf = { version = "0.13", optional = true }`
- `svg2pdf` のバージョンは `usvg` のメジャー系列（0.45）に追従させる。`usvg` を上げる際は `svg2pdf` の対応バージョンを同時に確認する（両者は `usvg` の型を共有するため、不整合だとコンパイルエラーになる）。**【訂正】** 実際のバージョン結合相手は `usvg` ではなく `pdf-writer` だった（#368 で実装を `to_chunk` + `pdf-writer` に変更したため）。経緯と正しい運用は後述の補遺「svg2pdf / pdf-writer のバージョン結合」を参照。
- WASM（`tdsl-wasm`）は `pdf` feature を**取り込まない**（PNG と同方針。ブラウザでの PDF 生成は需要が薄く、バンドルを slim に保つ）。
- CLI（`tdsl-cli`）は `tdsl-render = { features = ["png", "pdf"] }` で依存する。

### D3. モジュール構成

- 新規 `crates/tdsl-render/src/pdf.rs` を `png.rs` と同型に作る:
  - `PdfError`（`thiserror::Error`。SVG パース失敗 / PDF 変換失敗を表現）
  - `PdfOptions`（将来のオプション拡張余地。初版は空〜最小で良い）
  - `render_pdf(ir: &TimelineIr, opts: RenderOptions, pdf_opts: PdfOptions) -> Result<Vec<u8>, PdfError>`
  - `svg_to_pdf(svg_str: &str, pdf_opts: PdfOptions) -> Result<Vec<u8>, PdfError>`（SVG 文字列を保持済みの呼び出し側・テスト向けに分離）
- `crates/tdsl-render/src/lib.rs` で `#[cfg(feature = "pdf")] pub mod pdf;` とゲートし、`PdfError` / `PdfOptions` / `render_pdf` / `svg_to_pdf` を re-export する（`png` の re-export と対称）。

### D4. CLI 配線

- `crates/tdsl-cli/src/main.rs` の `enum RenderFormat` に `Pdf` を追加し、`--format pdf` を受け付ける。
- `cmd_render` の `match format` に `RenderFormat::Pdf => { ... }` を追加し、`render_pdf` の戻り（`Vec<u8>`）をバイナリ出力としてファイル / stdout に書き出す（PNG の出力経路を踏襲）。

### D5. 付随する決定事項

- **依存サイズ**: `svg2pdf` 追加で CLI バイナリが膨らむ場合も、opt-in feature（default off）のままとし、WASM・ライブラリ利用者には影響させない。
- **フォント / CJK の扱い**: 出力品質の鍵。詳細は「既知リスク」を参照。実装時に PNG と同じ `fontdb` システムフォント読み込み（`Options::fontdb_mut().load_system_fonts()`）を `usvg` パース時に行う方針とする。
- **エラー整形**: ライブラリ層は `thiserror` ベースの `PdfError` を返し、CLI 層で `miette` 整形 / `format!` メッセージに載せる（既存 `PNG rendering failed: {e}` と同様）。

## 比較した代替案

| 方式 | 出力 | 判定 | 理由 |
|------|------|------|------|
| **`svg2pdf`（採用）** | ベクタ | ✅ 採用 | 純 Rust。依存ツリーにある `usvg 0.45` の `Tree` をそのまま PDF 化。システム依存なし → CI offline・cross-compile・バイナリ配布と整合。`png` feature と同型に実装できる。 |
| `resvg`/`tiny-skia` でラスタ化し PDF に画像埋め込み（`printpdf` 等） | ラスタ | ❌ 不採用 | ベクタ品質を失う（拡大で劣化）。`printpdf` 単体は SVG を解釈しないため別途ラスタ化が必要で構成が複雑。ただし将来 `svg2pdf` が非対応の SVG 機能に遭遇した場合の**フォールバック候補**としては記録に残す。 |
| `cairo`（`cairo-rs` + `librsvg`） | ベクタ | ❌ 不採用 | `libcairo` / `librsvg` のシステムライブラリが必須。offline / static ビルドが破綻し、cross-compile（Windows / musl）と CI セットアップが重くなる。純 Rust 方針に反する。 |
| `chromiumoxide` / `headless_chrome` | ベクタ / ラスタ | ❌ 不採用 | 実行時に Chromium バイナリを要求する。CLI 配布（Homebrew / `cargo binstall`）と CI offline 前提に致命的に不適合。起動コスト・依存サイズも過大。 |

## 影響範囲

本 ADR は決定の記録のみ。実装は #265 の残受け入れ条件として後続で行う。実装時に変更が見込まれるファイルは次のとおり。

- **新規**: `crates/tdsl-render/src/pdf.rs`（`png.rs` をミラー）
- `crates/tdsl-render/Cargo.toml` — `pdf` feature と `svg2pdf` optional 依存の追加（D2）
- `crates/tdsl-render/src/lib.rs` — `#[cfg(feature = "pdf")]` ゲートと re-export（D3）
- `crates/tdsl-cli/Cargo.toml` — `tdsl-render` の features に `pdf` 追加（D2）
- `crates/tdsl-cli/src/main.rs` — `RenderFormat::Pdf` と `--format pdf` 配線（D4）
- `docs/cli-spec.md` / `README.md` / `README.ja.md` — 出力フォーマット一覧に PDF を追記
- `CHANGELOG.md` — PDF 出力追加の記録
- `examples/` の 1 サンプルで PDF 出力の smoke test（`scripts/e2e-smoke.sh` への追加を想定）

## 既知リスク

- **CJK フォントの埋め込み / アウトライン化**: `svg2pdf` のテキスト処理（`usvg` による text-to-path もしくはフォント埋め込み）で、日本語レーンラベルが PDF 上で正しく表示・選択可能になるかは実装時に検証が必要。`png.rs` と同様に `usvg` パース時へシステムフォントを読み込ませる必要があり、フォントが見つからない環境（最小 CI ランナー等）では字形が欠落する可能性がある。smoke test では「PDF マジックバイト（`%PDF-`）で始まること」と「妥当なサイズであること」を最低ラインとし、字形の目視確認は別途行う。
- **`svg2pdf` / `pdf-writer` のバージョン結合**: `pdf.rs` は `svg2pdf::to_chunk` が返す `Chunk` の `Ref`（svg2pdf 内部の pdf-writer）と自前 `pdf-writer` の `Ref` を混在させるため、両者が同一の pdf-writer バージョンに解決される必要がある。不整合だと pdf-writer が二重リンクされ `Ref` の型不一致でビルドが壊れる。Dependabot 等での自動更新時は両者を同時に上げる運用とする（詳細・経緯は後述の補遺「svg2pdf / pdf-writer のバージョン結合」を参照）。なお `usvg` は `svg2pdf::usvg`（re-export）経由でのみ利用するため svg2pdf に自動追従し、個別固定は不要（当初の本リスク記述が `usvg` を結合相手としていたのは誤りで、補遺で訂正済み）。
- **SVG 機能カバレッジ**: 本プロジェクトの SVG 出力（`svg.rs`）が将来 `svg2pdf` 未対応の機能（特定のフィルタ・グラデーション等）を使い始めた場合、PDF で再現されない可能性がある。現状の出力は矩形・線・テキスト・単色塗りが中心で対応範囲内と見込む。

## 未決定事項（本 ADR の範囲外）

- `PdfOptions` に持たせる将来オプション（用紙サイズ / マージン / メタデータ埋め込み等）の具体仕様。初版は最小とし、需要が出た時点で別途検討する。
- ラスタ埋め込み方式（フォールバック）への切り替え基準。`svg2pdf` で品質問題が顕在化した場合に再評価する。
- PDF/A など印刷・長期保存規格への準拠。現時点で要求がないため対象外。

## 補遺: PdfOptions 拡張（2026-06-07, #368）

本 ADR の「未決定事項」に挙げた `PdfOptions` の拡張が #368 で実装された。

- **実装方式**: `svg2pdf::to_chunk` + `pdf-writer` によるページ自前合成に切り替え。`svg2pdf::to_pdf`（`PageOptions.dpi` のみ、メタデータ・ページサイズ未対応）では要件を満たせないため。
- **追加フィールド**:
  - `page_size: PdfPageSize`（A4 / A3 / Letter、portrait pt 値を内部保持）
  - `landscape: bool`（横向き時に w/h を swap）
  - `margin_mm: f64`（mm → pt 変換後に content area を計算）。負値・非有限（NaN/Inf）、および印刷可能領域が残らない過大値（`2 × margin ≥ 用紙短辺`）は `PdfError::InvalidMargin` で明示的に拒否する。空白・破損 PDF を黙って生成するクランプ方式は採らない（本 ADR のコンテキスト「Explicit error over silent fallback」に従う）。
  - `title: Option<String>`（None のとき `render_pdf` が `ir.meta.title` で補完）
  - `creation_date: Option<PdfDate>`（呼び出し側が供給し決定性を保つ。CLI は `SystemTime::now()` で算出、テストは任意値）
- **CLI フラグ**: `--pdf-size` / `--pdf-landscape` / `--pdf-margin` / `--pdf-title` を `tdsl render` に追加。
- **テスト**: `pdf::tests` に各用紙サイズ・landscape・メタデータ・title 補完・マージン検証（過大／負値／非有限はエラー、有効な大マージンは描画）のテストを追加。

## 補遺: svg2pdf / pdf-writer のバージョン結合（2026-06-10, #415 / #416）

D2 および当初の「既知リスク」は依存結合の相手を **`usvg` ↔ `svg2pdf`** と記述していたが、これは誤りだった。本補遺で結合相手を **`svg2pdf` ↔ `pdf-writer`** に訂正する。

- **誤りの経緯**: 初版の `svg2pdf::to_pdf` 実装では `pdf-writer` を直接触らなかったため、結合は usvg 型ツリー側にあると見なしていた。しかし #368 の補遺で実装を `svg2pdf::to_chunk` + `pdf-writer` 自前合成に切り替えた時点で、真の結合は **pdf-writer** に移っていた（`pdf.rs` が svg2pdf の `Chunk` の `Ref` と自前 `pdf-writer` の `Ref` を混在させる）。契約類（Cargo.toml コメント / D2 / 既知リスク）はこの変化に追従できていなかった。
- **顕在化**: Dependabot PR #415 が `pdf-writer` を 0.12 → 0.15 に単独 bump したところ、svg2pdf 0.13（`pdf-writer ^0.12` 要求）との間で pdf-writer が二重リンクされ、`expected pdf_writer::Ref, found pdf_writer::object::Ref` の型不一致でビルドが失敗した。
- **正しい結合**: `svg2pdf` と `pdf-writer` は同一 pdf-writer バージョンに解決される必要があり、**両側を lockstep** で更新する。一方だけ bump させると逆向きにも同じ破綻が起きる。
- **`usvg` について**: `usvg` は `svg2pdf::usvg`（re-export）経由でのみ利用し、`usvg::Tree` を resvg と svg2pdf の間で受け渡さない（`png.rs` は `resvg::usvg`→`resvg::render`、`pdf.rs` は `svg2pdf::usvg`→`svg2pdf::to_chunk` で各々完結）。したがって usvg 世代を resvg と揃える必要はなく、`resvg` は独立して更新してよい。
- **運用への反映**: `.github/renovate.json5`（旧 `.github/dependabot.yml`、2026-07-24 に Renovate へ移行）の packageRules で `svg2pdf` / `pdf-writer` の minor/major 更新を無効化し（patch は許可）、上流が新版を出した時点で両者をまとめて手動更新する。`resvg` は抑止対象に含めない。

## 補遺: WebUI の PDF 出力（2026-06-03, #364）

本 ADR の D2 は「ブラウザでの PDF 生成は需要が薄く、バンドルを slim に保つ」として WASM に `pdf` feature を取り込まない決定をした。その後 #364（WebUI のエクスポート統合）で WebUI からの PDF 出力需要が顕在化したため、方式を改めて検討した。

- **決定**: WebUI の PDF 出力は **ブラウザのネイティブ印刷（print-to-PDF）** で実現する。`render_html` の HTML 出力を非表示 iframe に読み込み、`iframe.contentWindow.print()` を呼ぶ。ユーザーが印刷ダイアログで「PDF に保存」を選ぶ。
- **理由**:
  - **CJK が正しく出る**: ブラウザが自前のフォントスタックで日本語ラベルをシェイプする。WASM 経由（`svg2pdf` + `fontdb`）では D5 のシステムフォント読み込みがブラウザサンドボックスで機能せず、字形が欠落する。
  - **バンドル非肥大**: `svg2pdf` / `usvg` と CJK フォント埋め込み（数 MB）を WASM に持ち込まずに済み、D2 の slim 方針を維持できる。
  - **追加依存ゼロ**: 新たな Rust / JS 依存を増やさない。
- **D2 への影響**: D2 は**維持**する（CLI はベクタ PDF、WASM は `pdf` feature を取り込まない）。本補遺は D2 を supersede しない。WebUI の PDF は CLI のベクタ PDF とは**別経路**（ブラウザ印刷）である点を記録に残す。
- **トレードオフ**: 印刷ダイアログを経由するため CLI のような単発のファイル生成ではない。出力の体裁はブラウザの印刷レンダリングに依存する。ベクタ品質の単一ファイル PDF が将来 WebUI で必要になった場合は、フォント埋め込み込みの WASM 化を別 ADR で再評価する。
