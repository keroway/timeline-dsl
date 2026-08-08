# ADR 0005: タイムライン本体（チャート部分）の複数ページ化

- **Status**: Accepted（設計方針として承認。lane グループ軸は Spike #651 → 本実装 #660 → PDF 統合 #661 で完了。時間範囲軸の分割は Spike #662（#709/#710/#711）で検証完了し D3 で本実装 GO 判断、本実装は #729 → #733（コア昇格・CLIフラグ）/ #734（継続マーカー描画）/ #736（PDF統合）/ #735（PDF警告配線、#736 に吸収してクローズ）で完了。詳細は D4 参照）
- **Date**: 2026-07-21
- **Deciders**: keroway（承認済み、2026-07-21）
- **Related issues**: #649（本 ADR）, #609（親: paginated PDF export, ADR 0004 が分岐元）
- **Supersedes**: なし。ADR 0004「PDF ページ分割戦略（テーブルのみを対象とする縮小スコープ）」の D1・未決定事項で「将来必要になった時点で別ADRとして再評価する」とされた項目への回答として位置づける。ADR 0004 の D2〜D7（テーブルページ分割の仕様）は変更しない。

## コンテキスト

ADR 0004（#617〜#622, v1.27.0 で実装済み）は `--pdf-pagination` を「アイテムテーブルのページ分割」のみに限定し、タイムライン本体（チャート部分）は現行どおり単一ページに縮小して描画する設計を採用した。その理由は、チャート分割がページ境界をまたぐバー（span/event_range）のクリッピング・視覚的連続性の保証・group band やレーンラベルの再描画など、レイアウトエンジンを実質的に新規設計する規模の作業になり、親issue #609 の Effort L / Risk HIGH という見積もりに見合う検証時間が確保できなかったためである（ADR 0004 D1・既知リスク）。

本 ADR は、その「将来必要になった場合の再評価」を行う。**現時点では実装 GO の結論を出すものではなく、設計選択肢を整理し、次に着手する場合の出発点を明確にすることが目的**である。

## 検討事項

### 1. ページ分割軸

| 軸 | 概要 | 課題 |
|---|---|---|
| 時間範囲で分割 | タイムラインの時間軸を N 区間に分け、各区間を1ページに描画 | 区間境界をまたぐ span/event_range のクリッピング・「...続く」マーカーの要否。lane 間の相対的な時系列比較が区間をまたぐと分断される |
| lane グループで分割 | lane を N グループに分け、グループごとに1ページ（時間軸は共通） | 同時期の別 lane イベントの対比が分断される（ADR 0004 の代替案比較で既に「可読性がかえって下がる懸念」として却下理由に挙げられている） |
| 両方（時間範囲 × lane グループ） | 上記の直積でページを生成 | ページ数が組み合わせ的に増加し、印刷物としての実用性が低下する可能性。UI/UXの複雑さが最も高い |

現時点の暫定評価: 時間範囲分割はページをまたぐ span のクリッピング処理という実装コストが本質的に避けられない一方、lane グループ分割は既存の「1ページに全 lane を収める」構造をレーン単位で素直に分割できるため実装コストが相対的に低い。ただし lane グループ分割は「同時期の異なる出来事を並べて見る」というタイムライン本来の価値を損なう可能性があり、トレードオフは自明ではない。**両案とも本 ADR の時点では決定しない**（次節「未決定事項」参照）。

### 2. ページ境界をまたぐ span/event_range の扱い

時間範囲分割を採用する場合、以下のいずれかの戦略が必要になる:

- **クリップして継続マーカーを表示**: 各ページで span を可視区間にクリップし、次ページに続く場合は矢印等の継続インジケータを描画する。実装コストは高いが情報の欠落がない。
- **開始ページにのみ描画し、以降のページでは省略**: 実装は単純だが、後半ページだけを見た読者には span の存在が分からない（silent 情報欠落であり、implementation-strict.md §1「Explicit error over silent fallback」の精神に反する懸念がある）。
- **span が属する「主ページ」（開始時点を含むページ）に全体を縮小して描画し、後続ページには影響しない**: 現在の PDF ページ分割（ADR 0004）と親和性が高いが、非常に長い span（例: 王朝の存続期間が10ページ分にまたがる）では視覚的に意味をなさない。

いずれの戦略も未検証であり、プロトタイプなしに選択することはリスクが高い。

### 3. group band / gantt / zigzag / open-ended range の分割時の振る舞い

ADR 0004 D5 は「group band / gantt / zigzag / open-ended range はタイムライン本体（1ページ目）の描画にのみ関わり、テーブルページ分割の影響を受けない」と整理した。本 ADR がスコープとする**チャート本体自体の分割**では、これらの機能はいずれも新たな検討が必要になる:

