# Contributing to Timeline DSL

Timeline DSL へのコントリビューションを歓迎します。このガイドでは開発環境のセットアップから PR 送付までの手順を説明します。

---

## 開発環境のセットアップ

### 必要なもの

- **Rust** — リポジトリ直下の [`rust-toolchain.toml`](./rust-toolchain.toml) で固定（現在 1.94）。[rustup](https://rustup.rs/) を入れておけば、リポジトリで `cargo` を実行した時に自動で対応バージョンがインストールされる
- **cargo** — Rust に同梱

```bash
# rustup のインストール（未インストールの場合）
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh

# バージョン確認（rust-toolchain.toml のチャネルが使われる）
rustc --version
cargo --version
```

> **Rust バージョン更新の方針:** ローカル・CI ともに `rust-toolchain.toml` を単一の真実源とし、CI workflow の `dtolnay/rust-toolchain@<バージョン>` も同じバージョンに揃える。保守寄りに「最新 stable から 1 マイナー前」を目安に更新する。

### リポジトリのクローン

```bash
git clone https://github.com/keroway/timeline-dsl.git
cd timeline-dsl
```

---

## ビルド方法

```bash
# ワークスペース全体をビルド
cargo build --workspace

# リリースビルド
cargo build --workspace --release
```

---

## テスト方法

### ユニットテスト・統合テスト

```bash
# ワークスペース全体のテストを実行
cargo test --workspace
```

### E2E スモークテスト

```bash
# E2E スモークテストを実行（CLI が正常動作するか確認）
bash scripts/e2e-smoke.sh
```

---

## PR 送付前のチェックリスト

PR を送る前に以下をすべて通過させてください。

- [ ] `cargo test --workspace` が通ること
- [ ] `cargo clippy --workspace` でエラーがないこと（警告も可能な限り解消する）
- [ ] `cargo fmt --check` が通ること（フォーマットが崩れていないこと）

```bash
# まとめて実行する場合
cargo test --workspace && cargo clippy --workspace && cargo fmt --check
```

---

## DSL 文法の変更手順

`.tdsl` ファイルの文法を変更する場合は、以下の順序でファイルを更新してください。

1. `crates/tdsl-parser/src/grammar.pest` — PEG 文法を編集
2. `crates/tdsl-parser/src/ast.rs` — 新しい AST 型を追加・変更
3. `crates/tdsl-parser/src/builder.rs` — pest 解析木 → AST 変換ロジックを実装
4. `crates/tdsl-core/src/lower.rs` — AST → IR の lowering ロジックを追加
5. 必要に応じて `crates/tdsl-core/src/ir.rs` の IR 型を更新
6. `cargo test --workspace` で全テストが通ることを確認
7. `docs/dsl-spec.md` と `docs/dsl-spec.en.md` を**両言語同時に**更新する（dsl-spec は日英ペアで管理しており、片方だけの更新は不可。EBNF・コード例は両言語で内容を一致させる）

---

## Git 設定

コミット作者メールには GitHub の noreply アドレスを使用してください。

```bash
git config user.email "4470654+keroway@users.noreply.github.com"
```

個人の gmail アドレスはコミット履歴に残さないようにしてください（`.mailmap` で表示名を正規化済み）。

---

## コミットメッセージ形式

[Conventional Commits](https://www.conventionalcommits.org/ja/v1.0.0/) に従ってください。

```
<type>: <概要>
```

### type 一覧

| type | 用途 |
|------|------|
| `feat` | 新機能の追加 |
| `fix` | バグ修正 |
| `docs` | ドキュメントのみの変更 |
| `test` | テストの追加・修正 |
| `chore` | ビルド設定・CI・依存関係の更新など |
| `refactor` | 機能変更を伴わないリファクタリング |
| `perf` | パフォーマンス改善 |

### 例

```
feat: map ブロックに filter 構文を追加
fix: 負の年（紀元前）のパースエラーを修正
docs: CONTRIBUTING.md を追加
test: tdsl-parser の統合テストを追加
chore: dependabot の設定を更新
```

---

## Issue ラベルの説明

### priority（優先度）

| ラベル | 意味 |
|--------|------|
| `priority: high` | 早急に対応が必要 |
| `priority: medium` | 通常の優先度 |
| `priority: low` | 余裕があれば対応 |

### difficulty（難易度）

| ラベル | 意味 |
|--------|------|
| `difficulty: easy` | 初めてのコントリビューターにも適している |
| `difficulty: medium` | ある程度のコードベース理解が必要 |
| `difficulty: hard` | 深い設計理解や複雑な実装が必要 |

### area（担当領域）

| ラベル | 意味 |
|--------|------|
| `area: parser` | `tdsl-parser` クレートに関する変更 |
| `area: core` | `tdsl-core` クレートに関する変更 |
| `area: cli` | `tdsl-cli` クレートに関する変更 |
| `area: wikidata` | `tdsl-wikidata` クレートに関する変更 |
| `area: docs` | ドキュメントに関する変更 |
| `area: ci` | CI/CD の設定に関する変更 |

---

## リリース時にバンプするファイル一覧

新バージョンをリリースする際は、git tag を打つ前に以下のファイルを同一 PR で更新してください。

| ファイル | 更新箇所 |
|----------|---------|
| `Cargo.toml` | `[workspace.package].version` |
| `editors/vscode/package.json` | `"version"` フィールド |
| `editors/vscode/CHANGELOG.md` | `## [Unreleased]` の直下に新バージョンセクションを追加 |
| `CHANGELOG.md`（本体） | `## [Unreleased]` の直下に新バージョンセクションを追加 |

`editors/vscode/package.json` の version と git tag が一致していない場合、VS Code Marketplace への自動 publish がブロックされます。

詳細な手順は [docs/release.md](./docs/release.md) を参照してください。

---

## 質問・相談

不明点や設計についての相談は [GitHub Issues](https://github.com/keroway/timeline-dsl/issues) へお気軽にどうぞ。
