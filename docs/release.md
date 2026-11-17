# リリース手順

このドキュメントでは `timeline-dsl` のリリース時に更新が必要なファイルと手順を説明します。

現行の判断材料・残課題・隔週判断日は[次版の公開判断資料](release-readiness.md)を参照してください。

---

## リリース時に一括バンプするファイル一覧

新バージョン（例: `vX.Y.Z`）をリリースする際は、以下のファイルをすべて同一 PR でバンプしてから git tag を打つこと。

| ファイル | 更新箇所 |
|----------|---------|
| `Cargo.toml` | `[workspace.package].version` |
| `crates/tdsl-core/Cargo.toml` | `tdsl-parser` / `tdsl-wikidata` の内部依存 `version = "X.Y.Z"` 要求（2箇所） |
| `crates/tdsl-render/Cargo.toml` | `tdsl-core` / `tdsl-parser` の内部依存 `version = "X.Y.Z"` 要求（dev-deps 含め3箇所） |
| `editors/vscode/package.json` | `"version"` フィールド |
| `editors/vscode/package-lock.json` | ルートの `version` と `packages[""].version`。`cd editors/vscode && npm install --package-lock-only` で揃う（v2.0.0 の PR #808 でこの行が無かったため取りこぼし、自動レビューに指摘された） |
| `editors/vscode/CHANGELOG.md` | `## [Unreleased]` の直下に `## [X.Y.Z] - YYYY-MM-DD` セクションを追加 |
| `CHANGELOG.md`（本体） | `## [Unreleased]` の直下に `## [X.Y.Z] - YYYY-MM-DD` セクションを追加。末尾の compare リンク一覧にも `[X.Y.Z]: .../compare/v<前バージョン>...vX.Y.Z` を追加すること |

> **注意**: `editors/vscode/package.json` の version と git tag の version が一致していないと、
> `.github/workflows/vscode-publish.yml` の整合性チェックが失敗して Marketplace への publish がブロックされます。
>
> **注意**: `crates/tdsl-core/Cargo.toml` / `crates/tdsl-render/Cargo.toml` の内部依存 version を
> 揃え忘れると、`publish-crates` ジョブが「`tdsl-core X.Y.Z` が `tdsl-parser =古い版` を要求している」
> 不整合で失敗します（後述「Cargo.toml の一括バンプ方法」参照。`cargo set-version --workspace` を使えば
> この表の全バージョン値がまとめて揃うため、手動編集より強く推奨）。
>
> `apps/webui/package.json` は `"private": true` の非公開パッケージのため version バンプは不要です。

---

## 手順

### 1. バージョンバンプ PR を作る

**まず CHANGELOG の突合を行う**（過去に何度も漏らしている作業。飛ばさないこと）:

```bash
# 前回タグ以降の全コミットを、CHANGELOG.md の [Unreleased] と1行ずつ突き合わせる。
# feature/fix PR だけでなく、chore/ci/security/deps の PR も CHANGELOG 記載対象になり得る
# （dependabot/renovate の細かい依存更新はまとめて1行に要約してよい）。
git log v<前バージョン>..HEAD --oneline
```

続けてバンプ本体:

```bash
# ブランチを切る
git checkout -b chore/release-vX.Y.Z

# workspace 全体のバージョンを一括更新（内部依存の version も自動で揃う。後述）
cargo set-version X.Y.Z --workspace

# editors/vscode/package.json の version を更新
# editors/vscode/CHANGELOG.md に新セクションを追加
# CHANGELOG.md に新セクションを追加 + 末尾の compare リンクを追加

# バンプ後は Cargo.lock を更新し、同一コミットに含める
# （更新を忘れると Stop hook が "lockfile stale" で失敗する）
cargo check --workspace

git add Cargo.toml Cargo.lock crates/tdsl-core/Cargo.toml crates/tdsl-render/Cargo.toml \
  editors/vscode/package.json editors/vscode/CHANGELOG.md CHANGELOG.md
git commit -m "chore: release vX.Y.Z"
```

> PR マージ後は追加 PR を挟まず即タグを打つこと（間に別 PR が入ると tag 時点の内容と
> CHANGELOG の突合内容がずれる）。

### 2. PR をマージし、git tag を打つ

```bash
git tag vX.Y.Z
git push origin vX.Y.Z
```

tag の push により `.github/workflows/vscode-publish.yml` が起動します。
ワークフロー内で `editors/vscode/package.json` の version と tag が一致しているか検証されます。
不一致の場合はワークフローがエラーで停止します。

### 3. GitHub Releases を確認する

