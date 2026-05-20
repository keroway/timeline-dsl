# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

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
