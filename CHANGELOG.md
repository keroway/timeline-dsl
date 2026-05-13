# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

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

[1.5.0]: https://github.com/keroway/timeline-dsl/releases/compare/v1.4.0...v1.5.0
[1.4.0]: https://github.com/keroway/timeline-dsl/releases/compare/v1.3.0...v1.4.0
[1.3.0]: https://github.com/keroway/timeline-dsl/releases/compare/v1.2.2...v1.3.0
[1.2.2]: https://github.com/keroway/timeline-dsl/releases/compare/v1.2.1...v1.2.2
[1.2.1]: https://github.com/keroway/timeline-dsl/releases/compare/v1.2.0...v1.2.1
[1.2.0]: https://github.com/keroway/timeline-dsl/releases/compare/v1.1.0...v1.2.0
[1.1.0]: https://github.com/keroway/timeline-dsl/releases/compare/v1.0.0...v1.1.0
[1.0.0]: https://github.com/keroway/timeline-dsl/releases/compare/v0.1.0...v1.0.0
[0.1.0]: https://github.com/keroway/timeline-dsl/releases/tag/v0.1.0