- **group band**: グループの開始・終了がページ境界をまたぐ場合のバンド描画（帯そのものの分割）
- **gantt レイアウト**: 依存関係線がページをまたぐ場合の表現
- **zigzag レイアウト**: ジグザグの折り返し位置とページ境界の整合
- **open-ended range（`now` 終端）**: 「現在」を指す矢印がページ境界と衝突しないか

これらはいずれも ADR 0004 のスコープ外だったため未検証であり、本 ADR の時点でも仕様を確定できない。

### 4. 既存 `--pdf-pagination` との関係

- **同一フラグを拡張**: `--pdf-pagination` の意味を「テーブル分割」から「テーブル + チャート分割」に広げる案。ただし ADR 0004 D3 は「既存の `--format pdf` 単体・`--show-table` 単体の出力は完全に不変（後方互換）」を明記しており、フラグの意味変更は既存ユーザーの出力を変える可能性があるため、単純な意味拡張は避けるべきである。
- **新フラグを追加**（例: `--pdf-chart-pagination` や `--pdf-pagination=table,chart` のような値付きオプション）: 既存 `--pdf-pagination`（bool）の後方互換を壊さずに済む。現時点ではこちらが有力候補だが、CLI オプション設計は実装 GO 判断後に別途固める。

### 5. テスト戦略（ADR 0004 D7 を踏襲できるか）

ADR 0004 D7 は「ページ数アサーション」「構造的アサーション（見出し・ページ番号の存在検証）」に基づく決定的テストを採用し、ゴールデン画像比較を避けた。チャート分割でも同じ方針は踏襲可能と考えられる（span のクリップ座標・継続マーカーの存在検証など、構造検証で代替できる）。ただし境界ケース（1レコードが複数ページにまたがる場合の座標計算）の期待値をどう決定的に定義するかは、実装時にプロトタイプを作って初めて分かる部分が大きい。

### 6. Effort / Risk の再評価

親issue #609 時点の見積もり（Effort L, Risk HIGH）は、本 ADR の検討を経ても変わらないと判断する。理由:

- ページ境界をまたぐ span のクリッピング・継続表示は新規レイアウトエンジンの設計に相当し、ADR 0004 が回避した課題がそのまま残っている。
- group band / gantt / zigzag / open-ended range という4つの既存レイアウト機能すべてに対してページ分割時の振る舞いを再定義する必要があり、組み合わせテストの範囲が広い。
- 実装配置は `crates/tdsl-render/src/pdf.rs`（ページ分割ロジック）と `svg.rs`（チャート描画関数）を踏襲する前提だが、いずれも既存コードへの追加ではなく、チャート描画のコア関数群への構造的な変更を要する可能性が高い。

## 決定事項

### D1. 本 ADR の時点では実装方式を確定しない（承認済みだが、実装方式は Spike プロトタイプ issue の結果まで確定しない決定）

ページ分割軸（時間範囲 / lane グループ / 両方）、境界をまたぐ span の表示戦略、CLI フラグ設計のいずれも、プロトタイプなしに一意に決定するには材料が不足している。本 ADR は選択肢と評価軸を整理するに留め、GO 判断とプロトタイプ実装は implementation-strict.md §2 NO-GO フローチャートに従い、**着手前にユーザー確認を必須とする別issueに分割する**。

### D2. 次に着手する場合の推奨出発点

もし着手する場合、以下の順序を推奨する（コミットするものではなく、次の spike issue への申し送り事項）:

1. lane グループ分割（時間軸共通）を先にプロトタイプする。span クリッピングが不要なため実装コストが最小で、チャート分割全体のリスクを先に検証できる。
2. 時間範囲分割（span クリッピング必須）は、lane グループ分割の知見を得た後に着手する。
3. group band / gantt / zigzag / open-ended range は、いずれか1つのレイアウトスタイル（例: 通常の span/event のみ）でチャート分割が動くことを確認してから、順次対応範囲を広げる。

## 比較した代替案

| 方式 | 判定 | 理由 |
|---|---|---|
| 本 ADR で実装方式まで確定する | ❌ 不採用 | プロトタイプ・検証なしに span クリッピング戦略や CLI フラグ設計を確定すると、ADR 0004 が経験した「Effort/Risk に見合う検証時間の不足」を再び踏むリスクが高い |
| 選択肢整理 + 推奨出発点の申し送りに留める（採用） | ✅ 採用 | ADR 0004 の轍を踏まず、次の spike issue（プロトタイプ付き ADR、ADR 0003/0004 の `(Spike)` パターンを踏襲）で実測を伴って再評価できる |
| このニーズ自体を却下（対応しない） | ❌ 不採用 | issue #649 の起票理由（親issue #609 の暗黙の期待）が消えたわけではなく、却下するにはユーザー側のプロダクト判断が必要。本 ADR の権限外 |

## 影響範囲

