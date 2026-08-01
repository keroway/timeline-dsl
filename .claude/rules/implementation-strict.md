# 実装方針ルール（strict）

このドキュメントは `timeline-dsl` における**実装の厳密化ルール**です。
`CLAUDE.md`（`AGENTS.md` は `CLAUDE.md` への symlink）と併せて遵守すること。曖昧な状況では本ファイルを優先。

---

## 1. 基本原則

| # | 原則 | 言い換え |
|---|------|---------|
| 1 | Strictness over permissiveness | 寛容より厳格を選ぶ。未知トークンや参照は黙って通さない。 |
| 2 | Explicit error over silent fallback | フォールバックで誤魔化さない。エラーで止める。 |
| 3 | IR is the single source of truth | Parser/Renderer は IR を介してのみ通信。 |
| 4 | Smaller MVP over larger speculation | 先取り実装をしない。今要らないものは入れない。 |
| 5 | Spec / docs / tests / impl は同時更新 | どれか一つでも欠けたら未完了。 |

---

## 2. 必ず止めるべき変更パターン（NO-GO）

実装前に以下に該当する変更案ならば、**着手前に必ずユーザーに確認**する。

1. **MVP で deferred とされた機能の実装**（`CLAUDE.md`「未実装 / 意図的に対応しない機能」を参照）
   - `map source` の手動指定
   - 古い `query "..." as alias` の制約復活（既に実装済の場合は別）
   - 詳細な Wikidata qualifier（P39 + P580/P582 等）
2. **IR の破壊的変更**（既存 `.tdsl` / IR JSON コンシューマが壊れる）
3. **クレート間の循環参照**を導入する変更
4. **silent fallback** の導入（unknown lane に対する自動マッチ、未解決 import の無視等）
5. **`unwrap()` / `expect()` の本番コードへの追加**（テスト除く）
6. **`#[allow(clippy::*)]` / `#[allow(dead_code)]` の安易な追加**
7. **テストなしの新機能追加**
8. **docs（dsl-spec / README / CHANGELOG）を更新しない仕様変更**

---

## 3. 実装着手前のチェックリスト

新機能・修正に着手する前に、以下を埋めてから書き始める（脳内でも可）。

- [ ] **要件の 1 行要約**: 何を達成すれば「完了」か。
- [ ] **影響クレート**: tdsl-parser / tdsl-core / tdsl-wikidata / tdsl-render / tdsl-wasm / tdsl-cli / apps/webui / editors/vscode のどれを触るか。
- [ ] **依存方向の確認**: 触るクレートが上位クレート（cli, render）に依存していないか。
- [ ] **文法変更を含むか**: 含むなら `grammar.pest` → `ast.rs` → `builder.rs` → `lower.rs` → `ir.rs` → `dsl-spec.md` → `keywords.json` の更新順序を守る。
- [ ] **テスト方針**: parser のユニットテスト / lowering のテスト / E2E（`scripts/e2e-smoke.sh`）のどこに置くか。
- [ ] **docs 更新対象**: README.md / README.ja.md / docs/dsl-spec.md / docs/cli-spec.md / CHANGELOG.md のどれか。
- [ ] **後方互換**: 既存の `.tdsl` ファイル（`examples/` 配下）がそのまま通るか確認したか。

---

## 4. コードレベルのルール

### 4.1 エラー処理

- ライブラリ層（parser / core / wikidata / render）は `thiserror` ベースのエラー型を返す。
- CLI 層では `miette` でユーザー向けに整形する。
- `anyhow::Error` を crate の公開 API に出さない。CLI 内部 only。
- `.unwrap()` `.expect()` `.panic!()` は本番コード（`crates/*/src/`）では原則禁止。
  - テストコード（`#[cfg(test)]`）と CLI 起動直後の致命的初期化失敗のみ許容。
  - 例外的に使う場合は理由を 1 行コメントで残す。

### 4.2 型と enum

