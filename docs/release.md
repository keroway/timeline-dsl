# リリース手順

このドキュメントでは `timeline-dsl` のリリース時に更新が必要なファイルと手順を説明します。

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
- `TAP_BUMP_TOKEN` が未設定の場合、ジョブは失敗せず `::notice` を出して**静かにスキップ**する
  （他のリリース成果物をブロックしないための意図的な設計。逆に言うと、見落とすと
  Homebrew 側だけが更新されないまま気付かれない）
- 確認手順: Actions タブで `update-homebrew-formula` ジョブのログを確認し、
  「スキップされた」旨の notice が出ていないこと、および
  `keroway/homebrew-tap` に bump PR が実際に作成されていることを確認する

---

## crates.io への公開

### 通常フロー（タグ push で自動）

git tag の push により `.github/workflows/release.yml` の `publish-crates` ジョブが起動し、
4 コアクレート（`tdsl-parser` / `tdsl-wikidata` / `tdsl-core` / `tdsl-render`）を
crates.io に自動 publish します（認証は Trusted Publishing / OIDC、長期トークン不要）。

- ジョブは `continue-on-error` の独立ジョブなので、crates.io publish が失敗しても
  GitHub Release / npm / Homebrew のリリースはブロックされません。
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

### 手動再 publish（CI 失敗時のフォールバック）

`publish-crates` ジョブが失敗した場合は、ローカルから API トークンで再 publish できます。

```bash
# crates.io のトークンを設定済みであること（cargo login）
cargo publish -p tdsl-parser -p tdsl-wikidata -p tdsl-core -p tdsl-render --locked
```

すでに publish 済みのクレートはエラーになるため、未公開のクレートのみ `-p` で指定し直してください。

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