本 ADR 自体はドキュメントのみの変更であり、コード変更は伴わない。

- `docs/adr/0005-timeline-chart-pagination.md`（本ファイル、新規）
- 実装が GO と判断された場合の想定変更範囲（未確定、参考情報）: `crates/tdsl-render/src/pdf.rs`（ページ分割ロジック）, `crates/tdsl-render/src/svg.rs`（チャート描画関数群）, `crates/tdsl-cli/src/commands/render.rs`（CLIフラグ）, `docs/cli-spec.md` / `docs/dsl-spec.md`

## 既知リスク

- 本 ADR は「実装しない」判断ではなく「今は決めない」判断であり、issue #649 は `needs-refinement` のまま残る。次サイクルで優先度が上がらない限り着手されない可能性がある。
- lane グループ分割を先行プロトタイプする推奨（D2）は、時間範囲分割こそが本来のニーズ（長期タイムラインの分割）である可能性を考慮すると、優先順位が逆かもしれない。プロトタイプ着手時に再検討すべき。

## Spike 実施結果（issue #651, lane グループ分割プロトタイプ）

D2 の推奨に従い、lane グループ単位でのチャート分割を先行プロトタイプした（`crates/tdsl-render/src/svg_pagination_spike.rs`、`#[cfg(test)]` 限定・本番配線なし。issue #660 で `crates/tdsl-render/src/pagination.rs` へ本番昇格済み、spike ファイルは削除済み）。

### 実装アプローチ

`LayoutModel::compute` / `render_svg` は一切変更していない。既存パイプラインをそのまま再利用し、以下の手順のみを追加した:

1. `TimelineIr.lanes` を `LayoutModel::compute` と同じ順序（`(order, id)`）でソート。
2. `lanes_per_page` 件ずつチャンク分割。
3. チャンクごとに「同じ `meta`（時間軸を共通に保つ）+ そのチャンクの lane のみ + それらの lane に属する item のみ」を持つ `TimelineIr` を複製生成。
4. チャンクごとの `TimelineIr` を通常どおり `LayoutModel::compute` → `render_svg` に通し、ページ数分の SVG 文字列を得る。

### 検証できたこと

- **span/event_range のクリッピングは原理上発生しない**: `Item::lane` は単一の lane ID を持つフィールドであり、lane 軸でページを分割する限り、どの item も必ずちょうど1つのページ（1つの lane チャンク）に完全に属する。これはテスト `every_item_appears_on_exactly_one_page` で構造的に検証済み（4 item を 2 ページに分割し、各 item のラベルがちょうど1ページの SVG にのみ出現することを確認）。
- **実装コストは見積もりどおり最小**: 新規 lowering / レイアウトエンジン変更は不要で、既存 `TimelineIr` を lane 部分集合でフィルタして複製し、既存関数を複数回呼ぶだけで動いた。差分はおよそ 200 行（大半はテスト）。
- **group band はページ境界をまたぐと分断される（既知の課題）**: `group_bands` は `LayoutModel::compute` 内部で「現在渡された lane 集合」からその場で再計算されるため、ページ分割によって同じ `Lane::group` の lane が複数チャンクにまたがると、各ページは自分に見えている lane だけから独立に（切り詰められた）group band を再構築する。バンドが「1つの連続した帯」として复元されることはなく、エラーにもならず、単に静かに切り詰められた帯が各ページに描かれる。テスト `group_band_split_across_page_boundary_is_detected`（境界をまたぐケースの検出）と `split_group_band_still_renders_a_truncated_band_on_each_page`（クラッシュせず切り詰められた帯が各ページに描画されることの確認）で構造的に記録した。

### 想定外だった点

- group band の切り詰めは「クラッシュ」でも「情報欠落エラー」でもなく、見た目としては「1つのグループが複数の異なる帯に分かれて見える」という *サイレントな視覚的劣化* に近い。implementation-strict.md の "no silent fallback" 原則に照らすと、本番導入時はこの検出結果（`group_bands_split_across_pages` 相当の情報）をユーザーに警告として表示する仕組みが必須になる（現在の CLI/WebUI の `zigzag_fallback` 警告パターンと同様の扱いが妥当）。
- `Meta.range` を全ページ共通に保つ設計は素直に機能したが、`show_table` / `show_legend` はページ非対応のまま（今回のプロトタイプでは検証していない）。テーブル・凡例をページごとに複製すべきか、末尾ページにのみ出すべきかは次 issue の検討事項として残る。

### 次 issue への申し送り事項

