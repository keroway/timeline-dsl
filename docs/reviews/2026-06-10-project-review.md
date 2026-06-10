# プロジェクト全体レビュー（2026-06-10）

v1.16.0 リリース直後の棚卸しとして実施した、リポジトリ全体のレビュー記録。

## 調査範囲と方法

3 領域を並列で調査し、主要な指摘はファイル・行番号レベルで手動再検証した。

| 領域 | 対象 |
|------|------|
| Rust ワークスペース | crates/ 配下 7 クレート（parser / core / wikidata / render / wasm / cli / lsp）のコード品質・構造・テスト分布 |
| フロントエンド・エディタ統合 | apps/webui / editors/vscode / tdsl-wasm / tdsl-lsp の機能ギャップと品質 |
| ドキュメント・CI・配布 | docs/ / .github/workflows/ / examples/ / scripts/ / 配布チャネル全般 |

## 健全と評価した点

- **CI/CD**: 最小権限（`permissions: contents: read`）、actions のバージョン固定、Rust キャッシュ、npm Trusted Publishing（OIDC）、VS Code publish の version 整合チェック、dependabot の lockstep 制御まで整備済み。
- **エラー処理の一貫性**: 全クレートが thiserror ベースで、anyhow の公開 API への漏れなし。本番コード（`crates/*/src/` の非テスト部）に unwrap/panic の規約違反は実質なし。
- **LSP**: diagnostics / completion（コンテキスト依存）/ hover / goto definition / code action / document symbols / find references / rename / formatting の 10 機能が実装・テスト済み（計 95 テスト）。
- **キーワードのドリフト防止**: `keywords.ts`（単一真実源）↔ `tdsl-lsp/src/keywords.rs` の同期テスト、VS Code grammar の自動生成が機能している。
- **WebUI の型安全性**: any 型 0 件。localStorage / WASM 初期化 / 変換失敗のエラーハンドリングも一貫。
- **配布チャネル**: install.sh（aarch64 対応済み）/ Homebrew / cargo-binstall / npm / VS Code Marketplace / GitHub Actions composite action と網羅的。
- **テスト分布**: parser 93 / lsp 95 / render 98 件など主要クレートは充実。

## 検出した課題と起票結果

