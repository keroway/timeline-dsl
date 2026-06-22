# リリース手順

このドキュメントでは `timeline-dsl` のリリース時に更新が必要なファイルと手順を説明します。

---

## リリース時に一括バンプするファイル一覧

新バージョン（例: `vX.Y.Z`）をリリースする際は、以下のファイルをすべて同一 PR でバンプしてから git tag を打つこと。

| ファイル | 更新箇所 |
|----------|---------|
| `Cargo.toml` | `[workspace.package].version` |
| `editors/vscode/package.json` | `"version"` フィールド |
| `editors/vscode/CHANGELOG.md` | `## [Unreleased]` の直下に `## [X.Y.Z] - YYYY-MM-DD` セクションを追加 |
| `CHANGELOG.md`（本体） | `## [Unreleased]` の直下に `## [X.Y.Z] - YYYY-MM-DD` セクションを追加 |

> **注意**: `editors/vscode/package.json` の version と git tag の version が一致していないと、
> `.github/workflows/vscode-publish.yml` の整合性チェックが失敗して Marketplace への publish がブロックされます。

---

## 手順

### 1. バージョンバンプ PR を作る

```bash
# ブランチを切る
git checkout -b chore/release-vX.Y.Z

# Cargo.toml workspace.package.version を更新
# editors/vscode/package.json の version を更新
# editors/vscode/CHANGELOG.md に新セクションを追加
# CHANGELOG.md に新セクションを追加

git add Cargo.toml editors/vscode/package.json editors/vscode/CHANGELOG.md CHANGELOG.md
git commit -m "chore: release vX.Y.Z"
```

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