- 本番統合する場合、`group_bands_split_across_pages` に相当する情報を CLI/WebUI が検出し、`zigzag_fallback` と同様の「サイレントにしない」警告経路を設計すること。
- `--pdf-pagination` との統合方針（第4節「既存 `--pdf-pagination` との関係」で未決定のまま）は本 Spike でも決定しなかった。今回の SVG 分割はまだ PDF 生成パイプライン（`crates/tdsl-render/src/pdf.rs`）とは接続していない。
- D2 の次ステップである「時間範囲分割（span クリッピング必須）」は本 Spike のスコープ外のまま。lane グループ分割で得られた「既存パイプラインの再利用が容易」という知見は時間範囲分割には直接適用できない（span を実際にクリップする新規ロジックが必要になる）ため、着手時は改めて Effort/Risk を見積もり直すこと。

## 実装時の決定（issue #660）

Spike（issue #651）で得られた知見をもとに、lane グループ単位のチャート分割を本番配線した（`crates/tdsl-render/src/pagination.rs`、`crates/tdsl-cli`）。第4節「既存 `--pdf-pagination` との関係」と「次 issue への申し送り事項」で残されていた論点は以下のとおり確定した。

- **CLI フラグ**: 既存 `--pdf-pagination`（bool、テーブル分割専用）の意味は変更せず、新規フラグ `--chart-pagination <N>`（1 ページあたりの lane 数）を追加した。両フラグは独立しており、`--chart-pagination` は `--format svg` のみで有効（`--format pdf` との併用は明示エラー。PDF 統合は #661 で改めて設計する）。
- **出力方式**: `--output china.svg` を渡すと `china.page1.svg` / `china.page2.svg` … に分割出力する（連番幅は総ページ数の桁数に合わせて0埋め）。`--output` 省略時は明示エラー（stdout は複数ファイルを表現できないため）。
- **`--show-legend` の扱い**: 各チャートページに個別描画する（そのページの item に対応した凡例が、既存の `LayoutModel::compute` → `render_svg` パイプラインでそのまま生成される）。ページごとに異なる凡例内容になり得るのは意図した挙動。
- **`--show-table` の扱い**: 「最終チャートページに載せる」のではなく、チャートページ群の**後ろに専用のテーブルページを 1 枚**追加し、**IR 全体**の行を載せる方式を採用した。理由: 各チャートページの IR はそのページの lane の item しか持たないため、最終チャートページに表を描くと「最後の lane の item だけの表」になり索引として誤解を招く。既存の `svg::render_table_page_svg` / `layout::collect_table_rows`（ADR-0004 実装済み）をそのまま再利用しており、複数テーブルページへの分割（テーブル自体が用紙に収まらない場合）は #661 のスコープとして残した。
- **group band 分断の警告経路**: Spike で識別された「サイレントな視覚的劣化」（想定外だった点）に対し、`ChartPagination.group_bands_split_across_pages` を CLI 層で必ず `eprintln!("Warning: ...")` として警告する経路を実装した（`--layout-style zigzag` の `zigzag_fallback` 警告パターンを踏襲）。出力そのものは生成する（エラーにはしない）。
- **span/event_range クリッピング**: Spike の構造的な結論（lane 軸分割では原理上不要）どおり、本実装でも `LayoutModel::compute` / `render_svg` は無変更のまま再利用しており、クリッピングロジックの新規実装は発生しなかった。
- **スコープ外として残したもの**: 時間範囲分割（span クリッピングが必要、issue #662・`needs-refinement`）、PDF 統合（issue #661）、WebUI/WASM への配線（別途起票が必要な場合のみ）。

## 実装時の決定（issue #661: PDF 統合）

issue #660 で `--format svg` 限定だった `--chart-pagination` を `--format pdf` にも統合した（`crates/tdsl-render/src/pdf.rs`）。issue #660 の「スコープ外として残したもの」に挙げた PDF 統合はこれで解消した。

- **ページ構成**: 「チャートページ群（lane グループ順）→ テーブルページ群」の順で単一 PDF ファイル内に固定。SVG 版のように別ファイルには分割しない。
- **`--show-table` の扱い**: `--pdf-pagination` なしなら IR 全体を 1 枚の未分割テーブルページとして末尾に追加。`--pdf-pagination` を併用すると、既存の行分割ロジック（ADR-0004）でテーブルページ群を生成する。いずれの場合もテーブルページの `i / N` フッタはテーブルページ数のみを数え、先行するチャートページ数は含めない。
- **後方互換**: `--chart-pagination` を指定しない既存の `--format pdf` 出力（単体 / `--show-table` / `--pdf-pagination` のいずれも）は完全に不変（ADR-0004 D3 の制約を維持、回帰テストで保証）。
- **API**: `crates/tdsl-render` に `PdfOptions::chart_pagination: Option<usize>`（デフォルト `None`）と、group band 分断警告を返す新 API `render_pdf_with_warnings()` を追加した（既存の `render_pdf()` はラッパーのまま不変）。
- **テスト**: A4/A3/Letter × 縦横向きの決定的テストマトリクス（ADR-0004 D7 パターン）にチャート分割ケースを追加。