| Issue | 内容 | 優先度 |
|-------|------|--------|
| [#424](https://github.com/keroway/timeline-dsl/issues/424) | crates.io への cargo publish がリリースフロー（release.yml / docs/release.md）に未組み込み。README は「published」と記載しており、リリースごとにバージョンドリフトする | 高 |
| [#425](https://github.com/keroway/timeline-dsl/issues/425) | e2e-smoke.sh に --grid / --orientation / --show-table / --json-schema のスモークがない | 中 |
| [#426](https://github.com/keroway/timeline-dsl/issues/426) | README 両言語に --watch / --grid / --orientation / group が未記載 | 中 |
| [#427](https://github.com/keroway/timeline-dsl/issues/427) | dsl-spec.en.md が日本語版と非同期（66 行差、group 未記載）。#419 完了後に同期 | 中 |
| [#428](https://github.com/keroway/timeline-dsl/issues/428) | WebUI: `compileToIr` が WASM バインディングに実装済みなのに UI から未使用（JSON IR エクスポート欠落） | 中 |
| [#429](https://github.com/keroway/timeline-dsl/issues/429) | lint が WASM 未公開で WebUI から使えない（lint は #318 で tdsl-core 抽出済み） | 中 |
| [#430](https://github.com/keroway/timeline-dsl/issues/430) | App.tsx が 1,833 行に肥大化。コンポーネント分割（#420 着手前が望ましい） | 中 |
| [#431](https://github.com/keroway/timeline-dsl/issues/431) | WebUI のテストが 0 件。Vitest 導入（share.ts / history.ts から） | 中 |
| [#432](https://github.com/keroway/timeline-dsl/issues/432) | layout.rs の compute_item_horizontal / vertical がほぼ重複（両方に too_many_arguments allow 付き） | 低 |
| [#433](https://github.com/keroway/timeline-dsl/issues/433) | lower.rs が 1,277 行。パス別モジュール分割 | 低 |
| [#434](https://github.com/keroway/timeline-dsl/issues/434) | CI のベンチが compile-only で性能回帰を検知できない | 低 |
| [#435](https://github.com/keroway/timeline-dsl/issues/435) | WebUI モーダルに focus trap がない | 低 |

既存 open issue と重複するため起票しなかった指摘: WASM RenderOptions パラメータ化（#417）、dsl-spec EBNF 更新（#419）、orientation/grid/theme の WebUI 設定 UI（#420）、group/expand/qualifier の examples（#421）、VS Code への LSP 統合（#388）。

## 誤判定の記録（次回レビューへの教訓）

探索エージェントが「本番コードの unwrap/panic 規約違反」として報告した以下は、**すべて `#[cfg(test)]` モジュール内**であり誤判定だった:

- `crates/tdsl-cli/src/commands/lint.rs:94-126`
- `crates/tdsl-core/src/decompile.rs:298-326`
- `crates/tdsl-cli/src/commands/init.rs:578`

自動調査の指摘は、ファイル内の `#[cfg(test)]` 境界を確認してから採用すること（2026-06-02 の「未実装誤判定」に続き 2 回目の同種事例）。

## 方針提案（新しい視点）

### 1. LSP 資産の活用 — #388 の優先度引き上げ

LSP は 10 機能・95 テストまで実装が進んでいるのに、配布されている VS Code 拡張はハイライト + スニペットのみで、LSP の恩恵を受けられるのは `tdsl lsp` を手動設定できるユーザーだけ。実装資産が遊休状態にある。#388（VS Code への LSP クライアント統合、現在 priority:low / future）を次々回スプリント候補に引き上げる価値がある。

### 2. WebUI を「プレイグラウンド」から「エディタ」へ

現在の WebUI は共有 URL・localStorage・履歴と「試す場」として完成度が高い。次の段階として、File System Access API による .tdsl ファイルの直接オープン・上書き保存、PWA 化（オフライン利用）を視野に入れると、日常的な年表編集ツールとして使えるようになる。#430（分割）→ #417/#420（レンダリング制御）→ #428/#429（IR export / lint）の順で土台を整えた先の構想として記録する。

### 3. v1.17 候補テーマ: WebUI レンダリング制御一式

#417（WASM RenderOptions）→ #420（orientation/grid/theme UI）→ #428（JSON IR エクスポート）→ #429（lint 統合）は依存関係が一直線で、まとめて「WebUI レンダリング制御」スプリントにすると CLI と WebUI の機能差が大きく縮まる。前提リファクタとして #430 を先頭に置くのが望ましい。

### 4. ドキュメントの構造的な同期ずれ対策

今回の検出（README 未記載 #426、en 版非同期 #427）は、機能実装時のチェックリストに docs が含まれていても「どの docs か」が曖昧なために起きている。`docs/release.md` または PR テンプレートに「README（両言語）/ dsl-spec（両言語）/ cli-spec / examples」の明示的なチェック行を足すと再発しにくい。mdBook 等によるドキュメントサイト統合は、各ドキュメントの重複（README と spec の機能説明など）を単一ソース化できるなら検討の価値があるが、現状の規模では優先度低。

## issue 化しなかった軽微な指摘（補遺)

- `editors/vscode/scripts/gen-grammar-keywords.mjs` は keywords.ts を正規表現でパースしており、TS 側の書式変更に脆い。keywords を JSON に切り出して双方が import する形がより堅牢（現状はドリフト防止テストがあるため実害なし）。
- `crates/tdsl-core/src/decompile.rs:23-47` の `writeln!(...).unwrap()`: String への書き込みは infallible なので実害はないが、規約上は `let mut s = String::new()` + `push_str` か、infallible である旨のコメントが望ましい。
- `crates/tdsl-lsp/src/backend.rs:86` の capability fallback（`unwrap_or(false)`）が silent。ログ出力かコメントでの明文化を検討。
- `tdsl-cli` の build / fetch コマンドはユニットテストが薄い（統合テスト `tests/cli_integration_test.rs` と e2e-smoke が主なカバー）。#425 で一部補完される。
- CONTRIBUTING.md と docs/release.md のリリース手順に若干の重複がある。#424 の手順追記時に整理すると良い。
