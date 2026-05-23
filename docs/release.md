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

## Cargo.toml の一括バンプ方法

```bash
# workspace 全クレートのバージョンを一括更新（cargo-edit を使う場合）
cargo set-version X.Y.Z --workspace

# cargo-edit が入っていない場合は手動で Cargo.toml を編集する
# [workspace.package]
# version = "X.Y.Z"
```

---

## 関連ドキュメント

- [CONTRIBUTING.md](../CONTRIBUTING.md) — 開発フロー全般
- [CHANGELOG.md](../CHANGELOG.md) — 本体の変更履歴
- [editors/vscode/CHANGELOG.md](../editors/vscode/CHANGELOG.md) — VS Code 拡張の変更履歴