## Spike 実施結果（issue #711、時間範囲軸の group band / gantt / zigzag / open-ended 相互作用）

issue #709（時間範囲軸チャートページ分割の spike 土台、`crates/tdsl-render/src/time_range_pagination_spike.rs`）が確立した「境界をまたぐ span/event_range を検出する」土台の上で、第3節が未検証としていた4機能（group band / gantt / zigzag / open-ended range）の時間範囲軸での振る舞いを実装・検証した。`#[cfg(test)]` 限定・本番未配線のまま、`time_range_pagination_spike.rs` に5件のテストを追加した。

### 検証できたこと

- **group band は時間範囲軸では原理上分断されない**: lane グループ軸（`pagination::paginate_svg_by_lane_groups`）は `lanes` をページごとにフィルタするため、同じ `Lane::group` の lane が複数チャンクにまたがると group band が分断される（issue #660 で確定済みの既知課題）。一方、時間範囲軸の `split_ir_by_time_range` は `lanes` を一切フィルタしない（全ページに全 lane を複製する）。さらに `layout::compute_group_bands` は band の主軸（時間軸方向）の範囲を lane の並びからではなく `total_width - left_gutter - right_margin`（そのページの描画幅そのもの）から導出しており、item の時間範囲にも依存しない。したがって group band の lane 構成はどのページでも同一になり、band は常にそのページの全幅に描画される — **分断・警告の概念自体が存在しない**。テスト `group_band_spans_full_page_width_on_every_time_range_page_no_truncation` で構造的に確認した。
- **gantt / zigzag のレイアウト計算は時間範囲軸でも一貫性が保たれる**: `layout::assign_zigzag_parity` と `layout::assign_bar_stack_levels`（gantt の期間ラベル衝突回避が使う土台）はいずれも `LayoutModel::compute` に渡された `TimelineIr` の**全 item**（範囲外フィルタ前）に対して計算される。lane グループ軸は `items` もページごとにフィルタするため、あるページに見えている item だけからその場で再計算される（group band と同じ「ページごとの部分集合から再計算」パターン）。一方、時間範囲軸は `items` を一切フィルタしないため、同じ item の zigzag parity はどのページで計算しても同一の値になる。テスト `zigzag_parity_for_a_shared_item_is_identical_across_time_range_pages` で、あるページにしか実在しない item (`s-3`) の zigzag オフセットが全4ページで一致することを確認した。
- **open-ended（`now` 終端）の「進行中」表示はページ境界と無関係に成立する**: `end_open` は item の静的フィールドであり、`split_ir_by_time_range` は item を無変更で複製するため、「進行中」ラベル（`layout::open_ended_end_label`）はその item が現れるどのページでも同一に表示される。これは第2節（境界をまたぐ span の表示戦略）が扱う「クリップ」の問題であり、open-ended 固有の新規ロジックは不要と判明した。テスト `open_ended_span_reads_ongoing_on_every_page_it_is_laid_out_on` で確認した。

### 想定外だった点（新規コスト/リスク）

- **範囲外 item も毎ページ全レイアウト計算を通る**: `Event` は `layout::year_in_range` によってページ範囲外なら早期リターンされるが、`Span`/`EventRange` にはこの除外がない。`layout::primary_axis_segment` のクランプにより非正の幅（0 以下）に収束するだけで、`LaidItem` としては必ず push される。時間範囲軸は item を一切フィルタしないため、ある item を一度も含まないページでもその item の座標計算・(Gantt 有効時は)期間ラベル衝突判定パスを毎回通る。クラッシュや誤描画（正の幅の bar が出る）は起きないが、ページ数 × item 数のオーダーで無駄な計算コストが発生する。lane グループ軸にはこの種の無駄はない（item ごと1ページにしか属さない）。テスト `items_wholly_outside_a_page_segment_still_produce_a_laid_item_with_non_positive_extent` で構造的に確認した。本実装時は、`split_ir_by_time_range` 相当の処理で `items` もページの `[start, end]` と交差するものだけに絞り込む最適化を検討する余地がある（正しさには影響しないが、ページ数が多いタイムラインでの計算コストに影響する）。

### 本実装 GO/NO-GO 判断材料

