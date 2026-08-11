# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

### Added

- **`check` に安定した診断コード（E1xx / W2xx）と `--format json` / `--deny-warnings` を追加**（#748）: `docs/error-catalog.md` が定義していた診断コード体系が実際の出力に一切含まれておらず、CI で特定の警告だけを許容/禁止することができなかった。`LoweringError::code()` と `ValidationDiagnostic::code` を追加し、テキスト出力を `Warning [W204] line 2: ...` 形式に変更する（**破壊的変更**: 警告の出力文字列が変わる。メッセージ本体は従来どおりで、コードと行番号が前置される）。`--format json` は `code` / `severity` / `line` / `message` を含む機械可読な診断を出し、`lint --format json` と同じ形（`file` / `ok` / カウント + 配列）に寄せてある。`--deny-warnings` は警告があれば非ゼロ終了する（既定は従来どおり警告のみなら成功）。あわせて `check` が `source` を lowering へ渡すようにし、診断に行番号が載るようにした（以前は常に欠落していた）。カタログに節が無かった W208（アイテム内でのオフセット混在）・W209（range でのオフセット混在）を追記し、**実装が返すコードがすべてカタログに存在することを検証するドリフト防止テスト**を追加した
- **`check` / `lint` / `fmt` が複数ファイル・ディレクトリ入力に対応**（#750）: 引数を `Vec<PathBuf>` にし、ディレクトリを渡すと配下の `*.tdsl` を再帰的に処理する。これまでは単一ファイルのみで、複数 `.tdsl` を持つリポジトリの CI は `for f in **/*.tdsl; do tdsl fmt --check "$f" || exit 1; done` のようなシェルループを書く必要があった。1 件でも失敗すれば非ゼロ終了するが、**最初の失敗で打ち切らず全件処理してから要約を出す**（CI では「どのファイルが落ちたか」を一度に知りたいため）。対象が 2 件以上のときは `=== <path> ===` の見出しを付け、単一ファイル指定時の出力は従来どおり（後方互換）。処理順はパス名でソートし、ファイルシステムの列挙順に依存させない（診断の出力順が実行ごとに変わると CI のログ差分が読めなくなる）。**対象が 0 件ならエラー**（パスの打ち間違いが「問題なし」として通らないようにするため）。glob 展開はシェルに任せる方針で、新規依存（walkdir / glob）は追加していない
- **`tdsl lint` に `unused_lane` ルールを追加**（#756）: 宣言されているが `span` / `event` / `event_range` からも `map` / `template` / `apply` の `lane` プロパティからも参照されていない lane を WARN で報告する。アイテムが 1 つも乗らない lane は空帯として描画され続けるが、従来は誰も警告しなかった（`CLAUDE.md` の「未使用lane等の警告」という記述に対して実装が存在しない乖離もあった）。`map` / `template` / `apply` の参照も「参照済み」として数えるため、item を直接書かず import から生成するファイルでも偽陽性は出ない（既存 examples 18 件で確認）。`group` 内の lane も対象。`fixable: false`（「これから item を足すために先に宣言した」ケースを機械的に壊さないため）。あわせて `CLAUDE.md` のコンパイルパイプラインの記述を、実装場所（validate ではなく lint）に合わせて修正した
- **offline lowering で未解決のまま残る `import` / `map` / `apply` ブロックを明示警告する**（#751）: `tdsl check` と `tdsl build --offline` は lowering の Pass 1/2 のみを実行し、import 解決（Pass 3）と map 適用（Pass 4）を行わない。従来はその旨がどこにも出ず、import だけで構成されたファイルに対して `OK: 2 lanes, 0 items` と表示して exit 0 していたため、「アイテムが 0 件なのは書き方が悪いのか offline だからなのか」を利用者が区別できなかった（LSP は同じ状況を Information 診断で出しており、CLI が LSP より寛容という逆転が起きていた）。`lower_static_with_diagnostics` が未解決ブロック数を warning として積み、`check` の完了行にも `(N block(s) unresolved: offline lowering does not run import/map)` を付ける。`import` / `map` / `apply` を含まないファイルでは警告を出さない（通常ファイルでのノイズ化を避けるため）。あわせて `tdsl check --offline` フラグを追加した（現時点では唯一の動作だが、コマンドラインから offline であることが読めるようにするため。オンライン `check` 自体はスコープ外）。新エラーコード W211
- **`--chart-pagination-range <N>`: タイムライン本体（チャート部分）を時間範囲軸で複数ページに分割するオプションを追加**（ADR-0005 D3 / #733, #736, #734）: `tdsl render --chart-pagination-range <N>` で、`meta.range` を `N` 個の連続する非空の整数年区間に均等分割し、区間ごとに1ページを描画できるようになった。既存の `--chart-pagination <N>`（lane グループ軸、issue #660）とは独立したフラグで、両者は併用できない（明示エラー）。lane グループ軸と異なり、各ページの `TimelineIr` は全 lane/item を保持したまま `meta.range`（およびサブ年精度フィールド、ページ境界では意味を持たないためクリアされる）だけが書き換わる。区間境界をまたぐ `span`/`event_range` は既存の `primary_axis_segment` クランプでクリップされ、クリップされた辺に継続マーカー（三角形の `<polygon>`。`role="img"` の `aria-label`/`<title>` 付き。横書き/縦書き両対応。CSS フック `tdsl-item-continues-from-previous-page` / `tdsl-item-continues-to-next-page` / `tdsl-continuation-marker-from-previous-page` / `tdsl-continuation-marker-to-next-page`）を描画し、境界をまたぐ item がある場合は stderr にも警告を出力する（silent no-op にしない、implementation-strict.md §1）。マーカー描画は `tdsl_render::RenderOptions::show_boundary_clip_markers`（新規、デフォルト `false`）による opt-in で、`--chart-pagination-range` の内部レンダリングのみが有効化するため、ページ分割と無関係な狭い `range` 指定の通常レンダリングは従来どおりマーカーなしでクリップされる（後方互換）。group band / gantt / zigzag / open-ended range の4機能はこの軸では追加の分岐処理が不要（`lanes`/`items` がページごとにフィルタされないため）。`--output` が必須で、`--watch` との併用は明示エラー。`--format svg`（`<stem>.pageN.svg` の複数ファイル、#733）と `--format pdf`（単一 PDF 内の複数ページ、`--chart-pagination` の PDF 統合と同じページ構成規則、#736）の両方に対応。`tdsl_render::PdfOptions` に `chart_pagination_range: Option<usize>` を追加し、`render_pdf_with_warnings` の戻り値は `(Vec<u8>, Vec<String>)` から `(Vec<u8>, PdfWarnings)` に変更（`PdfWarnings` は `group_bands_split_across_pages` と `items_crossing_boundaries` の2フィールドを持つ。#708 で既に破壊的変更を含む Unreleased につき同一の SemVer メジャーへ合流）

### Changed

- **BREAKING（次回メジャーリリース予定）: `tdsl-render` の render API が `RenderError` を返すよう変更**（#708）: `--layout-style zigzag` が 2 lane を超える場合に通常レイアウトへフォールバックせず明示エラーにするため、`render_html` / `render_svg_only` のエラー型を `std::fmt::Error` から `RenderError` に変更した。`PaginationError::Render` の payload も `RenderError` に変更し、`PdfError::Render(RenderError)` を追加した。外部利用者は従来の書式エラーを `RenderError::Fmt`、未対応レイアウトを `RenderError::UnsupportedLayout` として処理すること。次のリリースタグは SemVer に従い 2.0.0 とする。
- **BREAKING（同上 2.0.0 へ合流）: `tdsl_render::render_pdf_with_warnings` の戻り値を `(Vec<u8>, Vec<String>)` から `(Vec<u8>, PdfWarnings)` に変更**（#736、詳細は Added の `--chart-pagination-range` PDF 統合を参照）: 外部利用者は `warnings.group_bands_split_across_pages`（従来の `Vec<String>` 相当）と `warnings.items_crossing_boundaries` をそれぞれ参照すること。

### Fixed

- **`osv-scan` CI が既知脆弱性で継続的に失敗していたのを修正**（#731）: `apps/webui` の `brace-expansion`（GHSA-mh99-v99m-4gvg / GHSA-rgw5-rvv9-x895、DoS）と `nanoid`（GHSA-2v37-7h3g-55p8、無限ループ）は `package.json` の `overrides` で修正版に固定した。`Cargo.lock` の `rustybuzz` 0.20.1（RUSTSEC-2026-0206）・`ttf-parser` 0.25.1（RUSTSEC-2026-0192）は CVE ではなく RustSec の unmaintained 判定（今後も修正版は出ない）で、`tdsl-render` の推移的依存（`usvg`/`resvg`/`fontdb` 経由）を直接置き換えるには major upgrade が必要なため本PRのスコープ外とし、`osv-scanner.toml` に理由と `ignoreUntil = 2026-11-08`（silent renewal を避けるための再確認期限）を明記した上で一時的に ignore 対象とした
- **WASM 出力の SVG/HTML に `data-line` 等の interactive 属性が一切埋め込まれていなかったのを修正**（#700）: `render_svg_from_source` / `render_svg_from_source_with_options` / `render_html_from_source` / `render_html_from_source_with_options` はいずれも `RenderOptions::interactive` を常に `false` のまま `tdsl-render` へ渡していたため、`data-id` / `data-label` / `data-type` / `data-source` / `data-line` が出力されず、`tdsl_wasm.d.ts` の docstring（「`data-line` が埋め込まれる」）と実際の出力が乖離していた。WASM バインディングはブラウザでのインタラクティブプレビュー専用（WebUI のカーソル↔プレビュー双方向ジャンプ等）であるため、`render_options_for_ir` で常に `interactive: true` を設定するよう修正した
- **SVG アイテムの `aria-label` を英語に固定 / lane 色カスタムプロパティのスコープを `:root` から `:where(.tdsl-root)` に変更**（#701）: `item_aria_label` が `"スパン"` / `"イベント"` / `"期間イベント"` / `"レーン"` を日本語ハードコードで出力しており、`obsidian-tdsl` 側が UI 文言を英語に統一済み（#82）であることと食い違っていたため `"Span"` / `"Event"` / `"Event range"` / `"Lane"` に修正した。また `<style>` 内の lane 色カスタムプロパティ（`--tdsl-lane-N`）の定義に `:root` セレクタを使っていたため、SVG を `<img>` ではなく `DOMParser` + `document.adoptNode` でインライン DOM として挿入するホスト（`obsidian-tdsl` 等）でホストページの `<html>` 全体を汚染していた問題を、`:where(.tdsl-root)`（生成される `<svg>` 自身のクラスに限定しつつ `:where()` で詳細度をゼロにしたセレクタ）にスコープを限定することで修正した。`timeline-dsl-lp` の semantic-token ブリッジ（DESIGN.md、`.tdsl-root { --tdsl-lane-N: var(--tdsl-lane-warm) }` 等）のようにホスト側が既に `.tdsl-root` で lane 色を上書きしている場合、詳細度をゼロにしたことでスタイルシートの読み込み順に関わらず確実にホスト側の定義が優先されるようになる（後方互換）

## [1.28.0] - 2026-07-26

### Added

- **`--chart-pagination`: タイムライン本体（チャート部分）を lane グループ単位で複数の SVG ページに分割するオプションを追加**（ADR-0005 / #660）: `tdsl render --format svg --chart-pagination <N>` で、1 ページあたり `N` レーンずつチャートを分割し、`<stem>.pageN.svg` として出力できるようになった。時間軸は全ページ共通のため、`Item::lane` が単一 lane を持つ構造上、span/event_range のページ境界クリッピングは発生しない（issue #651 Spike で構造検証済み）。`--show-legend` は各チャートページに個別描画され、`--show-table` を併用するとチャートページ群の後ろに IR 全体の item を一覧する専用テーブルページを 1 枚追加する。lane の `group` がページ境界をまたいで分断される場合は stderr に警告を出力する（silent no-op にしない、implementation-strict.md §1）。`--output` が必須で、`--watch` との併用は明示エラー。既存の `--pdf-pagination`（テーブル専用）とは独立したフラグで、意味変更や後方互換への影響はない

- **`--chart-pagination` を PDF 出力に統合**（ADR-0005 申し送り事項 / #661）: `tdsl render --format pdf --chart-pagination <N>` で、チャートを lane グループ単位で複数の PDF ページ（別ファイルではなく単一 PDF 内の複数ページ）に分割できるようになった。ページ構成は「チャートページ群（lane グループ順）→ テーブルページ群」の順で固定。`--show-table` のみ（`--pdf-pagination` なし）の場合は IR 全体を 1 枚の未分割テーブルページとして末尾に追加し、`--show-table --pdf-pagination` を併用した場合は既存の行分割ロジックでテーブルページ群を生成する。いずれの場合もテーブルページの `i / N` フッタはテーブルページ数のみを数え、先行するチャートページ数を含めない。`crates/tdsl-render` に `PdfOptions::chart_pagination: Option<usize>`（デフォルト `None`）と `render_pdf_with_warnings()`（group band 分断警告を返す新API、`render_pdf()` はラッパーのまま不変）を追加。ADR-0004 D3 の後方互換制約により、`--chart-pagination` を指定しない既存の `--format pdf` 出力（単体 / `--show-table` / `--pdf-pagination` のいずれも）は完全に不変（回帰テストで保証）。A4/A3/Letter × 縦横向きの決定的テストマトリクス（ADR-0004 D7 パターン）にチャート分割ケースを追加した