- Actions タブでワークフローが成功していることを確認
- GitHub Releases に `.vsix` が添付されていることを確認
- [VS Code Marketplace](https://marketplace.visualstudio.com/items?itemName=keroway.timeline-dsl) に新バージョンが反映されていることを確認

---

## Homebrew tap への反映

`.github/workflows/release.yml` の `update-homebrew-formula` ジョブ（`release` ジョブの後続、
`needs: release`）が、`keroway/homebrew-tap` へ Formula 更新の bump PR を自動作成します。

- 認証は `TAP_BUMP_TOKEN` シークレット（`keroway/homebrew-tap` への書き込み権限を持つ PAT）。
  **リリース前に、このシークレットが `timeline-dsl` リポジトリに設定されていることを確認すること**
- `TAP_BUMP_TOKEN` が未設定の場合、ジョブは**失敗する**（#811）。以前は `::notice` を出して
  静かにスキップしていたが、Homebrew 側だけが更新されないまま気付かれないため、
  リリースの可視な失敗として扱うよう変更した
- `gh pr create` が失敗した場合も同様にジョブを**失敗させる**（#861）。以前は終了コードを
  検査しておらず、PR が作られていなくてもジョブが成功していた
- 確認手順: Actions タブで `update-homebrew-formula` ジョブが成功していることと、
  `keroway/homebrew-tap` に `bump-tdsl-X.Y.Z` の PR が実際に作成されていることを確認する
- **formula の構造変更に注意**: ジョブは formula を毎回全量再生成して上書きする。
  caveats / 補完生成 / test 拡張など構造の変更を tap 側に入れる場合は、必ず
  `scripts/generate-homebrew-formula.sh` の heredoc にも反映すること。
  同期しないと次回リリースで構造が消える（実際に v2.1.0 で caveats・補完・test 拡張が
  消えた。keroway/homebrew-tap#96）

---

## crates.io への公開

### 通常フロー（タグ push で自動）

git tag の push により `.github/workflows/release.yml` の `publish-crates` ジョブが起動し、
4 コアクレート（`tdsl-parser` / `tdsl-wikidata` / `tdsl-core` / `tdsl-render`）を
crates.io に自動 publish します（認証は Trusted Publishing / OIDC、長期トークン不要）。

- 公開先ごとの独立ジョブとして失敗を残します（`continue-on-error` は使いません）。
  crates.io / npm の失敗だけでは GitHub 成果物と Homebrew の処理を止めません。
- ジョブ内で `[workspace.package].version` と git tag の一致を検証します
  （不一致なら publish せずエラー停止。vscode-publish.yml の整合チェックと同趣旨）。
- cargo 1.90+ の multi-package publish が依存順（parser/wikidata → core → render）を
  自動解決するため、順序を意識する必要はありません。

確認手順:

- Actions タブで `publish-crates` ジョブが成功していることを確認
- <https://crates.io/crates/tdsl-parser> / <https://crates.io/crates/tdsl-wikidata> /
  <https://crates.io/crates/tdsl-core> / <https://crates.io/crates/tdsl-render> に
  新バージョンが反映されていることを確認

### ブートストラップ手順（初回のみ）

crates.io の Trusted Publishing は「クレートが既に crates.io に存在する」ことが前提のため、
**初回はローカルから API トークンで手動 publish する必要があります**
（npm の Trusted Publishing ブートストラップ — README.md の npm 公開セクション参照 — と同様の制約）。

1. [crates.io の API Tokens](https://crates.io/settings/tokens) で `publish-new` スコープの
   トークンを発行し、`cargo login` で設定する
2. リポジトリ root で 4 クレートを一括 publish する

   ```bash
   cargo publish -p tdsl-parser -p tdsl-wikidata -p tdsl-core -p tdsl-render --locked
   ```

3. 公開後、各クレートの crates.io ページ → Settings → Trusted Publishing に
   GitHub の設定を追加する
   - Repository owner: `keroway`
   - Repository name: `timeline-dsl`
   - Workflow filename: `release.yml`
   - Environment: 空欄
4. 以降はタグ push で `publish-crates` ジョブが自動 publish する

### 既存タグを指定して再実行

Release / Publish VS Code Extension の `workflow_dispatch` に既存タグ `vX.Y.Z` を指定します。
従来の npm 用 `version` 入力は廃止しました。手動実行は既定ブランチに限定し、運用コードはその workflow ref、
ビルド・公開内容は検証済みタグの SHA から別ディレクトリへ checkout します。
タグ形式、SHA、Cargo workspace・公開クレート・npm生成物・VSIXの版が不一致なら停止します。

```bash
gh workflow run release.yml --ref main -f tag=vX.Y.Z
gh workflow run vscode-publish.yml --ref main -f tag=vX.Y.Z
# 状態表だけ再確認（公開処理を呼ばない）
gh workflow run release-status.yml --ref main -f tag=vX.Y.Z
```

公開済みの npm は tarball の integrity と `release-source.json`、crates.io は checksum と
`.cargo_vcs_info.json`、Marketplace は VSIX 内の版・`release-source.json` を検証してスキップします。
GitHub 成果物は隣接する `.provenance.json` のタグ版・SHA・SHA256と実際の asset digest を照合します。
Marketplace等のHTTP gzip応答は展開してから配布物を検証し、転送時・展開後とも128 MiBを上限にします。不正な圧縮や未対応のContent-Encodingは確認不能として停止します。
既存の成果物を `--clobber` で上書きしません。Homebrew は既存PR・同内容のbranchを再利用し、
force-pushをしません。異なる内容のbranchは停止してレビュー対象にします。

この変更以前の配布物に証跡ファイルが無い場合、同一版というだけでは成功・スキップと見なしません。
「確認不能」として停止し、当時のビルド・配布物・ソースSHAの対応を別途調査します。
このため旧タグの再試行は未公開の配布先だけが進み、証跡不足の既公開先は失敗する場合があります。
版やタグを付け替えて回避しないでください。

## 配布状況の状態表

Release本文の `tdsl-distribution:start` / `tdsl-distribution:end` 内だけを管理します。
CLI 5アーカイブ・WASM・npm・4クレート・VSIX・Marketplace・Homebrewを
「公開済み / 処理中 / 未完了 / 確認不能」で表示し、ソースSHAと確認時刻を記録します。

- 本体・VS Code workflowの終了時に同じ `release-status.yml` を呼びます。失敗したジョブは失敗のまま残ります。
- upload同士は `release-assets-<tag>`、状態表更新同士は `release-status-<tag>` で直列化します。
  公開workflow自体もworkflow別・タグ別に直列化し、手動の状態再確認が待機uploadを取り消す競合を避けます。
  集計は実行時に公開先を読み直すので、後から終わったworkflowが最新状態を反映します。
- 別workflowがまだ実行中なら未到達の配布先は「処理中」。API失敗・SHA不一致・証跡不足は「確認不能」です。
- Homebrewは「PR作成済み・マージ待ち」と「tapの既定branchに反映済み」を区別します。
  PR作成だけでは「公開済み」になりません。
- Release自体がまだ無い場合は Actions job summary に状態表を出します。
  手動の状態再確認だけでReleaseを新規作成することはありません。
- 通常のリリースノート本文は保持します。手書き編集は管理ブロックの外へ記載してください。
- 完了直後のレジストリ反映遅延やworkflow取消で表が古い場合は、状態再確認を実行します。

fixture検証は `python3 -m unittest discover -s scripts/tests -p 'test_release*.py' -v`
（Python 3.11以上）。テストから実公開を行いません。

---

## Cargo.toml の一括バンプ方法

```bash
# workspace 全クレートのバージョンを一括更新（cargo-edit を使う場合）
cargo set-version X.Y.Z --workspace

# cargo-edit が入っていない場合は手動で Cargo.toml を編集する
# [workspace.package]
# version = "X.Y.Z"
```

> **注意（手動編集時）**: `[workspace.package].version` だけでなく、各クレートの
> 内部依存に書かれた `version = "X.Y.Z"` 要求も同じ版へ揃えること。
> 例: `crates/tdsl-core/Cargo.toml` の `tdsl-parser = { path = "...", version = "X.Y.Z" }`、
> `crates/tdsl-render/Cargo.toml` の `tdsl-core` / `tdsl-parser`（dev-deps 含む）。
> `cargo set-version --workspace` を使えば自動で揃うため、手動編集より推奨。
> 揃え忘れても `^` セマンティクスで publish 自体は通るが、公開クレートが宣言する
> 依存要求が古い版に固定され不整合になる。

---

## 関連ドキュメント

- [CONTRIBUTING.md](../CONTRIBUTING.md) — 開発フロー全般
- [CHANGELOG.md](../CHANGELOG.md) — 本体の変更履歴
- [editors/vscode/CHANGELOG.md](../editors/vscode/CHANGELOG.md) — VS Code 拡張の変更履歴

## TAP_BUMP_TOKEN更新後の実効権限確認

`TAP_BUMP_TOKEN`の保存先はこのリポジトリのActions repository secret、PATの対象は
`keroway/homebrew-tap`。Fine-grained PATにはContentsとPull requestsのRead and writeが必要。
更新日時の確認や、既にtapへ反映済みのタグでのRelease再実行は、push権限の確認にならない。
後者は公開済みFormulaを検出してトークンを使う処理をスキップする。

`check-tap-token.yml`をmainへマージした後、既定ブランチから手動で実行する:

```bash
gh workflow run check-tap-token.yml --repo keroway/timeline-dsl --ref main
```

このworkflowは同じPATでtapへの一時ブランチpush、Draft PR作成・close、ブランチ削除を
実行する。名前は`token-check/tdsl-<run ID>-<attempt>`で、変更は`.token-check/tdsl.txt`だけ。
Formula、リリース、既存ブランチは変更しない。commitの`[skip ci]`でtapのpush/PR CIを抑止する。
手動実行専用で、PRからの自動実行・定期実行はない。既定ブランチ以外からの実行はスキップされるため、
それを検証成功と扱わない。値を画面やログへ出さず、secretを書き換える処理もない。

通常終了・エラー時はEXIT trapで後片付けし、PR closeやブランチ削除に失敗した場合もworkflowを失敗させる。
強制停止・runner消失・応答を受け取れない通信切断ではリソースが残る可能性がある。
失敗時はログのrun IDに対応するDraft PRとブランチを確認し、PRをcloseしてからその一時ブランチだけを削除する。
検証用PRをマージしないこと。push、PR作成、後片付けがすべて成功したrunを実効権限確認の証拠とする。

オフラインの回帰テストは通常の配布fixtureに含む:

```bash
python3 -m unittest discover -s scripts/tests -p 'test_release*.py' -v
```