- **実装コスト見積もり**: #709 の spike 土台（`split_ir_by_time_range` / `items_crossing_boundaries`）と本 spike の検証結果を踏まえると、group band / gantt / zigzag / open-ended の4機能は**いずれも追加の分岐処理を必要としない**（既存の `LayoutModel::compute` パイプラインをそのまま複数回呼ぶだけで正しく動く）。本実装で新規に書く必要があるのは (a) #709 の境界またぎ検出結果を CLI 警告として配線する経路（lane グループ軸の `group_bands_split_across_pages` パターンを踏襲）と、(b) 上記の「範囲外 item のフィルタ最適化」（任意、パフォーマンス目的）のみ。第6節の Effort L 見積もりは、この4機能に関する限り縮小方向に再評価してよい。
- **テスト戦略の実現性**: ADR-0004 D7 パターン（ページ数・構造アサーション中心、ゴールデン画像比較なし）がそのまま踏襲可能であることを本 spike で実証した(5テストすべてが構造アサーションのみで完結)。
- **CLI フラグ設計案**: lane グループ軸の `--chart-pagination <N>` との対称性から、時間範囲軸は `--chart-pagination-range <N>` のような独立フラグ、または両軸を区別する値（例: `--chart-pagination lane:<N>` / `--chart-pagination range:<N>`）が候補になる。どちらも既存 `--chart-pagination <N>`（lane グループ軸、#660 で確定済み）の後方互換を壊さない設計が前提。フラグ名の最終決定は本実装 issue に委ねる。
- **未解決のまま残る論点**: 第2節（境界をまたぐ span の表示戦略3案）は本 issue のスコープ外で、issue #710 の spike 結果を待つ。境界またぎ item がある場合、time-range 軸の group band / gantt / zigzag 自体への追加の影響はない（境界またぎの影響は span/event_range 自体のクリップ表示にのみ及び、bar-stack や zigzag の parity 計算には波及しないことを本 spike で確認済み）。

### Status 更新

第3節「group band / gantt / zigzag / open-ended range の分割時の振る舞い」は、本 spike により **group band / gantt / zigzag / open-ended の4機能すべてで「時間範囲軸固有の追加設計は不要」と判明し、未検証状態を解消した**。残る未検証事項は第2節（境界をまたぐ span の表示戦略、issue #710 が担当）のみ。

## Spike 実施結果（issue #710、境界をまたぐ span の表示戦略3案の比較）

issue #709 の spike 土台（`split_ir_by_time_range` / `items_crossing_boundaries`）の上に、第2節が挙げた3つの表示戦略を `crates/tdsl-render/src/time_range_pagination_spike.rs` に `#[cfg(test)]` 限定・本番未配線で実装し、同一 IR（`s-crossing`: `[80, 220]`、4ページ・0..400分割、境界は 100/200/300）に対する出力を比較した。

### 実装した3案

- **戦略1: クリップ + 継続マーカー**（`clip_with_continuation_markers`）— `layout::primary_axis_segment` の既存クランプ（`start_frac.max(year_min)` / `end_frac.min(year_max)`）がクリップ自体は既に行っているため、新規実装が必要なのは「どちら側がクリップされたか」を示すマーカーのみ。ページごとに `continues_from_previous_page` / `continues_to_next_page` の2フラグを返す。31行。
- **戦略2: 開始ページのみ描画**（`start_page_only_visible_items`）— item の `start` が属するページ1枚にのみ表示し、他ページでは完全に省略する。3案中最小の実装（25行）だが、クランプ計算そのものが不要なぶん単純なだけで、機能的には情報を捨てている。
- **戦略3: 主ページに全体を縮小描画**（`primary_page_shrunk_extent`）— item の全期間を「開始点を含むページ」の幅に強制的に圧縮する（`shrunk_width_frac`、`1.0` にクランプ）。36行。

### 検証できたこと

- **戦略1**: `s-crossing`([80,220]) は 0..400/4ページ分割でページ0・1・2の3ページにまたがる。各ページのマーカーは幾何学的に整合した値になった — ページ0は `continues_to_next_page=true` のみ、ページ1は両方 `true`（前ページから継続かつ次ページへ継続）、ページ2は `continues_from_previous_page=true` のみ。item が全く触れないページ3にはマーカー自体が存在しない（`両方false`ではなく「不在」で表現— 「クリップされたが継続していない」と「そもそも存在しない」を区別する設計）。テスト `strategy1_clip_markers_are_present_on_every_page_the_item_intersects` で確認。
- **戦略2**: `s-crossing` はページ0（`start=80` が `[0,100)` に属する）にのみ出現し、ページ1・2には一切現れない。ジオメトリ上はページ1・2にも実在する item が、見た目上は消える。テスト `strategy2_item_disappears_from_every_page_but_its_start_page` で構造的に確認（ADR-0005 §2 が予告した「後半ページだけを見た読者には span の存在が分からない」を実測で再現）。
- **戦略3**: `s-crossing` の実際の長さ(140年)は主ページ(ページ0, 幅100年)の1.4倍のため、`shrunk_width_frac` は `1.0` にクランプされる(テスト `strategy3_long_span_saturates_the_shrink_clamp`)。一方、ページ幅に収まる短い item(`s-1`: 80年 / 100年ページ = 0.8)はクランプされずそのまま比例した幅になる(テスト `strategy3_short_span_does_not_saturate`)。これは「実際の長さが2倍・10倍・100倍でもクランプ後は区別がつかない」ことを意味し、王朝の存続期間のような大きく異なる長さの span 同士が視覚的に同一になり得るという ADR-0005 §2 の懸念を実測で確認した。