- マジック文字列を使わない。`target_type` のような値域は enum で表現する。
- `Option<T>` の `unwrap()` ではなく `match` / `if let` で網羅する。
- `Result<T, E>` を捨てない（`let _ = ...` は禁止）。

### 4.3 async / I/O

- Wikidata クライアントは `WikidataClient` trait 経由でのみ呼ぶ。テストではモック実装を渡す。
- 新規 HTTP I/O を入れる場合は `tdsl-wikidata` の retry / cache 設計と整合させる。
- `tokio::spawn` を `tdsl-cli` 以外で使わない。

### 4.4 シリアライズ

- IR 型は `serde::Serialize + Deserialize` を derive する。
- `skip_serializing_if = "Option::is_none"` を **新しい optional フィールド** に必ず付ける（JSON IR の互換性のため）。
- JSON のキー命名は既存パターン（snake_case）に従う。

### 4.5 Lane / item ルール

- lane ID の決定性を保つ。日本語ラベルのみで空 slug になる場合は `lane_N` にフォールバック。
- imported item は必ず `source = wd:<QID>` / `origin = "imported"`。
- 静的アイテムの `source` も `sources[]` に登録する。

---

## 5. テストの最低ライン

| 変更種別 | 必須テスト |
|----------|-----------|
| 文法追加 | parser のユニットテスト（成功 + 失敗ケース） |
| AST 変更 | builder の単体テスト |
| lowering 変更 | core のロジックテスト + IR スナップショット |
| Wikidata 連携変更 | モック client を使ったテスト |
| CLI 追加 | `scripts/e2e-smoke.sh` への追加または `tests/cli_integration_test.rs` |
| Renderer 変更 | HTML/SVG 出力の正規表現 or DOM スナップショット |

ネットワーク必須テストは増やさない。CI は offline 前提。

---

## 6. PR 提出前ゲート

PR を出す前に必ず通すこと。

```bash
cargo fmt --all -- --check
cargo clippy --workspace --all-targets -- -D warnings
cargo test --workspace
bash scripts/e2e-smoke.sh
# WebUI を触っていれば
( cd apps/webui && npm run lint && npm run build )
```

Stop hook（`.claude/hooks/post-stop-check.sh`）が自動でかかるが、CI が `clippy -D warnings` を強制している前提で**人間側でも一度は手動実行する**こと。

---

## 7. コミット & PR

- Conventional Commits（CONTRIBUTING.md）に従う。`feat: / fix: / docs: / chore: / refactor: / test: / perf:`。
- 1 PR 1 目的。複数の独立変更を 1 PR に混ぜない。
- PR 本文には: **目的 / 変更点 / 検証手順 / スコープ外** を必ず含める。
- CI が落ちたら `--no-verify` で誤魔化さず原因を直す。`/fix-pr` を使うのも可。

---

## 8. 「迷ったら」フローチャート

```
                    ┌─────────────────────────┐
                    │  実装方針に迷いがある    │
                    └────────────┬────────────┘
                                 │
                ┌────────────────┼────────────────┐
                │                                 │
        ┌───────▼────────┐               ┌────────▼─────────┐
        │ §2 NO-GO に    │  Yes          │ MVP スコープに   │
        │ 該当するか?   ├──────────────▶│ 含まれているか?  │
        └───────┬────────┘               └────────┬─────────┘
                │ No                              │
                │                          ┌──────┴──────┐
                │                          │             │
                ▼                       Yes ▼           No ▼
        ┌───────────────┐         ┌──────────────┐  ┌──────────────────┐
        │ そのまま実装  │         │ 実装してOK   │  │ 着手前にユーザー │
        │ + テスト      │         │              │  │ に確認する       │
        └───────────────┘         └──────────────┘  └──────────────────┘
```

判断に迷ったら **app-dev-director サブエージェント** を呼んで設計レビューを受ける。

---

End of implementation-strict.md