### Changed

- **BREAKING: VS Code 拡張の `engines.vscode` を `^1.82.0` から `^1.91.0` に引き上げ**（#685）: `vscode-languageclient` を 9.0.1 → 10.1.0（major）に更新した際（#676）、`engines.vscode` の宣言が実依存要件（VS Code 1.91+ を要求）に追随していなかった。放置すると VS Code 1.82〜1.90 のユーザーに LSP 機能が壊れた状態の拡張が配布されるため修正した (#688)
- **依存更新（cargo/npm のマイナー・パッチ）をグループ化 PR でまとめて反映**: clap / tokio / thiserror / serde_json / async-trait / fast-uri / quinn-proto 等の cargo 更新、および apps/webui・editors/vscode の npm マイナー・パッチ更新19件を含む (#652–#658, #667, #670, #672, #673)
- **依存更新管理を Dependabot から Renovate に移行**: `.github/dependabot.yml` を削除し `.github/renovate.json5` を追加。cargo / npm（apps/webui, editors/vscode）/ github-actions の3エコシステムを移植し、minor/patch はグループ化して1PRに、major は個別PRのまま既存方針を踏襲。`pdf-writer`/`svg2pdf` の lockstep 制約（PR #415）と `rust-toolchain.toml` の手動更新方針は Renovate 側の抑止ルールとして維持 (#674)
- **apps/webui の TypeScript major 更新を Renovate で一時抑止**: `typescript-eslint@8.65.0` の peerDependencies が `typescript: ">=4.8.4 <6.1.0"` であり、TypeScript 7.x では `npm install` が ERESOLVE で失敗するため（TS7 対応版は現状 alpha のみ）。孤児化した Dependabot PR をクローズし、typescript-eslint が TS7 を安定版でサポートするまで Renovate の major 更新を抑止するルールを追加した。`editors/vscode` は eslint 非依存のため対象外で TS 7.0.2 のまま (#669, #689)
- **CI: GitHub Actions をコミット SHA ピン化し Node を 24 に更新**: サプライチェーン耐性向上のため、タグ参照の Actions をコミット SHA 参照に置き換えた
- **CI: Homebrew tap 更新を `TAP_BUMP_TOKEN` による自動 PR 作成方式に変更**: 陳腐化していたリポジトリ内 `Formula/` ディレクトリを削除し、`keroway/homebrew-tap` へ bump PR を自動作成する方式に統一した
- **CI: gitleaks / remove-in-progress-on-close を reusable workflow 呼び出しに置換**
- **ワークスペース標準ツールとして `justfile` / `lefthook.yml` / `.editorconfig` を追加**（#666）

### Fixed

- **AGENTS.md §5 の秒精度/UTCオフセット関連記述を実装状況に合わせて修正**（#645）: 「Sub-year precision beyond minute (e.g. seconds / time zones)」が未実装であるかのように記載されていたが、秒精度（`DateTimeSecond`）と UTC オフセット（`DateTimeOffset` / `DateTimeSecondOffset`）は #612〜#616（ADR-0003）で実装済み。実際に未実装のサブ秒精度と IANA タイムゾーン名（DST自動解決）のみを Deferred として記載するよう修正した
- **README.md / README.ja.md の「Known uncovered paths」を実カバレッジに照らして更新**（#646）: Wikidata クライアントの 429/5xx リトライは wiremock テストでカバー済み、PDF レンダリング（`svg2pdf`/`usvg`、in-process 純Rust・外部バイナリ非依存）はパジネーションのバリデーションエラーケースを含めユニットテスト済み、CLI の `--offline` は `build`/`export-csv` について単体テスト + ブラックボックス統合テストでカバー済みであることを確認し、実態に合わせて記述を更新した。真に未カバーなのは (1) Wikidata クライアントの低レベル接続エラー（`e.is_connect()`）リトライ分岐、(2) `tdsl-render` のフォントフォールバック特殊ケース・大規模ページ数マトリクスの網羅性、(3) `merge` サブコマンド自体を実バイナリ経由（clap dispatch込み）で叩く `--offline` 統合テスト（`cmd_build()` への委譲により関数レベルでは間接カバーされているのみ）の3点
- **VS Code 拡張: `tsconfig.json` に `types: ["node"]` を明示指定**: TypeScript 7.0.2 で TS2591 エラーが発生していたのを修正 (#684)
- **WebUI: pako 3.x の破壊的変更に追随**: `pako` を 2.1.0 → 3.0.1 に更新した際の API 変更に追随した (#671)

### Security

- **シークレット ignore パターンを統一**（#683）: `.gitignore` を整備し、gitleaks ワークフローが `pull-requests: write` 権限不足で startup_failure により**一度も実行されていなかった**問題を修正した
- **brace-expansion の高深刻度脆弱性を解消**（Dependabot #10, #11, #13）(#659)
- **brace-expansion の追加脆弱性を解消**（`editors/vscode` の間接依存。`vscode-languageclient` 9.0.1 → 10.1.0 major 更新に付随） (#676)
- **quinn-proto を 0.11.14 → 0.11.16 に更新** (#677)

### Docs

- **`--chart-pagination` の使用例を README.md / README.ja.md に追加し、examples ギャラリー表（README・WebUI 双方）の欠落4〜5件を補完**: `feature_showcase.tdsl` / `china_dynasties_filtered.tdsl` / `iss_docking_second_precision.tdsl` / `global_conference_timezones.tdsl`（README.ja.md はさらに `internet_history.tdsl`）。WebUI ギャラリーには `examples/*.tdsl` の全数網羅を検証するドリフト防止テストを追加した (#690)
- **`docs/dsl-spec.en.md` に `--pdf-pagination`（v1.27.0）と `--chart-pagination`（v1.28.0）を反映**し、1リリース分の英日ドリフトを解消 (#690)
- **ADR-0005 の Status を実装完了状況に更新し、issue #661（PDF統合）の実装結果セクションを追記**（従来 #660 のみ記載されており追記漏れだった）(#690)
- **ADR-0005 を Accepted に更新**、**ADR-0007（IANAタイムゾーン名対応）を Accepted に確定**: D1「対応しない」を正式決定。IANA タイムゾーン名は引き続き未サポート、固定数値 UTC オフセットのみ対応 (#687)
- **参照切れの修正と README への `--pdf-pagination` 追記** (#678)
- **VS Code README 等のキーワード列挙を `keywords.json` への参照に置換**し、ドリフトを防止 (#680)
- **note/link/color/now を実演する example を追加しギャラリー表を更新** (#681)
- **リリース手順（`docs/release.md` / `CONTRIBUTING.md`）に内部依存 version ピン5箇所と Homebrew tap 更新フローを追記**: バンプ対象表に `crates/tdsl-core` / `crates/tdsl-render` の内部依存 version ピンが抜けており、揃え忘れると crates.io publish が失敗する状態だったのを是正 (#691)

## [1.27.0] - 2026-07-19

### Added

- **秒精度・UTCオフセット（ADR-0003）: parser/AST対応**（#612）: `crates/tdsl-parser` の時刻リテラル構文に秒（`HH:MM:SS`）と UTC オフセット（`Z` / `±HH:MM`、-14:00～+14:00、範囲外・書式不正はパースエラー）を追加。`TimeValue` に `DateTimeSecond` / `DateTimeOffset` / `DateTimeSecondOffset` の新 variantを追加し、既存 variant（`Year`〜`DateTime`）は無変更（非破壊）。IR未対応の段階ではこれらの新 variant を使うと lowering が明示エラーを返す（silent fallbackしない）

- **秒精度・UTCオフセット（ADR-0003）: IR/schema/lowering/Wikidata対応**（#613）: IR（`ir.rs`）に `*_second` / `*_offset_minutes` フィールドを追加し、JSON schema も更新。lowering は offset 付き値同士を UTC 正規化して比較し、offset の有無が混在した比較は `MixedOffsetComparison` エラーとして拒否する（ADR 0003 D2、AGENTS.md §4.1）。Wikidata インポートは precision 14（秒）を `DateTimeSecond` にマッピングし、常に offset なしで格納する

- **`unit second` の軸ティック/ラベル生成 + decompile/fmt/CSV の秒・UTCオフセット対応**（ADR-0003 / #614）: `crates/tdsl-render` に `unit second` の軸ティック・ラベル生成（1s→5s→15s→30s の密度制御、`hour`/`minute` と同様のパターン）を追加し、Span/Event/EventRange の座標計算にも秒精度を反映した。`tdsl decompile` と `export-csv` は秒・UTCオフセット（`Z` / `±HH:MM`）を round-trip 可能な形式で出力するように修正（従来は無声で破棄していた）。`tdsl fmt` は `TimeValue::Display` 経由で既に秒・offsetを保持できていたことを round-trip テストで確認。`TimelineUnit` に `Second` variant、DSL/VS Code/LSP のキーワード一覧に `second` を追加

- **`--pdf-pagination`: PDF出力でアイテムテーブルを複数ページに分割するオプションを追加**（ADR-0004 / #618, #619, #620, #621, #622）: `tdsl render --format pdf --show-table --pdf-pagination` で、タイムライン本体（1ページ目、従来どおり単一ページ）とアイテムテーブル（2ページ目以降、用紙サイズ・余白・縦横向きから計算した行数ごとに分割）に分けて出力できるようになった。各テーブルページの先頭に列見出しを再描画し、フッタに `i / N` 形式のページ番号を付与する（`N` はテーブルページ数のみを数えたもの）。PDF ドキュメントタイトルメタデータはページ分割の有無に関係なく単一。`--pdf-pagination` は opt-in（デフォルト無効）で、既存の `--format pdf` 単体・`--show-table` 単体の出力は完全に不変。`--show-table` なしで `--pdf-pagination` を指定するとエラー（silent no-op にしない、AGENTS.md §4.1）。`--show-legend` / `--layout-style group-bands`・`gantt`・`zigzag` / open-ended range（`now` 終了）はいずれもタイムライン本体（1ページ目）の描画にのみ関わり、本フラグの有効/無効でタイムラインページの描画内容が変わることはない（ADR-0004 D5）。タイムライン本体（チャート部分）のページ分割は本機能のスコープ外（ADR-0004 D1）。A4/A3/Letter × 縦横の実 PDF ページ数マトリクス、CJKテーブル行、テーマ切り替えと pagination の相互作用なしを確認する決定的テストマトリクスを追加し、テスト方針（ADR-0004 D7: ゴールデン画像比較を使わず構造検証中心）を `crates/tdsl-render/README.md` にドキュメント化した。`docs/cli-spec.md` / `docs/dsl-spec.md` の `--pdf-pagination` 記述を実行検証（デフォルトは単一ページ維持、`--pdf-pagination` + `--show-table` で多ページ化、`--show-table` なしの `--pdf-pagination` はエラー終了）し確認済み

- **PDF pagination と既存レイアウト機能（show-legend/group-bands/gantt/zigzag/open-ended）の整合検証を拡充**（ADR-0004 / #620）: A4/A3/Letter × 縦横向きのマトリクステスト、CJK長文ラベル、テーマ切り替えなど、`render_pdf()` の実分岐を通る決定的構造検証テストを追加（ゴールデン画像比較は使用しない方針、ADR-0004 D7）。`crates/tdsl-render/README.md` にテスト戦略をドキュメント化

- **LSP hover/range の秒・offset対応 + WASMバンドルサイズ影響計測**（ADR-0003 / #615）: `crates/tdsl-lsp` の hover が時刻リテラル（span/event/event_range/timeline range）の精度（year〜second）と offset を表示し、リテラル全体（offsetサフィックス含む）をカバーする range を返すようになった。WASMバンドルサイズへの影響を実測（+3.43%、軽微）し結果を ADR 0003 に追記

- **秒・UTCオフセットのドキュメント整備 + minute-level既存ファイルの互換・移行ルール確定**（ADR-0003 / #616）: `docs/dsl-spec.md` に秒・UTCオフセットの正式仕様と比較セマンティクス（UTC正規化・MixedOffsetComparison）を追記。`examples/iss_docking_second_precision.tdsl`（秒精度 + UTC `Z`）と `examples/global_conference_timezones.tdsl`（複数タイムゾーンオフセット）を新規追加し、両方とも `crates/tdsl-core/src/tests/golden.rs` のスナップショットテストで回帰保護。既存 minute-level（秒・offsetなし）`.tdsl` サンプルが引き続きパース・buildできることを保証する回帰テストを追加。新規 `docs/migration-second-precision.md` で Wikidata インポート（常にoffsetなし）と静的offset付きデータの混在時の対処方法を明記。`docs/error-catalog.md` に E006～E008（秒/月日/オフセットのパースエラー）と E113（`MixedOffsetComparison`）を追記

### Changed

- **IR JSON schema を後方互換な追加形式で拡張**: 秒精度・UTCオフセット用の `*_second` / `*_offset_minutes` は省略可能フィールドとして追加した。既存の minute-level DSL と通常の serde consumer は無変更で利用できるが、未知フィールドを拒否する厳格な外部 consumer は schema 更新が必要（移行上の注意は `docs/migration-second-precision.md` を参照）

### Fixed

- **CLI `--show-table` が PDF/PNG/SVG 出力で常に `false` に上書きされていた問題を修正**: `crates/tdsl-cli/src/commands/render.rs` の `effective_show_table` 判定は `--show-table` を HTML 形式のみ有効としてそれ以外では警告を出して `false` に強制しており、tdsl-render が #536/#541 で SVG/PNG/PDF の `show_table` に対応済みだったことと不整合だった（CLI 経由では `--format pdf --show-table` を指定しても表が描画されない状態）。全フォーマットで `--show-table` をそのまま使うように修正した (#619)

## [1.26.0] - 2026-07-15

### Added

- **`export-csv` → `import-csv` の `source`/`origin` 往復保持に対応**: 従来 `import-csv` は先頭8列のみを読み `source`/`origin` を破棄していたが、任意列として受理し DSL の `source_ref` / `ident` 文法（`crates/tdsl-parser` に新規公開した `parse_source_ref_literal` / `parse_ident_literal`）で検証したうえで往復保持するようにした。`origin=wikidata` は `source=wd:Q<id>` を必須とし（不整合は行番号付きエラーで拒否）、`wd:Q…` source を持つ静的アイテムの `origin` は書き換えない。旧8列形式の CSV は引き続き受理される（後方互換） (#608)
- **WebUI: GitHub Pages デモの SEO/OGP メタデータと `<html lang>`/`document.title` のロケール連動を追加**: `apps/webui/index.html` に `<meta name="description">` / Open Graph / Twitter Card を追加し、SNS 共有時のプレビューが空だった問題を解消した（`og:image`/`og:url`/`twitter:image` は OGP 仕様上絶対 URL で固定記述）。OG 画像 （`apps/webui/public/og-image.png`, 1200x630）はこれまで未使用だった `src/assets/hero.png` を PWA の brand 背景色（`#13131f`）に合成して生成。新規フック `useDocumentMeta`（`apps/webui/src/hooks/useDocumentMeta.ts`）が設定ロケール（`ja`/`en`）に応じて `document.documentElement.lang` と `document.title`（`i18n.ts` 新規キー `documentTitle`）を実行時に同期する。静的 HTML の既定値も `lang="en"` 固定から `lang="ja"`（`DEFAULT_LOCALE`）に修正

### Fixed

- **キーワード管理ドキュメントの single source of truth 記載を `keywords.ts` → `keywords.json` に修正**: 実装（`keywords.ts` 自身のコメント、`gen-grammar-keywords.mjs`、`crates/tdsl-lsp/src/keywords.rs`）は既に `keywords.json` を真実源としていたが、`CLAUDE.md` / `apps/webui/README.md` の記述が乖離していた (#623)

## [1.25.0] - 2026-07-13

### Fixed

- **`tdsl build` がパースエラーメッセージを握りつぶし空の `Err(String)` を返していた問題を修正**: `load_ir`（`crates/tdsl-cli/src/commands/build.rs`）は miette のキャレット付き診断を stderr へ出力するのみで、呼び出し元へは空文字列しか返していなかった。`check::print_parse_error` から診断文字列を返す `render_parse_error` を分離し、`load_ir` はそれを `Err` ペイロードとして伝播する。無効な `.tdsl` を `cmd_build` に通し、返却エラーが空でなく実際の診断を含むことを検証する回帰テストを追加 (#606)
- **WebUI: Web Worker が `onerror` 後に回復不能となり、リロードするまで compile/render が恒久的に壊れる問題を修正**: `client.ts` の `getWorkerClient()` は致命的エラー後も同一の `sharedClient` をキャッシュし続け、`readyState === 'error'` になった Worker へのリクエストが永久に失敗していた。`onFatalError` コールバックで共有クライアントを無効化し、次回 `getWorkerClient()` 呼び出し時に新しい Worker を生成・`ready()` し直すようにした。`useWasm` の初期化は一時的な失敗に対して 1 回リトライし、`useCompiler` / `useExport` は Worker クライアントをリクエスト単位で再解決することで、無効化されたクライアントに固執しないようにした (#607)

## [1.24.0] - 2026-07-05

### Added

- **WebUI の設定パネルに「イベントラベル常時表示」トグルを追加**: レンダラコア（#403）/ WASM facade（#417）で既に実装済みだった `show_event_labels` が `apps/webui` の UI からは制御できず、CLI/WASM 直接呼び出し経由でしか利用できなかったギャップを解消。設定モーダルに ON/OFF トグルを追加し、SVG/HTML プレビューとエクスポート（SVG/PNG/HTML/PDF）に反映する。一覧確認や印刷資料作成用途でホバー不要で全イベントのラベルを常時可視化できる (#586)
- **`--layout-style zigzag` でレーン内アイテムを上下交互配置**: `--orientation` と直交する新しいレイアウトスタイル。レーン内アイテムを開始時刻順に上下交互に配置し、単一/少数レーンの歴史タイムラインの可読性を向上。レーン数が 2 以下のときのみ有効で、それを超える場合は警告を出して通常 `timeline` レイアウトにフォールバックする（黙殺フォールバックはしない、AGENTS.md §4.1）。#549（バー重なり回避サブ行スタッキング）とは排他 (#578)
- **`--layout-style gantt` で月グリッド強調＋期間ラベルを追加**: `--orientation` と直交し `group-bands` と相互排他のプロジェクト管理向けスタイル。`--grid` 未指定時は月単位グリッド相当を自動で強制し、`span` / `event_range` のバーに開始〜終了の期間ラベルを常時表示する。近接ラベルは #537 のスタッキングアルゴリズムを流用し、#549 のサブ行 Y 位置を尊重して重なりを回避。WASM facade にも `layout_style` フィールドを追加し、従来 WASM 未露出だった `group-bands` も同時に利用可能にした (#577)
- **`span` / `event_range` / `event` アイテムに `note` / `link` / `color` オプションを追加**: `note "..."` はツールチップ/詳細に表示する自由記述、`link "..."` は参照 URL（lowering で `http(s)` 以外のスキームを `LoweringError` として拒否＝黙殺フォールバックなし、AGENTS.md §4.1）、`color "..."` はアイテム個別色。レンダー色の優先順位は `item.color` > `color_map(tag)` > lane パレット。`tdsl fmt` / `decompile` はラウンドトリップ安全。error-catalog に E111/E112 を追記 (#574, closes #566/#567/#568)
- **WebUI: File System Access API による開く/上書き保存に対応**: `showOpenFilePicker()` / `showSaveFilePicker()` をラップし、開いた `.tdsl` の書き込み可能ハンドルを保持。以降の保存（ツールバー/エクスポートメニュー/Ctrl+S）は同一ファイルを上書きする。API 非対応ブラウザは従来の `<input type="file">` + ダウンロードにフォールバックしつつ、明示的なトーストで通知する（黙殺フォールバックなし、AGENTS.md §4.1）(#572, closes #569)
- **WebUI: PWA オフライン対応を追加**: `manifest.webmanifest` と Service Worker を追加し、アプリシェル（JS/CSS/WASM）を事前キャッシュ。初回ロード後は静的 DSL 編集・プレビューをオフライン起動可能にした。新バージョン検知時は画面上部の更新通知から再読み込みできる (#571)

### Changed

- **WebUI の WASM 参照方式を npm 依存に切替え**: `apps/webui` は `apps/webui/src/wasm/` 配下のコミット済み成果物参照を廃止し、公式 npm パッケージ `@keroway/tdsl-wasm` への通常の `dependencies` に切替えた。`crates/tdsl-wasm` 変更時の手動 build+commit 運用と、それに伴う CI ドリフト検知（#579）を廃止した（ADR 0001 D6, #580）

### Fixed

- **`now` キーワードが WASM ビルドをクラッシュさせる問題を修正**: `span` / `event_range` の `end` に `now`（#561）を含む `.tdsl` を `tdsl-wasm`（`wasm32-unknown-unknown`）でパースすると、未実装の `std::time::SystemTime::now()` が捕捉不能な WASM トラップを起こしていた。ウォールクロック取得を target 別に分離し、wasm32 では `js-sys::Date` を使用（ネイティブ CLI の挙動は不変）(#583)
- **同一レーン内で重なる `span` / `event_range` のバー本体をサブ行スタッキングで回避**: 在位期間・戦争・製品ライフサイクルなど、同一レーンに重複期間を持つアイテムが完全に重なって描画されて読めなくなる問題を修正。区間スケジューリング（greedy interval coloring）でサブ行を自動割り当てし、サブ行数に応じて lane の実効高さ/幅を自動拡張。重なりがないレーンは従来通りの座標で回帰なし。`--layout-style group-bands` と互換 (#549)

## [1.23.0] - 2026-07-03

### Added

- **`unit hour` / `unit minute` の日中軸目盛りに対応**: 時・分単位のタイムラインで `hour_ticks()` / `minute_ticks()` による密度間引き（1h/3h/6h/12h、1min/5min/15min/30min）と `HH:MM` / `MM-DD HH:MM` ラベルを追加。`examples/apollo_11_hourly.tdsl` を追加 (#562)
- **`span` / `event_range` の `end` に `now` キーワードを追加**: 令和のような「継続中」の期間を、架空の終了年をでっち上げずに表現できるようにした。IR に `end_open` フラグを追加（既存 JSON 消費者は非破壊）。`tdsl fmt` / `tdsl decompile` は `now` をラウンドトリップする (#561)
- **`color_map` のキーに非 ASCII タグ向け文字列リテラルを許可**: 既存の bare-ident 形式との後方互換を保ちつつ `color_map_key = string_literal | ident` に拡張。`tdsl fmt` / `decompile` はラウンドトリップ安全 (#560)
- **`--show-legend` で静的凡例パネルを追加** (#544)
- **`--layout-style group-bands` で era/group の背景帯を追加**: `--orientation` と直交する新しい CLI フラグ。全 HTML/SVG テーマに対応 (#546)
- **常時表示 Event ラベルの重なりをスタッキングで回避** (#542)
- **`show_table` が SVG/PNG/PDF 出力でもレンダリングされるように対応** (#541)
- **バーからはみ出るラベルを縮小/省略/再配置** (#540)
- **ラベル幅推定を追加** (#539)
- **WebUI: 型付き i18n 基盤を追加** (#531)
- **`color_map` の伝播、レンダーオプション、キーボードアクセシビリティ（plans 005-008）** (#530)
- **日付精度モデルを拡張** (#529)
- **`export-csv` コマンドを追加**: IR を CSV として書き出す。`import-csv` と対称な列構成で往復可能 (#527)

### Fixed

- **`timeline.range` 外のアイテムに警告を追加**: 従来は無診断で描画側がドロップ/クリップしていたが、`validate_with_spans` 経由で `tdsl check`/`tdsl build`/LSP に警告として表示するようにした（W204/W205/W206/W207 を error-catalog.md に追記）（AGENTS.md §4.1「No silent fallback」）(#559)
- **`timeline.unit` / `lane.kind` の silent fallback を解消**: 未知の `unit` はローダエラー（`LoweringError::UnknownTimelineUnit`）に、未知の `kind` は検証警告にした（AGENTS.md §4.1「No silent fallback」）(#558)
- **デフォルトフォントの CDN 依存を削除** (#557)
- **分単位精度の検証修正とドキュメント同期、VS Code publish のロックファイルインストール化** (#533)

### Tests

- **`--layout-style group-bands` の group-bands カバレッジを追加しドキュメントを同期** (#543)

### Chore

- **`indicatif` を 0.18.4 → 0.18.5 に更新** (#532)
- **pi-lens の自動フォーマットを無効化するルート `biome.json` を追加** (#528)
- **キーワードを JSON 単一ソース化し、decompile の `unwrap` を文書化** (#526)

## [1.22.0] - 2026-06-26

### Changed

- **WebUI: WASM を遅延ロードし vendor チャンクを分割**: `wasmLoader.ts` の WASM glue を静的 import から動的 `import()` に変更し、独立した遅延チャンクとして分離。`vite.config.ts` の `manualChunks` で `react-vendor` / `codemirror-vendor` を分割した。メイン entry JS が 740.41 kB → 137.66 kB（gzip 236.67 → 43.87 kB）に縮小し、500KB 超チャンク警告を解消 (#515)

### Fixed

- **LSP: `documentChanges` capability の silent fallback を解消**: `backend.rs` の `.unwrap_or(false)` を `DocumentChangesSupport` enum（`Supported` / `ExplicitlyUnsupported` / `Unspecified`）と解決関数に置き換え、非対応時は INFO ログで観測可能にした（AGENTS.md §4.1「No silent fallback」）(#517)

### Security

- **WebUI: 未使用の `vite-plugin-top-level-await` を削除**: Vite 8 のネイティブ top-level await により不要だった依存を削除し、脆弱な推移的依存 `uuid`（moderate, GHSA-w5hq-g745-h8pq）を除去。`npm audit` が 0 vulnerabilities になった (#513)

### Tests

- **CLI 統合テストスイートを拡張**: `tests/cli_integration_test.rs` に build（単一 / merge / `--output`）/ check（valid / 構文不正 / 未定義 lane）/ lint / import-csv（stdout / `--output` / `--append`）のカバレッジを追加（+10、計 17）。パスは `CARGO_MANIFEST_DIR` 起点で CWD 非依存、offline 前提 (#516)

## [1.21.0] - 2026-06-24

### Added

- **`JsRenderOptions` に `lane_height` を追加**: WASM/JS から縦方向の lane 密度を指定できるようにした。未指定・0・負値・不正文字列はレンダラ既定値にフォールバックする (#507, #508)
- **`map` / `apply` の必須フィールド未解決を lowering 警告として報告**: マッピング対象の Wikidata エンティティが `lane` / `label` / `start` / `end` / `time` を解決できずアイテムを 1 件も生成しなかった場合、従来は無診断でスキップしていたが、非致命的な警告として収集し `tdsl build` / `tdsl check` が `Warning:` で表示するようにした（AGENTS.md §4.1「No silent fallback」）。新 API `lower_static_with_diagnostics` / `lower_with_wikidata_and_diagnostics` を追加（既存 `lower_*` は後方互換のため warnings を破棄）。`expand` 使用時は警告に `(プロパティ#インデックス)` を付与する (#498)

### Changed

- **WebUI ギャラリーのテンプレートを刷新**: トップページのサンプルテンプレートを更新した (#506)

### Fixed

- **Wikidata クライアントの User-Agent を実バージョンに同期**: ハードコードされていた `tdsl/0.1.0` を `env!("CARGO_PKG_VERSION")` でコンパイル時に埋め込むようにした。リリースごとに自動で正しいバージョンが反映される (#496)

### Security

- **WebUI の npm 推移的依存の脆弱性を非破壊修正**: `@babel/core` (low, GHSA-4x5r-pxfx-6jf8) と `brace-expansion` (moderate, GHSA-jxxr-4gwj-5jf2) を `npm audit fix`（非 --force）で解消した。いずれも devDependency の推移的依存でアプリ実行時バンドルには影響しない。残る `uuid` 系 (moderate) は破壊的変更が必要なため #513 で追跡 (#512)

### Docs

- **AGENTS.md の実装状況を現状に同期**: §5「Unsupported / Deferred Features」が実装済みの `query` / `template` / `apply` / qualifier mapping（P39+P580/P582）を「未実装」と誤記していたのを修正し、実際に未実装の項目（`map` ブロック内 `source:`、月日超の精度、紀元前の月日精度）のみを残した。§4.3 の imported item の `origin` 値を実装に合わせ `"wikidata"` に修正。CLAUDE.md のリトライ回数記述を実装（`DEFAULT_MAX_RETRIES = 5`）に合わせて「最大3回」→「最大5回」に修正 (#497)
- **error-catalog に lowering 警告 W210 を追記**: `map` / `apply` の必須フィールド未解決による無生成警告を文書化した (#498)

### Chore

- **rustfmt の `style_edition = "2024"` を明示**: `rustfmt.toml` を追加し、workspace の Rust edition とエディタ/エージェント側 rustfmt の import 並びを揃えてフォーマット差分を防止した (#509, #510)

## [1.20.0] - 2026-06-22

### Added

- **SVG 出力の lane 色を `--tdsl-lane-*` CSS variables 化**: `render_svg` の `<style>` ブロックに `:root { --tdsl-lane-N: #hex; }` を追加し、lane アイテムの fill を `var(--tdsl-lane-N, #hex)` 形式に変更した。LP サイトが `global.css` で同名トークンを上書きするだけで dark mode / high-contrast テーマに追従できる。デフォルト hex を fallback として保持するため standalone SVG でも描画は保証される。usvg は `var()` を解釈しないため、`RenderOptions.use_css_vars` フラグで PNG/PDF 出力時は plain hex を格納する (#485, #486)
- **DSL コメントを AST に保持しフォーマッタで再現**: パーサにコメント収集パス（`comments::scan_comments`）を追加し、`File.comments: Vec<Spanned<Comment>>` に行（`//`）・ブロック（`/* */`）コメントを byte span 付きで保持するようにした。文字列リテラル内の `//` / `/* */` はコメントとして誤認識しない (#472)
- **`tdsl fmt` がコメントを保持**: フォーマッタ（`format_file` / `format_source`）がトップレベルのコメント（文の前後・同一行末尾）を位置を保ったまま再 emit するようになった。ブロック内部のコメントは内容を失わずにブロック境界に移動される。整形は冪等（idempotent）で、lowering はコメントを無視するため IR は不変 (#473, #362)

### Docs

- **コメント保持の仕様を明記**: `docs/dsl-spec.md` の「コメントの扱い」を `tdsl fmt` での保持振る舞い・lowering 不変・`tdsl decompile` の非対応に更新し、`fmt`（`main.rs` / `commands/fmt.rs`）と `lint::fix_source` の既知制約 doc コメントを修正した (#474)
- **decompile のコメント非対応を明記**: `tdsl decompile` は JSON IR を起点とし IR にコメント情報が存在しないため、元ソースのコメント（`//`・`/* */`）を復元できないことを `docs/dsl-spec.md` / `docs/dsl-spec.en.md` / `docs/cli-spec.md` および `decompile` の doc コメントに明記した。IR を単一の真実とする設計上の恒久的制約である (#474)

## [1.19.0] - 2026-06-17

### Added

- **VS Code 拡張に LSP クライアントを統合**: `tdsl lsp` を stdio で spawn して VS Code の LanguageClient と接続する本体実装を追加。診断・補完・hover・定義ジャンプ・コードアクション・ドキュメントシンボル・参照検索・リネーム・フォーマットが VS Code 上で動作する。PATH 優先 + `timelineDsl.serverPath` 設定でバイナリの上書き解決が可能で、解決失敗時はインストール手順を案内するエラー通知を表示する (#470, #388)

### Docs

- **VS Code 拡張 README の LSP 機能説明を実装に合わせて正確化**: 実装済みの `tdsl lsp` コマンドや設定項目（`timelineDsl.serverPath`）の説明を追記し、LanguageClient が提供する機能一覧を正確に反映した (#483)

## [1.18.0] - 2026-06-16

### Added

- **WASM facade に `JsRenderOptions` を追加し orientation / grid / theme などをパラメータ化**: `render_svg_from_source_with_options` / `render_html_from_source_with_options` を追加。`JsRenderOptions` クラス（TypeScript 型定義付き）を通じて `orientation`（horizontal / vertical）・`grid`（none / decade / year / month）・`theme`（default / dark / print / pastel）・`show_table`・`show_event_labels` を JS から制御できるようになった。既存の `render_svg_from_source` / `render_html_from_source` は変更なし（後方互換）。`wasmLoader.ts` に `renderSvgWithOptions` / `renderHtmlWithOptions` および `RenderOptions` TypeScript interface を追加 (#417)
- **WebUI の Settings パネルに orientation / grid / theme の選択 UI を追加**: 設定パネルにトグルボタン形式のレイアウト方向（水平 / 垂直）・グリッド線（none / decade / year / month）・SVG テーマ（default / dark / print / pastel）の選択 UI を追加し、SVG プレビューにリアルタイムで反映するようにした。#417 で追加した WASM `JsRenderOptions` と連携し、WebUI からレンダリングパラメータを制御できるようになった (#420)
- **lint を WASM 経由で WebUI に提供**: `tdsl-wasm` に `lint_source(source) -> JSON`（issue 一覧）と `lint_fix_source(source) -> String`（修正後ソース）を追加。WebUI の診断パネルに lint 結果（`[lint:<code>]` プレフィックス + fixable 表示）を統合し、Toolbar に "Lint Fix" ボタンを追加した。Format ボタンと同様の UX で、自動修正の適用前に確認ダイアログでコメント / フォーマットが書き換わる旨を通知する。CI の WASM smoke / verify ステップに新 export を追記 (#429)
- **WebUI に Vitest ユニットテストを導入し CI に組み込む**: WebUI に Vitest テストスイートを導入し、#430 のコンポーネント分割で生まれたカスタムフック・ヘルパー関数のユニットテストを追加した。CI の Build WebUI ジョブに `npm run test -- --run` ステップを組み込み、PR / push 時にフロントエンドのテストが自動実行されるようにした (#431)
- **WebUI モーダルに focus trap を実装し a11y を改善**: `useFocusTrap` フックを新規追加し、設定 / ギャラリー / 履歴モーダル表示中の Tab / Shift+Tab フォーカスをモーダル内で循環させるようにした。モーダルを閉じた際は呼び出し元の要素へフォーカスが復帰する。Escape キーでも閉じられるよう統一し、ギャラリー / 履歴モーダルにも Escape クローズを追加。各モーダルに `role="dialog"` / `aria-modal="true"` / `aria-labelledby` を付与し、閉じるボタンに `aria-label` を追加 (#435)
- **CI に `tdsl.tmLanguage.json` の生成ドリフト検知ステップを追加**: `Build WebUI` ジョブに `gen-grammar-keywords.mjs` を再実行して `git diff --exit-code` でドリフトを検出するステップを追加した。`apps/webui/src/lang-tdsl/keywords.ts` を変更したのに `editors/vscode/syntaxes/tdsl.tmLanguage.json` の再生成・コミットを忘れた PR を CI が自動で fail させる（PR #448 で発生した手動再生成ケースの再発防止）(#452)
- **Criterion ベンチマークを main push 時に CI で実行し結果をアーティファクト保存**: `bench` ジョブを追加し、`main` ブランチへの push 時のみ `cargo bench --workspace` を実際に実行するようにした。従来の `bench-compile`（`--no-run`）は PR 時のコンパイル確認として継続。Criterion HTML レポートと stdout ログを `criterion-reports-<sha>` アーティファクトとして 90 日間保存し、性能トレンドをダウンロード確認できるようになった。ジョブは非ブロッキング・並列実行 (#434)
- **VS Code 拡張に TypeScript ビルド基盤と Language Client / Server の足場を追加**: `editors/vscode/` に TypeScript ビルド設定（`tsconfig.json` / `esbuild` バンドラ）と Extension Host 側の Language Client 足場コードを追加した。LSP サーバ（`tdsl-lsp`）を VS Code から起動・通信する土台となる実装で、Language Features（hover / completion / diagnostics）を VS Code 上で提供するための準備段階 (#469)
- **CI に gitleaks による secret scan を追加**: GitHub Actions CI に gitleaks を使ったシークレットスキャンジョブを追加した。PR / push 時に `.gitleaks.toml` のルールに従いソースコード中の認証情報・API キー等の流出を自動検知し、false positive は `# gitleaks:allow` コメントで除外できる (#467)
- **`.mailmap` とコミット設定でメール漏洩を予防**: `git log` / `git shortlog` で実メールアドレスが露出しないよう `.mailmap` を追加し、プライバシーアドレスを公開用ダミーアドレスにマッピングした (#464)

### Changed

- **App.tsx（約1,800行）をコンポーネント・フック・ヘルパーに分割**: WebUI のメインコンポーネントを責務別に分割し、App.tsx を1,858行から291行まで削減した。Editor / Preview / DiagnosticsPanel / Toolbar / Modals などのコンポーネント群、`useCompile` / `useExport` / `useFocusTrap` などのカスタムフック、および共通ヘルパー関数を独立したファイルに分離し、テスタビリティと保守性を改善した (#430)
- **`lower.rs`（約1,300行）をパス別モジュールに分割**: lowering の 4 パス（Pass 1: timeline/lane 収集・Pass 2: 静的アイテム変換・Pass 3: import 解決・Pass 4: map 適用）を `crates/tdsl-core/src/lower/` 配下の独立モジュールに分離した。外部 API（`lower_static` / `lower_with_wikidata` 等）は変更なく、各パスの責務が明確化してテストを書きやすくなった (#433)
- **`layout.rs` の `compute_item_horizontal` / `compute_item_vertical` の重複実装を orientation 抽象化で統合**: `tdsl-render` の `layout.rs` にあった水平・垂直レイアウト向けの2つの独立実装を、orientation を引数に取る単一の `compute_item` に統合した。コード量を削減し、将来レイアウトモードを追加する際の変更箇所を一本化した (#432)

### Fixed

- **WebUI の JSON IR エクスポートで import / map が黙って欠落する問題を修正**: WASM 環境では Wikidata fetch が実行されないため、import / map ブロックを含むソースの JSON IR エクスポートはインポート由来のアイテムを含まない部分的な IR になるが、その旨の通知なく保存されていた。未解決 import / map がある場合は確認ダイアログで「インポート由来のアイテムは含まれない。完全な IR は CLI の `tdsl build` で取得できる」ことを明示し、同意した場合のみ静的アイテムの IR を保存するようにした（#428 のフォローアップ）
- **VS Code 拡張のハイライトに `expand` / `qualifier` キーワードを反映**: v1.16.0 #361 で `keywords.ts` に追加された 2 キーワードが、コミット済みの生成ファイル `tdsl.tmLanguage.json` に反映されていなかった（生成は `npm run build` 時のみ実行されるため）。`gen-grammar-keywords.mjs` で再生成してドリフトを解消

### Docs

- **dsl-spec 両言語の expand 例の無効な構文を修正**: qualifier / expand セクションの例が `import wd as w { entity Q9682; }` + `map w to span` となっており実際の文法（map は `<import_alias>.<entity_key>` の dotted_ident 必須、import 元は `wikidata`）でパースエラーになっていた。`import wikidata as w { entity Q9682 as elizabeth_ii; }` + `map w.elizabeth_ii to span` に修正し、パースが通ることを検証した

## [1.17.0] - 2026-06-10

### Added

- **`tdsl render --show-event-labels` で Event / EventRange のラベルを常時表示**: SVG 年表でイベント内容をマウスオーバー（tooltip）なしで読めるようにする表示モードを追加。水平・垂直レイアウト両対応で、静的閲覧・印刷時の可読性を向上させる。デフォルトは従来どおり非表示 (#403)
- **LSP 補完をコンテキスト依存補完に切り替え**: カーソル位置のブロック構造（timeline / lane / group / map / import / アイテムオプション等）を解析し、現在編集中のブロック種別に応じたキーワード候補のみを提示するようにした。map コンテキストでは `claim()` / `label@` のスニペット補完も提供する (#367)
- **WebUI で import / map ブロック未解決時に Info 診断を表示**: WASM 環境では Wikidata fetch が実行されず import / map が silent にスキップされるため、該当ブロックに Info レベルの診断を表示して理由を明示するようにした。診断パネルに INFO ラベルとスタイルを追加 (#418)
- **WebUI のエクスポートメニューに JSON IR 保存を追加**: WASM バインディング `compile_to_ir()` を利用し、エディタの内容を JSON IR（pretty-print、`source_span` 付き）として `.json` ファイルでダウンロードできるようにした。コンパイルエラー時は Toast 通知でエラー内容を表示する (#428)
- **crates.io への cargo publish をリリースフローに組み込み**: `.github/workflows/release.yml` に `publish-crates` ジョブを追加し、タグ push 時に 4 コアクレート（`tdsl-parser` / `tdsl-wikidata` / `tdsl-core` / `tdsl-render`）を Trusted Publishing（OIDC）で crates.io に自動 publish する。ジョブは `continue-on-error` の独立ジョブで、失敗しても GitHub Release / npm / Homebrew をブロックしない。publish 前に workspace version と git tag の一致を検証する。publish 対象外の `tdsl-cli` / `tdsl-wasm` / `tdsl-lsp` には `publish = false` を設定し誤公開を防止。初回ブートストラップ手順（ローカルからの手動 publish + Trusted Publishing 設定）を `docs/release.md` に追記 (#424)
- **コアクレートを crates.io に公開**: `tdsl-parser` / `tdsl-wikidata` / `tdsl-core` / `tdsl-render` の各 `Cargo.toml` に `description` / `repository` / `keywords` / `categories` を追加し、内部依存に `path + version` の二重指定を整備した。`cargo add tdsl-core` 等で Rust プロジェクトから依存できるようになる。README に Rust ライブラリとしての使用例を追記 (#370)
- **PDF 出力に用紙サイズ・マージン・メタデータを追加**: `tdsl render --format pdf` に `--pdf-size`（`a4` / `a3` / `letter`）、`--pdf-landscape`（横向き）、`--pdf-margin`（mm）、`--pdf-title`（Title メタデータ上書き）フラグを追加。`svg2pdf::to_chunk` + `pdf-writer` で PDF を自前合成することで MediaBox・CreationDate・Title を記録する。未指定時は年表タイトルを Title に補完し、生成日を自動設定する。負値・非有限・印刷可能領域が残らない過大な `--pdf-margin` は空白／破損 PDF を黙って生成せず明示エラーで停止する (#368)

### Docs

- **examples/ に group・expand・qualifier の使用例ファイルを追加**: `examples/grouped_dynasties.tdsl`（`group` ブロックによる lane のグループ化、静的定義のみ）と `examples/officeholder_wikidata.tdsl`（`expand claim(P39)` + `claim(P39).qualifier(P580/P582).year` による役職の複数 span 展開、Wikidata 連携・オンラインビルド必須）を追加。CLAUDE.md / README.md / README.ja.md のサンプルファイル一覧にも追記（#421）
- **dsl-spec.en.md を日本語版（dsl-spec.md）と同期**: 取り残されていた英語版の言語仕様を日本語版ベースで全面更新。EBNF を v1.16 対応版（`group` ルール・`map_expr` / `lang_expr` / `label_ref`・filter / expand / qualifier）と文字単位で一致させ、`group` / filter 式 / qualifier アクセス / expand / template・apply（実装済み版）/ map の target_type 制約 / render の PDF・PNG・`--interactive` の各セクションを追加。旧「Constraints (MVP)」「Future Extensions」など実装と乖離した記述を削除し、セクション構成を日本語版と一致させた。あわせて CONTRIBUTING.md の「DSL 文法の変更手順」に dsl-spec の日英同時更新ルールを追記 (#427)
- **README.md / README.ja.md に既存機能の記載を追加**: `tdsl render` の `--watch`（変更監視・自動再レンダリング）/ `--grid`（補助グリッド線）/ `--orientation vertical`（縦方向レイアウト）のコマンド例をクイックスタートに、`group` ブロック構文の説明と例を DSL 文法セクションに追記（#426）
- **dsl-spec.md の EBNF を v1.13〜v1.16 の文法に同期**: v1.13 で追加された `group` ブロックの EBNF ルール（`<group>`）と本文説明を追加。`<mapping_rule>` の `start` / `end` / `time` が参照する式を未定義の `<expr>` から実際の文法に対応する `<map_expr>`（整数リテラル `??` フォールバック対応、#359）へ、`label` を `<lang_expr>` へ修正。filter の文字列マッチ（`contains` / `startswith`）が単一の `label@<lang>` 参照のみを取ることを `<label_ref>` として明確化 (#419)

### Tests

- **e2e-smoke.sh に v1.14〜v1.16 の新オプションのスモークケースを追加**: `tdsl render --grid decade/year/month`（グリッド線数の段階比較）、`--orientation vertical`（SVG が縦長になること）、`--show-table`（HTML に一覧表が付く／付けない場合は付かないこと）、`tdsl build --json-schema`（入力ファイルなしで TimelineIr スキーマを出力）を検証する。常駐プロセスとなる `tdsl render --watch` は対象外とし、理由をスクリプト内コメントに明記（#425）

## [1.16.0] - 2026-06-07

### Added

- **`tdsl render --show-table` で HTML 出力に内容一覧表を追加**: `--show-table` フラグを追加し、HTML 出力時に年表 SVG の直下に時系列順の内容一覧表（時期・ラベル・レーン・タグ列）を挿入する。Span / EventRange は開始〜終了、Event は時点を整形表示。`--format html` のみ有効で、SVG / PNG / PDF では無視される (#402)
- **`tdsl render --watch` でファイル変更時に自動再レンダリング**: `--watch` フラグを追加し、入力 `.tdsl` ファイルの変更を監視して自動的に再レンダリングする。`--output` が必須。`html` / `svg` フォーマットのみ対応（`png` / `pdf` は非対応）。Ctrl+C で終了する (#366)
- **Wikidata qualifier でのマッピングを追加**: map ブロックで `claim(P39).qualifier(P580).year` 構文により Statement の qualifier プロパティにアクセスできるようになった。また `expand claim(P39);` ディレクティブを追加し、1 エンティティの複数 Statement（例: 複数の役職）から複数アイテムを生成できるようになった。qualifier が存在しない Statement はスキップされる（silent fallback しない）。既存の `claim(P).accessor` 構文との後方互換性を維持 (#361)
- **`tdsl build --json-schema` で `TimelineIr` の JSON Schema を出力**: `schemars` クレートを `tdsl-core` に追加し、IR 型（`TimelineIr` / `Meta` / `Lane` / `Item` / `ImportRecord` / `SourceRecord` / `SourceSpan`）に `JsonSchema` を derive した。`tdsl build --json-schema` を実行すると入力ファイルなしで JSON Schema Draft 7 形式のスキーマを標準出力する。`--pretty` で整形出力、`--output` でファイルへの保存も可能。スキーマには Rust のドキュメントコメントが `description` フィールドとして反映される (#369)

### Fixed

- **install.sh を Linux aarch64 (arm64) に対応**: `detect_platform()` の Linux 分岐に `aarch64|arm64` ケースを追加し `tdsl-linux-aarch64.tar.gz` を取得するようにした。バイナリは release.yml で既に生成・配布されていたが、インストールスクリプトが未対応で ARM64 Linux ユーザーがエラー終了していた問題を修正。README / README.ja.md に対応プラットフォーム一覧（macOS x86\_64/arm64, Linux x86\_64/aarch64）を追記 (#387)

## [1.15.0] - 2026-06-03

### Added

- **WebUI のエクスポートを統合し PDF 出力に対応**: 既存のエクスポートメニュー（ダウンロード: .tdsl / SVG / HTML / PNG、コピー: SVG / PNG / Markdown / Share link）に **PDF 保存（印刷）** を追加した。PDF はブラウザのネイティブ印刷（`render_html` の出力を非表示 iframe に読み込み `print()` を呼ぶ）で生成するため、CJK ラベルがブラウザのフォントで正しくシェイプされる。CLI のベクタ PDF（`svg2pdf`）とは別経路で、WASM バンドルや Rust 依存は増やさない（ADR-0002 補遺を参照）。あわせて各ダウンロード関数の Blob→URL→クリック処理を `triggerDownload` ヘルパに集約して重複を解消した (#364)
- **WebUI の CodeMirror に Hover ツールチップを追加**: エディタ上の識別子・キーワードにカーソルを当てると定義情報を表示する。lane ID にはラベル / kind / order、import エイリアス・entity・query エイリアスにはインポート元（Wikidata エンティティ QID 等）、キーワードには簡潔な説明を表示する。解析はソースの静的解析（補完と同じ正規表現方式）で行い WASM 往復は不要。一定遅延（300ms）後に表示し、マウス移動で消える。タッチ環境は対象外（ポインタ前提）。LSP サーバの `textDocument/hover`（#309）に相当する機能を、LSP を使わない WebUI に提供する。あわせて補完の import エイリアス抽出が実文法（`import wikidata as wd` / `entity Q… as alias` / `query "…" as alias`）にマッチしていなかったバグを修正し、entity / query エイリアスと import 元を正しく候補に出すようにした (#363)
- **WebUI に lane/tag フィルタパネルを追加**: SVG プレビューで、レーンのチェックボックスとタグ検索によるフィルタリングができるようにした。`tdsl-render` が非インタラクティブ SVG でもアイテム `<g>` に `data-lane` / `data-tags` 属性を常時付与し、WebUI が DOM の opacity 制御でフィルタを反映する。フィルタ状態は sessionStorage に保存・復元される (#365)
- **map の `??` フォールバックに整数リテラルを追加**: map ブロックの start / end / time 式で、claim チェーン（`claim(P580).year ?? claim(P571).year`）に加えてリテラル整数フォールバック（`claim(P580).year ?? 9999`）が記述できるようにした。`grammar.pest` の `map_operand`、AST の `MapFallback`（`Claim` / `Literal`）、format / lowering、`docs/dsl-spec.md` を更新 (#359)

### Fixed

- **WASM `check_source` に正確な行・列を付与**: これまで全 Diagnostic で `line: 0, col: 0` をハードコードしていたため WebUI 診断パネルからエラー箇所へジャンプできなかった。パースエラーは `ParseError::source_location`、validation 警告はアイテムの `source_span` から **1-based** の行・列を取り出して Diagnostic に反映するようにした（IR `SourceSpan` / `render_svg_from_source` の `data-line` 属性と同じ番号付け）。これにより WebUI 診断パネルのクリックで該当行へジャンプできる。位置情報を持たない lowering エラー（未宣言 lane 等）は従来どおり `line: 0, col: 0`（クリック不可）。診断 JSON の形状（`severity` / `message` / `line` / `col`）は不変 (#386)

## [1.14.0] - 2026-06-02

### Added

- **`tdsl fmt` サブコマンドを追加**: `.tdsl` ファイルを正準スタイル（2 スペースインデント・ブロック間空行 1 行）にフォーマットする。デフォルトで整形結果を標準出力に出力。`--write` でファイルを上書き、`--check` で差分があれば非ゼロ終了（CI 向け）。`--check` と `--write` は排他。フォーマットには WebUI Format / `tdsl lint --fix` と同一の emitter（`tdsl_parser::format_source`）を使用する。現状フォーマットするとコメント（`//`・`/* */`）は失われる（grammar で COMMENT が silent のため。根治は別 issue で対応予定）(#351)
- **LSP に `textDocument/formatting` を実装**: LSP サーバが `tdsl_parser::format_source` による全文置換 `TextEdit` を返すようにした。パースエラー時・差分なし時は `None` を返し、UTF-16 末尾 `Position` を正確に計算する。`tdsl fmt` / WebUI Format と同一の正準フォーマッタを使用するため出力は一致する。ネットワーク I/O 不要（offline 前提）(#352)
- **`tdsl render --grid` オプションを追加（SVG 時間軸グリッド線）**: `RenderOptions` に `GridStyle`（`None` / `Decade` / `Year` / `Month`）を追加し、`tdsl render --grid decade|year|month|none` で補助グリッド線を描画できるようにした。水平・垂直レイアウト両対応。グリッド線は薄く（`stroke-opacity: 0.4`）描画し、支援技術に読み上げさせないよう `role="presentation"` を付与。デフォルトは `none` で既存 SVG 出力は不変（後方互換）(#353)
- **SVG の各アイテムに ARIA 属性を付与しアクセシビリティを改善**: span / event / event_range の `<g>` に `role="group"` と `aria-label="<種別>: <情報>、レーン: <レーン名>"` を付与。レーン帯背景・軸罫線・グリッド線・グループ区切り線等の装飾要素に `role="presentation"` / `aria-hidden="true"` を付与し、スクリーンリーダーへの不要な読み上げを抑制する。SVG ルートの `role="img" aria-label="timeline"` は既存のまま維持。`docs/styling.md` にアクセシビリティ方針を追記 (#354)
- **構文エラーを miette キャレットで強調表示**: `tdsl-parser` に `ParseDiagnostic` ラッパーを追加し、pest の `InputLocation` からバイトオフセット→`SourceSpan` を構成。CLI（`check` / `build`）で `ParseError` 発生時に miette の fancy レポート（キャレット付きスニペット）を stderr に出力するようにした。pest スニペットと miette スニペットの二重描画は解消済み。ライブラリ API（`ParseError`）は変更なしで後方互換を維持 (#355)

### Changed

- **未宣言 lane を Wikidata フェッチ前に early exit で検出**: lowering の pass1/pass2 でエラー（未宣言 lane 参照等）が出た場合、Wikidata フェッチを行わずに即座に `UnknownLane` を返すようにした。offline でも未宣言 lane を即報告でき、不要な API 呼び出しを回避する (#357)
- **`map` の `target_type` 制約を docs に明示しエラーメッセージを改善**: `target_type` が `span` / `event` / `event_range` のみ許可されることを `docs/dsl-spec.md` / `docs/error-catalog.md` に明記し、不正値に対するパースエラーメッセージを改善した (#358)

### Fixed

- **lint で無効なカレンダー日付（2 月 30 日等）を検出**: span / event / event_range の Date 精度の日付をカレンダー妥当性チェックし、2 月 30 日・4 月 31 日・非閏年の 2 月 29 日などを `invalid_calendar_date` warning として報告する。チェックには `ir.rs` の `days_in_month` / `is_leap_year` を再利用し、外部依存は追加しない (#350)

### Internal

- **SVG レンダリングの `unwrap()` を `fmt::Result` 伝播に置き換え**: `render_svg` と各描画ヘルパを `-> std::fmt::Result` / `-> Result<String, std::fmt::Error>` に変更し、`writeln!(...).unwrap()` を `?` に統一。`PdfError` / `PngError` に `Fmt(#[from] std::fmt::Error)` を追加。CLI・WASM の呼び出し側を `map_err` で伝播。本番コードの `unwrap()` は 24 → 0 箇所。出力 byte は不変 (#356)
- **CI にコードカバレッジ計測ジョブを追加**: `.github/workflows/ci.yml` に `coverage` ジョブを追加。`cargo-llvm-cov` を使ってプッシュ・PR 時に全クレートのカバレッジを計測し、lcov 形式のレポートを `coverage-report` アーティファクトとして 30 日間保存する。目標値を README に記載（`tdsl-parser` 70%+、`tdsl-core` 60%+、`tdsl-render` 50%+）。既存のテストジョブは変更なし (#371)
- examples の IR JSON・SVG 出力を insta スナップショットで固定（#372）

## [1.13.0] - 2026-05-30

### Added

- **`textDocument/codeAction` で lint auto-fix を quick fix として提供**: LSP サーバに `textDocument/codeAction` を追加。fixable な lint issue（`start_gt_end` / `invalid_tags` / `missing_id`）が存在するとき、`tdsl lint --fix` 相当の自動修正を 1 件の quick fix「tdsl: 自動修正可能な lint をすべて修正 (N 件)」として提示する。適用すると `WorkspaceEdit` でドキュメント全文が修正後ソースに置換される（`tdsl lint --fix` と**同一の emitter** を使うため出力は一致する。全文再 emit 方式のためコメントは整形時に失われる）。fixable でない issue（`unknown_lane` / `duplicate_id` / `empty_label`）しか無い場合は quick fix を出さない。client が `workspace.workspaceEdit.documentChanges` をサポートする場合、全文置換はドキュメントバージョン付きの `documentChanges` として返すため、コードアクション計算後に編集された場合は client がバージョン不一致を検出して stale な置換の適用を拒否する（新しい編集を上書きしない）。非対応クライアントには `changes`（バージョン保護なし）にフォールバックする。ネットワーク I/O 不要（offline 前提）。修正・再 emit ロジックは `tdsl-core::lint::fix_source`（`tdsl-parser::format_file` で AST を再 emit）として公開 API 化し、CLI / LSP で共有する (#311)
- **`tdsl render --orientation vertical`（垂直レイアウト）を追加**: `tdsl render` に `--orientation horizontal|vertical` オプションを追加し、時間軸を縦方向（上→下）に描画する垂直レイアウトをサポートした。`tdsl-render` の `RenderOptions` に `Orientation`（`Horizontal` / `Vertical`）を追加し、`LayoutModel` が向きに応じてレーン軸と時間軸を入れ替える。SVG / HTML / インタラクティブ HTML / PNG / PDF の全出力形式に対応。デフォルトは `Horizontal` で既存の出力・`.tdsl` ファイルは不変（後方互換）(#320)
- **`group` ブロックによる lane のグループ化を追加**: `group "名前" { lane ... }` 構文で複数の lane をまとめ、レンダリング時にグループラベルと境界を表示して視覚的に階層化できるようにした。`grammar.pest` に `group_decl` ルール、`tdsl-parser` の AST に `GroupDecl` を追加。`tdsl-core::ir::Lane` に `group: Option<String>`（`skip_serializing_if` で JSON 互換）を追加し、lowering で group 内の lane に group 名を付与する。`tdsl-render` がグループラベルとグループ境界線を SVG に描画する。`decompile` でも `group` ブロックを再現する。`group` を使わない既存 `.tdsl` はそのまま動作（後方互換）(#321)
- **`textDocument/references` で lane ID の全参照位置を返す**: LSP サーバに `textDocument/references` を追加。lane の宣言または参照箇所にカーソルを当てると、その lane ID の全参照位置（`lane` 宣言・`span` / `event` / `event_range` の `lane` 指定・`map` の `lane` プロパティ）を返す。`includeDeclaration` の指定を尊重する。ソーステキストの静的解析で解決し、ネットワーク I/O 不要（offline 前提）(#331)
- **`textDocument/rename` / `prepareRename` で lane ID の一括リネーム**: LSP サーバに `textDocument/rename` と `prepareRename` を追加。lane ID を宣言・全参照箇所まとめて `WorkspaceEdit` で一括置換する。`prepareRename` でリネーム可能な範囲を事前検証し、lane ID 以外（キーワード・文字列リテラル等）の上ではリネームを拒否する。ネットワーク I/O 不要（offline 前提）(#332)
- **`textDocument/documentSymbol` で階層シンボルを提供**: LSP サーバに `textDocument/documentSymbol` を追加。`timeline`（Module）> `lane`（Namespace）> `span` / `event` / `event_range`（Array / Event）の階層構造でドキュメントシンボルを返す。`source_span` を用いた正確な Range（CJK を含む UTF-16 オフセット対応）を付与する。パース不能・IR エラー時は黙ってフォールバックせず空リストを返す。ネットワーク I/O 不要（offline 前提）(#333)

### Changed

- **`tdsl lint --fix` の再 emit を正準フォーマッタに統一**: `tdsl lint --fix` の出力を、CLI 独自の emitter から `tdsl-parser` の正準フォーマッタ（`tdsl fmt` / WebUI Format と同じ `format_file`）に統一した。これにより LSP の Code Action（quick fix）と `tdsl lint --fix` が同一の出力になることを保証し、CLI 内に重複していた再 emit ロジック（約 370 行）を削除した。出力スタイルが従来のインライン 1 行から 2 スペースインデントの複数行に変わり、従来の CLI emitter が取りこぼしていた `timeline` の `color_map` ブロックも保持されるようになった (#311)

### Internal

- **`tdsl-core/src/lib.rs` を機能別テストモジュールに分割**: 約 2000 行に肥大化していた `lib.rs` 末尾の統合テストを `tests/{helpers,lower_static,lower_wikidata,validation}.rs` に分割し、`tests/mod.rs` から束ねる構成に整理した。プロダクションコード・テスト内容・挙動は不変（#319）
- **レンダリングのレイアウトエンジンを `LayoutModel` に集約**: `LaneBandModel`・色解決（`resolve_item_color` + レーンパレット）・ツールチップ生成（`item_tooltip` / `format_year` / `format_date` 等）を `svg.rs` から `layout.rs` の `LayoutModel` へ移動し、各 `LaidItem` に解決済みの `color` / `tooltip` を事前計算して持たせた。`svg.rs` は pre-resolved なプリミティブを SVG XML に変換するだけになった。出力は不変（既存テスト・e2e smoke 通過）(#322)

## [1.12.0] - 2026-05-27

### Added

- **`textDocument/definition` で lane 宣言へのジャンプを実装**: LSP サーバに `textDocument/definition` を追加。lane を参照している箇所（`span` / `event` / `event_range` の `lane` 指定、`map` の `lane` プロパティ）にカーソルを当てて Goto Definition を実行すると、対応する `lane` 宣言の位置にジャンプする。`tdsl-core::ir::Lane` に宣言位置を保持する `source_span: Option<SourceSpan>`（ソーステキストを渡した場合のみ付与、JSON 互換のため `skip_serializing_if`）を追加し、参照箇所の解決はソーステキストの静的解析で行う。ネットワーク I/O 不要（offline 前提）(#310)
- **`textDocument/hover` で hover 情報表示を実装**: LSP サーバに `textDocument/hover` を追加。lane ID にカーソルを当てるとラベル・kind・order を、QID にカーソルを当てるとキャッシュ済みエンティティ情報（ラベル・主要 claim 年）をマークダウンで表示する。ネットワーク I/O 不要（offline 前提）。`tdsl-wikidata` に `read_cached_entity` 関数を追加し、TTL 無視でキャッシュ読み出しを可能にした (#309)
- **`textDocument/completion` でキーワード補完を実装**: LSP サーバに `textDocument/completion` を追加し、BLOCK / ITEM / MISC の全 DSL キーワードを補完候補として返す（文脈非依存）。`crates/tdsl-lsp/src/keywords.rs` に Rust 側キーワードミラーを新設し、`apps/webui/src/lang-tdsl/keywords.ts`（単一真実源）との同期をドリフト防止テストで保証する (#308)

### Changed

- **npm パッケージに README / LICENSE を同梱**: `crates/tdsl-wasm/README.md`（npm 利用者向けの JS/TS 使用例・API 表）を追加し、`Release` ワークフローで pkg に README と root の MIT `LICENSE` を含めるようにした。これまで `@keroway/tdsl-wasm` の npm ページは README 未設定（"No README data found"）だったのを解消する
- **npm publish を Trusted Publishing（OIDC）に移行**: `@keroway/tdsl-wasm` の npm 公開を長期トークン（`NPM_TOKEN` / `NODE_AUTH_TOKEN`）方式から npm Trusted Publishing（OIDC）に切り替えた。`Release` ワークフローの `build-wasm` ジョブに `permissions: id-token: write` を付与し、publish 直前に npm CLI を 11.5.1+ に更新する。認証は OIDC で自動取得され、provenance attestation も自動付与される。GitHub Secrets への `NPM_TOKEN` 登録は不要になった。npmjs.com 側の Trusted Publisher 設定手順は README を参照。npm 未設定・障害時に本体（CLI バイナリ / Homebrew）リリースをブロックしない挙動（#314）は `continue-on-error` で維持
- **リリースワークフローの安定化**: Homebrew formula を毎回 4 プラットフォーム分の sha256 で完全再生成する方式に変更し、初回更新以降 sha が stale になる欠陥・aarch64-linux バイナリが無効・トップレベル URL がバージョン固定だったバグを解消（#315）。npm publish が未設定トークンや障害時でも本体（CLI バイナリ / Homebrew）リリースをブロックしないよう修正（#314）

### Internal

- **`tdsl-cli` の `main.rs` をサブコマンド別モジュールに分割**: 約 3000 行に肥大化していた `main.rs` を clap 引数定義と `fn main()` ディスパッチのみ（約 540 行）に整理し、各サブコマンドを `commands/` 配下の独立モジュールへ分割。共有ヘルパーを `commands/mod.rs` に集約。CLI の挙動・出力フォーマットは不変（#317）
- **lint ロジックを `tdsl-cli` から `tdsl-core` へ抽出**: `tdsl lint` / `tdsl lint --fix` の検出・修正ロジックを `tdsl-core::lint`（`LintIssue` / `LintSeverity` / `lint_issues` / `apply_lint_fixes`）として公開 API 化し、CLI / LSP 双方から再利用可能にした。LSP Code Action（#311）実装の前提リファクタ。振る舞い・出力・終了コードは現状と完全一致（#318）

## [1.11.0] - 2026-05-24

### Added

- **`tdsl lsp` サブコマンド（LSP サーバ）を追加**: `tdsl-lsp` クレートを新設し、`tower-lsp 0.20` をベースにした LSP サーバを提供。`textDocument/didOpen`・`didChange` を受けてパースエラー・検証警告を実際の line/col 付きで `publishDiagnostics` として返す。静的 lowering のみ（Wikidata fetch 不要）。ネットワーク不要で判定できる `map` / `apply` の静的参照エラー（未宣言の import alias / template）は offline でも Error として検出する。エンティティ解決が必要なブロックは黙って無視せず、各ブロック位置に Information レベルの診断（「offline 診断では未解決」）を表示する。Completion / Hover / Code Action は別 issue で実装予定（#307）
- **`tdsl-parser::ParseError::source_location`**: パースエラーの 1-based 行・列を返すアクセサを追加。Syntax variant は pest `line_col`、バイトオフセット variant（`InvalidInt` 等）はソーステキストから算出（#307）
- **`tdsl-core::validate::validate_with_spans`**: バリデーション警告を item の `source_span` に紐付けた構造化診断を返す新関数。既存 `validate()` はこの薄いラッパに変更（後方互換維持）（#307）
- **`tdsl-core::validate::validate_static_references`**: AST から、ネットワーク不要で判定できる `map` / `apply` の参照エラー（未宣言 import alias / template）をバイト span 付きで収集する新関数。LSP の offline 診断で利用（#307）
- **`tdsl-core::ir::SourceSpan` に `PartialEq` を追加**（#307）
- **`tdsl render --format pdf` を追加**: `svg2pdf` / `usvg` 経由でベクター PDF を出力できるようになった。`tdsl-render/pdf` Cargo feature でゲートされており、WASM ビルドへの影響なし。PNG と対称な `PdfError` / `PdfOptions` / `render_pdf` / `svg_to_pdf` API を `tdsl-render` に追加。システムフォントを `fontdb` 経由でロードするため CJK レーンラベルも適切に描画される（#265、ADR-0002）
- **`tdsl render --format png` に `--dpi` / `--png-scale` オプションを追加**: `PngOptions { dpi, scale_factor }` を `tdsl-render` に追加し、`--dpi <N>`（デフォルト 96）で SVG ユーザー単位からピクセルサイズを `dpi / 96.0` 倍にスケール、`--png-scale <f>` で倍率を直接指定できるようにした。両者は clap の `conflicts_with` で排他制御。HTML/SVG 出力には影響しない（#264）
- **時間式に整数オフセット演算（`+` / `-`）を追加**: `map` ブロックの時間値式で `start claim(P569).year + 1` / `end claim(P570).year - 5` のような整数オフセットを記述できるようにした。`ClaimExpr` に `offset: Option<i32>`（`None` デフォルトで後方互換）を追加し、lowering 時に year へ加算する。decompile / inspect 出力および WebUI Format でも `+ N` / `- N` 形式で再現される（#148）

### Changed

- `tdsl-wasm` is now published to npm as `@keroway/tdsl-wasm` on each release tag push. Install with `npm install @keroway/tdsl-wasm` ([#292])
- **SVG CSS スコープの改善**: SVG ルート要素に `class="tdsl-root"` を追加し、埋め込みスタイルのセレクタを `.tdsl-root text { }` にスコープ。Obsidian 等の外部ホストに SVG をインライン埋め込みした際のグローバル `text { }` セレクタによる CSS 干渉を防ぐ ([#293])
- **`RenderOptions::font_family` フィールドを追加**: SVG 出力のフォントファミリーをカスタマイズ可能に。`None`（デフォルト）は CJK 対応フォントスタックを維持 ([#293])

## [1.10.1] - 2026-05-21

### Fixed

- **リリース CI のクロスビルドターゲット未インストール問題を修正**: v1.10.0 で追加した `rust-toolchain.toml`（#279）の影響で `dtolnay/rust-toolchain` の `targets:` 指定が無視され、`x86_64-apple-darwin` 等のホスト以外のターゲットがインストールされなかったため、`Release` ワークフローで `error[E0463]: can't find crate for 'core'` により全マトリクスがキャンセルされ、CLI バイナリ配布と Homebrew formula 更新がスキップされていた。`release.yml` に明示的な `rustup target add` ステップを追加して復旧

## [1.10.0] - 2026-05-20

### Added

- **`tdsl render --format png` を追加**: `resvg` / `usvg` / `tiny-skia` をオプション feature `tdsl-render/png` 経由で統合し、内部 SVG を PNG にラスタライズして出力できるようにした。`fontdb` に system fonts を読み込むため、Noto Sans JP / Hiragino Sans / Yu Gothic 等が利用可能な環境では CJK レーンラベルもそのまま描画される。`tdsl-cli` は `png` feature を有効化して同梱、`tdsl-wasm` は依存させないため WASM バンドルサイズへの影響なし。composite action `action.yml` も `format: png` をサポート。後続の DPI / scale オプション（#264）と PDF 出力（#265）は別 PR で対応予定（#263）
- **WebUI に Format（整形）ボタンと `Ctrl/Cmd+Shift+F` を追加**: AST→再 emit 方式で 2 スペースインデント・ブロック間空行 1 行の標準形に整形する。整形は CodeMirror transaction 経由なので Undo/Redo が機能する。パース失敗時は Toast でエラー通知のみ行い、エディタ内容は変更しない。コメントは AST に残らないため整形時に削除される（コメントを含むソースを整形した場合は Toast で警告）。`tdsl-parser` に `format_source` 公開関数、`tdsl-wasm` に同名のバインディングを追加（#262）
- **`tdsl import-csv` で月日リテラルを受理**: `start` / `end` / `time` 列が `YYYY-MM-DD` / `YYYY-MM` / `YYYY` の 3 精度を判別パースするようになった。v1.9.0 の月日精度サポートを CSV 経路にも拡張。`tdsl-parser` に `parse_time_literal` を公開し、grammar 由来の検証ロジックを再利用（#260）
- **VS Code 拡張に月日リテラル対応スニペットを追加**: `span-day` / `span-month` / `event-day` / `event_range` / `tl-day` スニペットを追加し、`event_range` の不足を解消（#259、拡張版 v1.7.0 として公開）

### Changed

- **CI: Node.js を 22 LTS に統一**: WebUI ビルド・VS Code 拡張ビルド・LP ビルド系の各ワークフローを `actions/setup-node@v5` + Node.js 22 に揃え、複数バージョンの差異から来る不安定さを解消（#280）
- **Rust ツールチェーンを `rust-toolchain.toml` で pin**: CI と開発環境の Rust バージョン差を解消するため、ワークスペース直下に `rust-toolchain.toml` を追加した（#279）
- **WebUI ESLint 違反の解消と CI での自動実行**: `apps/webui` の既存違反を全て修正し、CI に `npm run lint` を追加して新規違反の混入を防ぐ（#270）
- **依存更新**: `getrandom` を tdsl-wasm の `0.3` 系統に更新し、合わせて `0.3.4 → 0.4.2` への bump にも追従（#281、#285）
- **VS Code 拡張の `engines.vscode`**: 一時的に `^1.85.0` へ引き上げたが互換性影響を考慮して `^1.75.0` に差し戻し（#282、#283）

### Docs

- **`docs/dsl-spec.md` の EBNF を月日精度対応に更新**: `YYYY-MM` / `YYYY-MM-DD` リテラルおよび `.month` / `.day` アクセサを正式に明文化（#258）
- **`docs/cli-spec.md` を新規作成**: 全 CLI サブコマンド（`build` / `check` / `ast` / `fetch` / `search` / `inspect` / `resolve` / `scaffold` / `render` / `init` / `import-csv` / `lint` / `decompile` / `merge` / `cache` / `completions`）のリファレンスドキュメントを整備（#267）

### Tests

- **`tdsl-render` クレートのテスト整備**: SVG / HTML / インタラクティブ HTML / PNG レンダリングのスナップショット相当テストを追加し、回帰検知体制を強化（#261）
- **`tdsl-wasm` クレートのテスト整備**: WASM バインディングの compile / render / format_source 各 API について `wasm-bindgen-test` を用いたテストを整備（#266）

## [1.9.0] - 2026-05-17

### Added

- **日付リテラルのパース対応（YYYY-MM / YYYY-MM-DD）**: パーサで月精度・日精度の日付リテラルを受理し、AST `TimeValue` に `Year` / `YearMonth` / `Date` の 3 バリアントを導入。比較は `to_sortable() -> (i64, u8, u8)` で行う（#243）
- **月・日精度の静的 lowering / range 拡張 / decompile 対応**: core で月・日精度のアイテムを lowering し、混在範囲補完（`range_start_month` 等）・decompile（IR → `.tdsl`）も月日対応。既存の年精度 IR JSON は完全後方互換（#247）
- **月・日精度レンダリング（`unit day` / `day_ticks` / 日精度ラベル）**: render で `unit day` をサポートし、月・日精度の軸ラベル・自動 tick 配置を実装（#248）
- **CLI シェル補完スクリプト生成（`tdsl completions`）**: bash / zsh / fish / PowerShell / elvish の補完スクリプトを生成するサブコマンドを追加（#244）
- **WebUI プレビュー全画面表示モード**: プレビューエリアを画面全体に拡大表示できる全画面トグルを実装。`?preview=1` クエリでも起動可（#241）
- **WebUI エディタにインラインエラーハイライト**: `@codemirror/lint` を統合し、構文エラー/警告を CodeMirror 内の波線アンダーラインとガターアイコンで表示。ホバーで tooltip 表示。既存の診断パネルは維持（#239）
- **WebUI 分割ペイン比率を LocalStorage に永続化**: エディタ/プレビューの分割比率をドラッグ完了時に LocalStorage に保存し、ページリロード後も維持されるようにした（#240）
- **WebUI Ctrl/Cmd+S で `.tdsl` ソースをダウンロード**: キーボードショートカットでエディタ内容を `.tdsl` ファイルとしてダウンロード可能にした（#246）

### Docs

- **月・日精度の時間表現 仕様設計書**: `docs/spec-date-precision.md` を追加し、日付リテラル文法・IR スキーマ拡張・後方互換性方針を明文化（#242）

## [1.8.0] - 2026-05-16

### Added

- **WebUI ダークモード自動追従**: `prefers-color-scheme` メディアクエリに連動してダーク/ライトテーマを自動切り替え。Toast 通知コンポーネントを追加し、コピー成功等の操作フィードバックを表示するようにした（#233）
- **WebUI 共有 URL コピー**: pako 圧縮 + URL Hash でエディタ内容をエンコードし、クリップボードにコピーできる「共有」ボタンを追加（#234）
- **WebUI LocalStorage 自動保存・リストア**: エディタ内容を LocalStorage に自動保存し、ページ再読み込み時にリストアする機能を追加（#227）
- **WebUI 履歴スナップショット**: コンパイル成功時に自動で最大5件のスナップショットを保存し、手動保存・復元・削除ができる履歴パネルを追加（#228）
- **WebUI プレビュー↔エディタ双方向ジャンプ**: SVG アイテムをクリックすると対応する DSL 行へカーソルが移動し、エディタのカーソル行に対応する SVG アイテムをハイライトする双方向ナビゲーションを実装（#230）
- **WebUI 設定パネル永続化**: 設定パネル（PNG 背景色・スケール・履歴表示トグル等）を LocalStorage に保存し、再訪問時に設定を復元するようにした（#225）
- **WebUI ヘッダー再編**: ヘッダーメニューを「ファイル / テンプレート / エクスポート / 設定 / About」に縮約・整理（#224）
- **サンプル追加**: `examples/internet_history.tdsl`（インターネット史）を追加し、ギャラリーから利用可能にした（#232）

### Docs

- **チュートリアル・DSL 仕様書の英語翻訳**: `docs/tutorial.md` および `docs/dsl-spec.md` の英語版を追加（#223）

## [1.7.0] - 2026-05-14

### Added

- **テンプレートギャラリーモーダル**: WebUI に「テンプレート」ボタンを設置し、`examples/*.tdsl` を全件ロードできるモーダルを実装。概要テキスト付きリストからサンプルを選択するとエディタにロードされる（#214）
- **月・日精度のタイムライン軸表示**: Wikidata precision=10（月）・precision=11（日）を持つアイテムのレンダリング時に、月・日精度に応じた軸ラベルを表示するよう対応（#212）

### Changed

- **シンタックスハイライト キーワード自動生成**: `apps/webui/src/lang-tdsl/keywords.ts` を単一真実源として、VS Code TextMate grammar（`tdsl.tmLanguage.json`）を `npm run build` 時に自動生成するよう変更。`color_map`・`policy`・`title`・`field_priority`・`origin` の同期漏れを解消（#207）

## [1.6.0] - 2026-05-13

### Added

- **Wikidata precision 対応（月・日精度）**: Wikidata TimeValue の precision フィールド（10=月, 11=日）を解析し、IR の `Span`/`Event`/`EventRange` に `start_month`/`start_day`/`end_month`/`end_day`/`time_month`/`time_day` フィールドとして伝播させる。`skip_serializing_if` により既存 JSON IR との後方互換を維持。DSL 構文に `.month`/`.day` アクセサを追加（`claim(P569).month` 等）（#146）
- **WebUI サンプル拡充**: `examples.ts` に「DSL 基本文法（最小構成）」と「Wikidata インポート（オフライン不可）」の 2 件を追加。合計 4 件以上になり文法カバレッジを向上（#200）
- **`?source=` URL クエリパラメータ対応**: WebUI に `?source=` クエリパラメータを追加し、URL からエディタ初期内容を設定できる deep link に対応（#201）
- **コンパイル失敗時の stale プレビュー保持**: WebUI でコンパイルエラー発生時に直前の成功 SVG を表示し続け、「直前の成功時プレビューを表示中」バッジを重畳表示するようにした（#202）

### Changed

- **README 英語化**: `README.md` を英語版に置き換え、日本語版を `README.ja.md` として温存。両ファイルに相互リンクを追加（#90）

### Docs

- **シンタックスハイライト grammar 整合方針を文書化**: `apps/webui/src/lang-tdsl/index.ts` と `editors/vscode/syntaxes/tdsl.tmLanguage.json` の二重管理方針（手動同期）を `CLAUDE.md` と `apps/webui/README.md` に明記。フォローアップ issue #207 を起票（#203）

## [1.5.0] - 2026-05-13

### Added

- **時間値フォールバック演算子（`??`）の map expr 対応**: `map` ブロックの `start` / `end` / `time` でも `??` 演算子をサポートし、複数の Wikidata プロパティから最初の有効値を取得できるようにした（#143）。例: `start claim(P580).year ?? claim(P571).year`。既存の単項式（`start claim(P571).year`）は完全互換
- **ARM Linux (aarch64) バイナリ配布**: `aarch64-unknown-linux-musl` ターゲットをリリース CI のビルドマトリクスに追加し、`tdsl-linux-aarch64.tar.gz` を GitHub Release に同梱（#145）。Homebrew formula も Linux ARM 環境を判定して適切なバイナリを取得するように更新
- **`map` フィルター句**: `map` ブロックに `filter <expr>;` 句を導入し、インポートしたエンティティから条件に合うものだけをアイテム化できるようにした（#142）。`==`, `!=`, `<`, `<=`, `>`, `>=`, `&&`, `||`, `!`, `null`, `claim()` をサポート。複数の `filter` は AND 結合
- **WebUI モバイル対応**: WebUI でモバイル（≤768px）時にエディタ/プレビューをタブ切り替え、タブレット（769px〜1024px）時に上下2段レイアウトに自動切り替えする対応を追加（#141）。タッチ操作向けにボタン高さを 36px 以上に拡大
- **WebUI ドラッグ分割幅調整**: エディタとプレビューの間にドラッグ可能な 4px 幅のディバイダーを追加し、分割比率を 15%〜85% の範囲で変更できるようにした（#150）。デフォルト比率は 40% / 60%（エディタ / プレビュー）
- **CI: `.tdsl` 変更時 SVG プレビュー自動生成**: `.tdsl` ファイルが変更された PR でのみ `tdsl-preview` ジョブが起動し、SVG をレンダリングしてアーティファクトとして保存するワークフローを追加（#171）
- **CI: PR へのプレビューコメント自動投稿**: `tdsl-preview` ワークフローから PR に IR サマリ（アイテム数・レーン数・期間）を含むプレビューコメントを自動 upsert するようにした（#172）
- **GitHub Actions composite action**: `uses: keroway/timeline-dsl@v1` で `.tdsl` → SVG / HTML レンダリングを CI から呼び出せる composite action を公開。`docs/ci-integration.md` にユースケース別レシピを追記（#173）
- **テストカバレッジ強化**: `tdsl-wikidata` に precision バリエーション（世紀・十年・千年・日）のテストを追加、`tdsl-cli` に `build`・`check`・`ast` サブコマンドのユニットテストを追加（#132）

### Fixed

- **WebUI Auto スケール時の年ラベル重なり解消**: `render_svg_from_source` の Auto scale クランプ上限を `8.0 → 50.0 px/yr` に引き上げ、短期間レンジ（例: `range 2018..2030`）で年ラベルが重なる問題を解消（#194）

### Changed

- **CI: `in-progress` ラベル自動剥離**: issue または PR が close されると `in-progress` ラベルを自動剥離するワークフローを追加（#162）

## [1.4.0] - 2026-05-10

### Added

- **WebUI CodeMirror 言語拡張**: `@codemirror/lang-markdown` 流用を廃止し、TDSL 専用の StreamParser ベース言語拡張を実装（#136）。キーワード・QID・PID・`claim()` 式・文字列・コメントをダーク／ライト両テーマで色分け
- **WebUI 診断行クリックジャンプ**: 診断パネルのエラー行をクリックするとエディタの該当行へスクロール・フォーカスする機能を追加（#139）
- **Wikidata fetch プログレス表示**: `indicatif` クレートを導入し、Wikidata fetch 中にスピナーと進捗バーを表示（#135）
- **Wikidata Retry-After 対応**: HTTP 429 レスポンスの `Retry-After` ヘッダを尊重してリトライ待機。デフォルトリトライ回数を 5 回に引き上げ、`--wikidata-retries` で設定可能（#140）
- **`--wikidata-timeout` CLI オプション**: Wikidata API リクエストのタイムアウト秒数を指定できる `--wikidata-timeout` オプションを追加（#134）
- **VS Code スニペット**: `timeline` / `lane` / `span` のスニペットテンプレートを追加（#137）

### Changed

- **CJK フォントスタック改善**: SVG レンダリング時のフォント指定に Noto Sans JP を優先し、CJK 文字の表示品質を改善（#138）
- **エラーチェーン統一**: `serde_json::to_string*().unwrap()` を `?` によるエラーチェーンに統一（#131）
- **ベンチマーク**: `criterion::black_box` を `std::hint::black_box` に移行（#154）

## [1.3.0] - 2026-05-06

### Added

- **WebUI スケール（ズーム）制御**: タイムラインプレビューのスケールをスライダーで調整できる機能を追加（#128）。ホイールズームと独立した倍率コントロールを搭載

## [1.2.2] - 2026-05-06

### Added

- **VS Code 拡張機能の README / CHANGELOG**: Marketplace ページに説明・コード例・各種リンク（LP / WebUI / GitHub）を表示するため `editors/vscode/README.md` および `editors/vscode/CHANGELOG.md` を追加（#120）

## [1.2.1] - 2026-05-06

### Changed

- **GitHub Actions**: `vscode-publish` ワークフローを Node.js 24 / `actions/setup-node@v5` に更新（#126）

## [1.2.0] - 2026-05-06

### Added

- **VS Code Marketplace 公開**: `editors/vscode/` 拡張機能を VS Code Marketplace に公開（#120, #125）。`ext install keroway.timeline-dsl` でインストール可能
- **vscode-publish ワークフロー**: タグプッシュ時に Marketplace へ自動公開する GitHub Actions を追加。VSIX を GitHub Release にも添付

## [1.1.0] - 2026-05-06

### Added

- **color_map ブロック**: `timeline` ブロック内でタグ→色マッピングを宣言的に定義できる DSL 構文を追加（#67）。`tdsl render --color-map` フラグでも上書き可能
- **decompile コマンド**: JSON IR を `.tdsl` ソースに逆変換する `tdsl decompile` コマンドを追加（#68）
- **WebUI MVP**: WASM + Vite/React によるブラウザ上エディタを追加（#62）。CodeMirror 6 によるシンタックスハイライト・リアルタイム SVG プレビュー・診断パネル・SVG ダウンロードを搭載。GitHub Pages にデプロイ（#105）
- **インタラクティブHTML出力**: `tdsl render --interactive` でズーム・パン・アイテム検索・凡例・詳細パネルを搭載したインタラクティブ HTML を生成（#49）
- **SVG 直接出力**: `tdsl render --format svg` でスタンドアロン SVG ファイルを出力（#50）
- **Windows バイナリ対応**: CI・リリースワークフローに Windows ターゲットを追加（#58）
- **キャッシュ管理 CLI**: `tdsl cache status` / `tdsl cache clear [--older-than <days>]` でローカル Wikidata キャッシュを管理（#45, #46）
- **フィールド別インポート優先度**: `import` ブロック内で `policy field_priority { label: manual; time: wikidata; tags: merge; }` によりフィールド単位のマージ戦略を指定可能（#47）
- **Criterion ベンチマーク**: パーサ・lowering・レンダリングのパフォーマンスを計測する benchmark suite を追加（#60）
- **CONTRIBUTING.md・Issue テンプレート**: コントリビューション手順と Bug/Feature Issue テンプレートを追加（#78, #79）

### Fixed

- `tdsl render` のインタラクティブモードにホイールズーム機能を追加
- WebUI プレビューの UX 改善（ツールチップ・スクロール対応・サンプルラベル修正）
- Clippy の全警告を修正（`fix/v1.1.0-quality-improvements`）
- セキュリティ: rustls-webpki を 0.103.13 に更新（DoS 脆弱性 GHSA-82j2-j2ch-gfr8 対応）

## [1.0.0] - 2026-05-04

### Added

- **template / apply 構文**: 共通フォーマットをテンプレート化して再利用できる DSL 構文を追加（#39）
- **Wikidataキャッシュ**: 取得結果を `~/.cache/tdsl/` に保存し TTL/オフライン連携を実現（#28）
- **Wikidata APIリトライ**: HTTP 429・5xx に対する exponential backoff リトライ（最大3回）を追加（#48）
- **SVG直接出力**: `tdsl render --format svg` でスタンドアロン SVG ファイルを出力（#50）
- **VS Code構文ハイライト**: `editors/vscode/` に TextMate grammar ベースの VS Code 拡張を追加（#52）
- **Homebrew formula**: `brew tap keroway/tap && brew install tdsl` によるインストールをサポート（#57）
- **E2Eテスト拡充**: 全 12 CLIサブコマンドの正常系・異常系を網羅した E2E スクリプトを整備（#59）
- **Getting Startedチュートリアル**: `docs/tutorial.md` を追加（#54）
- **サンプルファイル拡充**: 日本史・戦国武将（Wikidata連携）・世界大戦・科学技術の4サンプルを追加（#55）

### Fixed

- エラー診断の改善: 未定義lane参照時に利用可能な lane 候補を提示（#40）

### Changed

- リリースワークフローに Homebrew formula 自動更新ジョブを追加
- README.md にエディタサポートセクションと Homebrew インストール手順を追加

## [0.1.0] - 2026-04-20

### Added

- PEG文法 + パーサ（7種のstatement: timeline, lane, span, event, event_range, import, map）
- AST → IR変換（静的定義 / Wikidata連携 両方）
- Wikidata HTTPクライアント（wbgetentities API, wbsearchentities, SPARQL）
- CLI 12サブコマンド: `build` / `check` / `ast` / `fetch` / `search` / `inspect` / `resolve` / `scaffold` / `render` / `init` / `import-csv` / `lint`
- JSON IR出力（`origin` フィールドを含む）
- コメント構文（行 `//` / ブロック `/* */`）
- `map` の `target_type` enum 検証（span / event / event_range のみ許可）
- imported item の `source` を `wd:<entity_id>` で自動付与
- 日本語 lane 名で `as` 省略時の `lane_N` 自動採番
- 静的アイテム（event / event_range）の `sources[]` 登録
- 再インポートポリシー（merge_by_source / overwrite_imported / keep_manual）
- `query "SPARQL" as alias` による複数エンティティ一括インポート
- HTMLレンダリング（`tdsl-render` クレート、インラインSVG）
- `tdsl lint` による品質チェックと自動修正（`--fix`）
- validate における `start > end` チェック
- SPARQL QID 抽出改善

[1.28.0]: https://github.com/keroway/timeline-dsl/releases/compare/v1.27.0...v1.28.0
[1.27.0]: https://github.com/keroway/timeline-dsl/releases/compare/v1.26.0...v1.27.0
[1.26.0]: https://github.com/keroway/timeline-dsl/releases/compare/v1.25.0...v1.26.0
[1.25.0]: https://github.com/keroway/timeline-dsl/releases/compare/v1.24.0...v1.25.0
[1.24.0]: https://github.com/keroway/timeline-dsl/releases/compare/v1.23.0...v1.24.0
[1.23.0]: https://github.com/keroway/timeline-dsl/releases/compare/v1.22.0...v1.23.0
[1.22.0]: https://github.com/keroway/timeline-dsl/releases/compare/v1.21.0...v1.22.0
[1.21.0]: https://github.com/keroway/timeline-dsl/releases/compare/v1.20.0...v1.21.0
[1.20.0]: https://github.com/keroway/timeline-dsl/releases/compare/v1.19.0...v1.20.0
[1.19.0]: https://github.com/keroway/timeline-dsl/releases/compare/v1.18.0...v1.19.0
[1.18.0]: https://github.com/keroway/timeline-dsl/releases/compare/v1.17.0...v1.18.0
[1.17.0]: https://github.com/keroway/timeline-dsl/releases/compare/v1.16.0...v1.17.0
[1.16.0]: https://github.com/keroway/timeline-dsl/releases/compare/v1.15.0...v1.16.0
[1.15.0]: https://github.com/keroway/timeline-dsl/releases/compare/v1.14.0...v1.15.0
[1.14.0]: https://github.com/keroway/timeline-dsl/releases/compare/v1.13.0...v1.14.0
[1.13.0]: https://github.com/keroway/timeline-dsl/releases/compare/v1.12.0...v1.13.0
[1.12.0]: https://github.com/keroway/timeline-dsl/releases/compare/v1.11.0...v1.12.0
[1.11.0]: https://github.com/keroway/timeline-dsl/releases/compare/v1.10.1...v1.11.0
[1.10.1]: https://github.com/keroway/timeline-dsl/releases/compare/v1.10.0...v1.10.1
[1.10.0]: https://github.com/keroway/timeline-dsl/releases/compare/v1.9.0...v1.10.0
[1.9.0]: https://github.com/keroway/timeline-dsl/releases/compare/v1.8.0...v1.9.0
[1.8.0]: https://github.com/keroway/timeline-dsl/releases/compare/v1.7.0...v1.8.0
[1.7.0]: https://github.com/keroway/timeline-dsl/releases/compare/v1.6.0...v1.7.0
[1.6.0]: https://github.com/keroway/timeline-dsl/releases/compare/v1.5.0...v1.6.0
[1.5.0]: https://github.com/keroway/timeline-dsl/releases/compare/v1.4.0...v1.5.0
[1.4.0]: https://github.com/keroway/timeline-dsl/releases/compare/v1.3.0...v1.4.0
[1.3.0]: https://github.com/keroway/timeline-dsl/releases/compare/v1.2.2...v1.3.0
[1.2.2]: https://github.com/keroway/timeline-dsl/releases/compare/v1.2.1...v1.2.2
[1.2.1]: https://github.com/keroway/timeline-dsl/releases/compare/v1.2.0...v1.2.1
[1.2.0]: https://github.com/keroway/timeline-dsl/releases/compare/v1.1.0...v1.2.0
[1.1.0]: https://github.com/keroway/timeline-dsl/releases/compare/v1.0.0...v1.1.0
[1.0.0]: https://github.com/keroway/timeline-dsl/releases/compare/v0.1.0...v1.0.0
[0.1.0]: https://github.com/keroway/timeline-dsl/releases/tag/v0.1.0