### `primary_axis_segment` の利用/拡張と差分行数

3案とも `primary_axis_segment` そのものは変更していない（本番レイアウトコードは無傷のまま）。差分は本 spike ファイル内の純粋関数のみで完結し、`git diff --stat` 実測で **251行**（テスト含む。うち戦略実装本体は3関数合計92行、残りはテスト）。

- 戦略1は `primary_axis_segment` の既存クランプをそのまま利用し、クリップ判定に追加の境界比較ロジック（`end <= seg_start || start >= seg_end` の除外 + 2フラグの算出）を足すだけで済んだ。本実装時に必要な追加コストはこのマーカー算出ロジックと、SVG側での継続マーカー描画（矢印等のグラフィック要素）のみ。
- 戦略2は `primary_axis_segment` を全く使わない（クランプ計算自体が不要）。実装コストは3案中最小だが、これは「クリッピングという難しい問題を回避した」のではなく「情報を落として問題自体をなくした」ことによる見かけ上の単純さである。
- 戦略3は `primary_axis_segment` の代わりに新規の圧縮計算（主ページの決定 + 幅比のクランプ）が必要で、既存クランプとは別の計算軸を持つ。

### 継続マーカーの決定的テスト可否（ADR-0004 D7 パターン）

戦略1のマーカーは「ページごとの2フラグ(bool)」という構造化データであり、ADR-0004 D7 が採用した「ページ数・構造アサーション中心、ゴールデン画像比較なし」パターンがそのまま踏襲できることを実測で確認した(`strategy1_clip_markers_are_present_on_every_page_the_item_intersects` は座標やSVG文字列ではなく `bool` の組み合わせのみをアサートしている)。クリップ座標自体も `primary_axis_segment` の既存クランプ式から決定的に導出されるため、追加の非決定性は生じない。

### 3案の比較表

| 案 | 実装コスト(spike実測) | 情報欠落リスク | 視認性 |
|---|---|---|---|
| 1. クリップ + 継続マーカー | 中(31行 + 本実装時はSVG側のマーカー描画が別途必要) | なし(マーカーが継続を明示) | 高(クリップ位置・継続方向が正確に伝わる) |
| 2. 開始ページのみ描画 | 低(25行) | 高(後続ページで item が完全に消える。implementation-strict.md §1 に反する) | 低(item の存在自体が一部ページで分からない) |
| 3. 主ページに縮小描画 | 中(36行) | 中(長さの相対関係が飽和して失われる) | 低〜中(短い item では機能するが、長い item ほど視認性が劣化する) |

### 推奨案

**戦略1(クリップ + 継続マーカー)を推奨する。** 戦略2は情報欠落が implementation-strict.md §1「Explicit error over silent fallback」の精神に明確に反するため却下。戦略3は実装コストが戦略1と同程度でありながら、長い span ほど情報欠落が悪化するという戦略2と同種の問題を抱える(飽和により長さの違いが視覚的に区別できなくなる)ため却下。戦略1は本 spike で実測した通り、既存の `primary_axis_segment` クランプを流用でき追加コストが最小であり、かつ ADR-0004 D7 の決定的テストパターンをそのまま踏襲できる。

### 未解決のまま残る論点

- 継続マーカーの具体的な描画(矢印の形状・色・アクセシビリティラベル)は本 spike のスコープ外。本実装 issue で改めて設計する。
- 戦略1は item がページ内で intersect する場合のみマーカーを返す設計にしたが、「visually clipped but marker suppressed」のようなオプトアウト経路が必要かは本実装 issue で検討する。
- 縦書き(vertical)レイアウトでの継続マーカーの向きは未検証(本 spike は水平レイアウトの `primary_axis_segment` の座標系のみで検証した)。

### Status 更新

第2節「ページ境界をまたぐ span/event_range の扱い」は、本 spike により3戦略の実装比較と推奨案(戦略1: クリップ + 継続マーカー)が確定し、未検証状態を解消した。ADR-0005 の未検証事項は全節で解消済みとなった(第1節のページ分割軸の最終選定、第4節のCLIフラグ名、継続マーカーの具体的な描画仕様は、いずれも本実装 issue に委ねる実装詳細として残る)。

## D3. 時間範囲軸チャートページ分割の本実装 GO/NO-GO 判断（issue #662 の結論）

issue #709/#710/#711 の spike 結果を踏まえ、**時間範囲軸のチャートページ分割を本実装する（GO）と判断する。**

