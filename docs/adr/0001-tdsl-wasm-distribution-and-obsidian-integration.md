# ADR 0001: tdsl-wasm の配布方式と Obsidian 連携の方針

- **Status**: Accepted
- **Date**: 2026-05-21
- **Deciders**: keroway
- **Related issues**: #291（本 ADR）, #147（親）, #292, #293, #294, #295
- **Supersedes**: なし

## コンテキスト

Obsidian ノート内の `` ```tdsl `` コードブロックを SVG プレビューに置換するプラグイン（親 issue #147）を実現するには、`tdsl-wasm` を外部プロジェクトから依存解決できる形で配布する必要がある。

現状の配布・参照経路は次のとおりで、外部利用者向けの配布方式が未確定である。

- `tdsl-wasm` は release タグ push で `.github/workflows/release.yml` の `build-wasm` ジョブが `wasm-pack build crates/tdsl-wasm --target web --release` を実行し、生成物（デフォルト出力先の `crates/tdsl-wasm/pkg/`）を tar.gz に固めて GitHub Release に自動アップロードする運用となっており、npm 等の registry には公開されていない。外部プロジェクトからの依存解決経路がない。
- `crates/tdsl-wasm/pkg/` は wasm-pack のデフォルト出力先で、`crates/tdsl-wasm/pkg/.gitignore` により全ファイルが ignore 対象となっており、リポジトリには含まれない。CI ジョブが一時的に生成するか、開発者がローカルで `wasm-pack build` を実行したときに生成される。
- WebUI が利用する WASM 成果物のコミット済み source of truth は `apps/webui/src/wasm/` 配下のみ（`tdsl_wasm.js` / `tdsl_wasm.d.ts` / `tdsl_wasm_bg.wasm` / `tdsl_wasm_bg.wasm.d.ts` / `package.json` の 5 ファイルが tracked）で、wasm-pack が出力する `.gitignore` のみ `apps/webui/.gitignore` で除外している。`apps/webui/src/wasmLoader.ts` から `./wasm/tdsl_wasm.js` として直接 import される。
- `apps/webui/src/wasm/` の更新は、開発者が `apps/webui` ディレクトリで `wasm-pack build ../../crates/tdsl-wasm --target web --out-dir src/wasm --no-opt`（`apps/webui/README.md` に記載）を実行し、`--out-dir` で `apps/webui/src/wasm/` に直接出力されたファイルをそのままコミットする運用に依存している（中間で `crates/tdsl-wasm/pkg/` を経由する手動コピー手順は存在しない）。自動同期スクリプト・CI ステップも存在しない（`apps/webui/README.md:51` には「`src/wasm/` は .gitignore 対象」と記載があるが、実際には tracked であり README の記述が古い）。
- `.github/workflows/deploy-pages.yml`（GitHub Pages 配信）は WebUI ビルド時に WASM を再ビルドせず、コミット済みの `apps/webui/src/wasm/` をそのまま使う。CI（`.github/workflows/ci.yml`）の `build-wasm` ジョブは `wasm-pack build crates/tdsl-wasm --target web` を `pkg/` 出力で実行し export を検証する smoke test のみで、`apps/webui/src/wasm/` への同期は行わない。
- `crates/tdsl-wasm/src/lib.rs` は静的 lowering（`lower_static_with_source`）のみを呼んでおり、`import` 文を含む `.tdsl` は WikidataClient が必須のため lowering エラーになる（= ブラウザ環境では Wikidata 連携不可）。

後続 sub-issue（#292〜#295）の前提となる決定事項を、本 ADR で確定する。

## 決定事項

### D1. 配布レジストリ: 公式 npm（npmjs.com）

- `tdsl-wasm` の WebAssembly バンドルを公式 npm registry へ publish する。
- 採用理由:
  - Obsidian プラグインのビルドツール（esbuild）から `npm i` だけで素直に依存解決できる。
  - エコシステム標準であり、外部利用者の追加設定（`.npmrc` 等）が不要。
- 代替案として検討したが採用しなかったもの:
  - **GitHub Packages**: 利用者側に `.npmrc` 設定を強いるため、Obsidian プラグイン利用者の導入摩擦が大きい。
  - **tar.gz 維持のみ**: 外部プロジェクトから依存解決が成立せず、後続 sub-issue（#292・#294・#295）の前提を満たさない。

### D2. パッケージ名: `@keroway/tdsl-wasm`（スコープ付き）

- npm 上のパッケージ名はスコープ付きの `@keroway/tdsl-wasm` とする。
- publish 時は `--access public` を指定して公開パッケージとする。
- 採用理由:
  - 無スコープ名は衝突リスクがあり、占有確認・取得コストを要する。
  - スコープ付きであればプロジェクトの所有者が明確で、将来 GitHub Packages 併用も容易。
- メタデータ（`pkg/package.json`）に最低限以下を含める:
  - `name: "@keroway/tdsl-wasm"`
  - `publishConfig.access: "public"`
  - `repository`, `homepage`, `bugs`, `license: "MIT"`
  - `version` は CI で Cargo workspace の version から注入（後述 D5）。

### D3. obsidian-tdsl リポジトリ運用: 独立リリースサイクル

- Obsidian プラグインは `keroway/obsidian-tdsl` の**別リポジトリ**で管理する。
- 本リポジトリ（`keroway/timeline-dsl`）の release タグとは**連動させず**、Obsidian 側は独自のタイミングでリリースする（疎結合）。
- 採用理由:
  - Obsidian Community Plugin の申請ポリシーが独立リポジトリ前提であり、monorepo 化は実質却下。
  - リリース連動の自動化（PR 自動作成、CI 連結）は両リポの保守コストが大きく、`tdsl-wasm` の API が安定してくれば手動更新で十分。
  - `@keroway/tdsl-wasm` を npm 依存として参照することで、必要なタイミングで Obsidian 側がバージョンを上げる運用が成立する。
- ライセンスは本リポと同じ **MIT** に揃える。

### D4. Wikidata 連携不可の制約の伝え方: WASM 側で明示エラーを整形して返す

- `tdsl-wasm` の公開 API（`compile_to_ir` / `render_svg_from_source` / `render_html_from_source` / `check_source`）の入口で、AST に `import` 文が含まれる場合は専用の整形済みエラーメッセージを返す。
- メッセージ例（実装時に確定）:
  > Wikidata import はブラウザ環境（Obsidian / WebUI）では利用できません。`import` 文を取り除くか、ローカル CLI（`tdsl build`）で IR を生成してください。
- 採用理由:
  - 既存の lowering エラーをそのまま見せる方式（README/UI 警告のみ）では、ユーザーが原因を特定しづらい。
  - feature flag で parser/lowering 側から import 機能を完全に除外する方式は実装規模が大きく、保守コストに見合わない。
- 影響範囲: `crates/tdsl-wasm/src/lib.rs` の入口チェック追加のみ。`tdsl-parser` / `tdsl-core` には変更を入れない。
- README（本リポおよび `obsidian-tdsl`）にも同制約を明記する。

### D5. 付随する決定事項

ADR 本体決定に付随する技術判断を以下に明示する。後続 sub-issue で参照されたら、本 ADR を rationale として扱う。

- **バージョニング**: Cargo workspace の version を CI で `pkg/package.json` の `version` に注入する現行方式（`build-wasm` ジョブの該当ステップ）を踏襲する。npm パッケージのバージョンは本リポの release タグに 1:1 で連動する。
- **publish trigger**: release タグ push で自動 publish。緊急時の再 publish 用に `workflow_dispatch` も併設する。
- **`NPM_TOKEN`**: GitHub Secrets に登録する。トークン取得・ローテーション手順は #292 の作業範囲で README に追記する。
- **WebUI（`apps/webui`）の参照方式**: 当面は `apps/webui/src/wasm/` 配下のコミット済み成果物をそのまま参照する現行方式を維持する（開発者が `wasm-pack build ../../crates/tdsl-wasm --target web --out-dir src/wasm --no-opt` を実行してコミットする運用も継続）。`@keroway/tdsl-wasm` への npm 依存切替・自動同期スクリプト化・README の「.gitignore 対象」記述の訂正は本 ADR の範囲外とし、別 issue で検討する。
- **`pkg/package.json` のメタデータ整備**: CI で `name`, `version`, `publishConfig.access`, `repository`, `homepage`, `bugs`, `license` を確実に注入できるよう、`build-wasm` ジョブのスクリプトを拡張する（#292 で実施）。

## 後続 sub-issue の前提条件（決定事項の要約）

| sub-issue | 本 ADR で確定した前提 |
|-----------|----------------------|
| #292 (NPM 配布基盤) | 公式 npm に `@keroway/tdsl-wasm` を release タグ駆動で publish。`NPM_TOKEN` を GitHub Secrets に登録。`build-wasm` ジョブで `publishConfig.access=public` を含むメタデータ注入を行う。`workflow_dispatch` でも publish 可能とする。 |
| #293 (SVG/API 整備) | `compile_to_ir` 等の公開 API 入口で `import` 文検出時の整形済みエラーを実装する（D4）。SVG class の `.tdsl-root` スコープ化・フォント外部指定など外部統合向けの API 整備を含む。 |
| #294 (外部リポ scaffold) | `keroway/obsidian-tdsl` を独立リポジトリで作成、ライセンスは MIT。`@keroway/tdsl-wasm` を npm dep として取り込む。本リポからのリリース自動連動は行わない。 |
| #295 (プラグイン本体) | `@keroway/tdsl-wasm` の API 安定後に Obsidian 側の独自タイミングで実装。Wikidata 不可は README とプラグイン UI の両面で明示する。 |

## 影響範囲

- **新規ファイル**: 本 ADR（`docs/adr/0001-...md`）、および将来作成する `keroway/obsidian-tdsl` リポジトリ。
- **変更が見込まれるファイル**（後続 sub-issue で対応）:
  - `.github/workflows/release.yml` — `build-wasm` ジョブに publish ステップ追加（#292）
  - `crates/tdsl-wasm/src/lib.rs` — `import` 検出による明示エラー（#293, D4）
  - `crates/tdsl-wasm/Cargo.toml` の `package.metadata.wasm-pack` — 必要に応じてメタデータ調整（#292）
  - `README.md` / `README.ja.md` — 外部利用者向けインストール手順の追記（#292）
  - `CHANGELOG.md` — 配布方式変更の記録

## 未決定事項（本 ADR の範囲外）

- WebUI（`apps/webui`）を `@keroway/tdsl-wasm` の npm 依存に切り替えるか、`apps/webui/src/wasm/` のコミット済み成果物を継続維持するか。継続する場合でも、`wasm-pack build ... --out-dir src/wasm` の実行を CI/スクリプトで自動化するか、および `apps/webui/README.md` の「`src/wasm/` は .gitignore 対象」という誤記の修正は別途検討。
- Obsidian Community Plugin への正式申請のタイミングと前提条件（#295 で検討）。
- `tdsl-wasm` の semver 運用ルール（破壊的変更時の major bump 基準）。
