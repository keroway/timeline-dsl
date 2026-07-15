# tdsl-render

`TimelineIr` を HTML / SVG / PNG / PDF へレンダリングするクレート。

## PDF pagination のテスト戦略（ADR 0004 D7）

`--pdf-pagination`（`crates/tdsl-render/src/pdf.rs`）は、**ゴールデン画像比較を使わない**方針を採用している（ADR 0004 D7、および「未決定事項」節）。理由は以下のとおり:

- ローカル外部PDFビューア・OS依存のフォントレンダリングに依存する画像比較は、CI環境間で再現性が低い。
- ページ分割の正しさは「何ページ生成されるか」「各ページに見出し/ページ番号が存在するか」という**構造**の問題であり、ピクセル単位の見た目の一致は要求しない。

代わりに、次の2種類の**決定的な構造検証**を組み合わせている:

1. **ページ数アサーション** — 生成PDFのページオブジェクト数（`page_object_count()`、`/Type/Page` の出現数から `/Type/Pages` を除いた数）が、テーブル行数・用紙サイズ・向き・余白から計算した期待値と一致することを確認する。
2. **構造的存在検証** — 中間SVG文字列（`render_pdf_svg_pages()`、PDF変換前の各ページのSVG）に対して、列見出し（`TABLE_COL_TIME` 等）・ページ番号フッタ（`"i / N"`）・CJKテキストが実際に含まれることを文字列一致で確認する。バイナリPDF内のテキストは `svg2pdf` によって XObject として埋め込まれ、生バイトから中身をgrepでは検証できないため、変換前のSVG段階でアサーションする。

### `render_pdf_svg_pages` を使う理由

`render_pdf()` はSVG生成とPDF変換を一体で行う公開APIだが、テストが実際の分岐（pagination有効/無効、`show_table` の強制など）を経由して検証できるよう、ページ生成ロジックだけを `render_pdf_svg_pages()`（`pdf.rs` 内 `pub(crate)` 相当、テストモジュールから直接呼び出し可能）として切り出している。

**テストを書く際は、`render_pdf()` の分岐を再実装した独自ヘルパーを作らないこと。** 過去に「pagination有効/無効で同じ関数を2回呼ぶだけ」のトートロジーテストが紛れ込み、実際の分岐を経由しない誤った不変条件テストになった経緯がある（#620 reviewer指摘）。必ず `render_pdf()` または `render_pdf_svg_pages()` を実引数を変えて呼び出し、その戻り値を比較・assertすること。

### 用紙サイズ・向きのページ数計算式

`table_rows_per_page()`（`pdf.rs`）は以下の式でページあたりの完全なデータ行数を計算する:

```
content_height = page_height - 2 * margin  (mm→ptに変換後)
complete_rows  = floor(content_height / TABLE_ROW_HEIGHT)   // TABLE_ROW_HEIGHT = 22.0pt
rows_per_page  = complete_rows - 1   // 先頭の列見出し行の分を1行差し引く
```

新しい用紙サイズ・デフォルト余白を追加する場合は、`crates/tdsl-render/src/pdf.rs` の `pagination_page_count_matrix_across_page_size_and_orientation` テストのコメントにある計算式を使って期待ページ数を再計算し、テストケースを追加すること。

### 新しいレイアウト機能を追加する場合の拡張手順

`--layout-style` に新しいレイアウト（group-bands/gantt/zigzag 以外）を追加した場合、ADR 0004 D1 により「タイムライン本体はページ分割の対象外」という不変条件は変わらない。以下を追加すること:

1. `pdf.rs` の `pagination_does_not_change_timeline_page_svg_with_*` パターンに倣い、新レイアウトでも `render_pdf_svg_pages(ir, opts, &PdfOptions{pagination:false,..})` と `pagination:true` の `pages[0]`（タイムラインページ）が一致することを確認するテストを追加する。
2. 新レイアウト固有のCSSクラス（例: `tdsl-grid-gantt`）が実際にそのページに含まれることをサニティチェックとして確認する（不変条件テストが「常に空文字列同士を比較して常に真になる」退化を防ぐため）。

同様に、CJKテキスト・`color_map` テーマなど「タイムライン本体の見た目にのみ影響する要素」を追加した場合も、テーブルページの内容・ページ数には影響しないことを検証するテストを追加する（`cjk_table_rows_paginate_with_expected_page_count_and_repeated_header` / `color_map_theme_does_not_affect_pagination_or_page_count` を参照）。

### ゴールデン画像比較を将来導入する場合

ADR 0004 の「未決定事項」に記載のとおり、視覚的なゴールデン画像比較の導入自体は本ADRのスコープ外として棚上げされている。将来導入する場合は、OS/フォントバージョン依存を避けるためにDockerなどで固定されたレンダリング環境を用意し、新しいADRとして別途設計判断を行うこと。既存の構造検証テストは、画像比較を導入した後も「軽量な決定的リグレッションガード」として維持する想定である。