- ADR 0004（#609）が Effort L / Risk HIGH と見積もっていた根拠（境界をまたぐ span のクリッピング・継続表示・レイアウトエンジンの構造的変更）は、spike で以下の通り縮小した:
  - group band / gantt / zigzag / open-ended の4機能はいずれも追加の分岐処理が不要（#711）。
  - 境界をまたぐ span の表示戦略は「クリップ + 継続マーカー」に確定し、既存の `primary_axis_segment` クランプを流用できる（#710、spike実装92行）。
  - テスト戦略は ADR-0004 D7 の構造アサーションパターンをそのまま踏襲可能と実証済み（ゴールデン画像比較不要）。
- 残るコストは (a) CLI 警告経路の配線（lane グループ軸の `group_bands_split_across_pages` パターン踏襲）、(b) 継続マーカーの SVG 描画、(c) 範囲外 item のフィルタ最適化（任意）に限定される。
- 本実装 issue で確定させる実装詳細（本 ADR では決定しない）: CLI フラグ名（`--chart-pagination-range <N>` 案 / `--chart-pagination lane:<N>|range:<N>` 案）、継続マーカーの具体的描画仕様（矢印形状・色・アクセシビリティラベル）、縦書きレイアウトでの継続マーカーの向き、`--pdf-pagination`（テーブルページ分割）との組み合わせ時の挙動。
- issue #662 はこの判断をもって完了とする。本実装は新規 issue として起票する。

## D4. 時間範囲軸チャートページ分割の本実装で確定した仕様（issue #729/#733/#734/#736 の結論）

D3 が本実装 issue に委ねた実装詳細は、以下の通り確定・実装済み（#729 の分割issue #733/#734/#735/#736 経由）。

- **CLI フラグ名**（#733）: `--chart-pagination-range <N>` に確定。既存の lane グループ軸 `--chart-pagination <N>` とは独立した別フラグとし、両者は併用不可（明示エラー）。`--chart-pagination lane:<N>|range:<N>` 案は不採用（バリデーション形状を lane 軸と完全に揃えられる独立フラグの方が単純で後方互換リスクが低いと判断）。
- **継続マーカーの描画仕様**（#734）: 境界をまたぐ `span`/`event_range` のクリップされた辺に、小さな三角形の `<polygon>` を直接描画する（`<defs><marker>` + `marker-start`/`marker-end` は不採用 — `<marker>` の内容は多くの実装でアクセシビリティツリーの外側に置かれるため、`role="img"` `aria-label`/`<title>` を持たせる要件を満たすには直接描画する図形の方が適切と判断）。色は固定のニュートラルカラー `#555` をインライン指定（standalone SVG がホストCSSに依存せず正しく見えるようにするため）。CSS フックは `tdsl-item-continues-from-previous-page` / `tdsl-item-continues-to-next-page`（`<g>` 要素、既存の `tdsl-item-open-ended` パターン踏襲）と `tdsl-continuation-marker-from-previous-page` / `tdsl-continuation-marker-to-next-page`（`<polygon>` 要素自体）。マーカー描画は新規 `RenderOptions::show_boundary_clip_markers`（デフォルト `false`）による opt-in で、`--chart-pagination-range` の内部レンダリングのみが有効化する（ページ分割と無関係な狭い `range` 指定の通常レンダリングは、既存の「サイレントにクランプする」挙動を変えない）。
- **縦書きレイアウトでの継続マーカーの向き**（#734）: 横書きは左右方向、縦書き（`Orientation::Vertical`）は上下方向に矢印が向くよう、両orientationに対応。
- **`--pdf-pagination` との組み合わせ挙動**（#736）: lane グループ軸の PDF 統合（issue #661）と同じページ構成規則を踏襲。単一 PDF 内で「チャートページ群（時間範囲順）→ テーブルページ群」の順に並び、テーブルページの `i / N` フッタはテーブルページ数のみを数える（先行するチャートページ数は含めない）。`--chart-pagination-range` を指定しない既存の `--format pdf` 出力は本機能追加後も完全に不変。
- **PDF 経路の境界またぎ警告**（#736、当初は別issue #735 で計画したが実装が重複するため #736 に吸収してクローズ）: SVG 経路と同じ `items_crossing_boundaries` の結果を元に、共通ヘルパー関数で文言を揃えた stderr 警告を PDF 経路にも配線した。
- **範囲外 item のフィルタ最適化**: D3 で任意（optional）とされたパフォーマンス施策で、今回のスコープでは未着手。各ページの `TimelineIr` は全 item を保持したまま `LayoutModel::compute` を通すため、そのページに実際には表示されない item も毎回レイアウト計算される（issue #711 の spike で確認済みの想定コスト）。計測の上で必要になった場合に別issueで対応する。

## 未決定事項（本 ADR の範囲外）

- HTML/SVG インタラクティブレンダリング（`tdsl render --interactive`）への同様のページ分割ニーズの適用可否（本 ADR は PDF/SVG 静的出力のみを対象とする）。
