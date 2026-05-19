---
description: 自分が作成した PR の CI 失敗を解析し、原因を特定して修正コミットを push する。引数に PR 番号を指定（省略時は現在のブランチの PR を自動検出）。
argument-hint: "[PR番号]"
allowed-tools: Bash(gh:*), Bash(git:*), Bash(cargo:*), Bash(npm:*), Read, Edit, Write, Grep, Glob
---

# /fix-pr -- PR の CI 失敗を自動修正

ユーザーの指定した PR（または現在のブランチに紐づく PR）について、CI が失敗していれば原因を解析し、修正コミットを作って push する。

## 引数

`$1` = PR 番号（省略可）。省略時は `gh pr view --json number,headRefName` で現在のブランチに紐づく PR を取得する。

## 手順

1. **PR の特定**
   - `$1` があればその番号を使う。
   - なければ `gh pr view --json number,headRefName,state,statusCheckRollup,baseRefName` で現在ブランチの PR を取得。
   - PR が存在しない、または既に merge / closed なら中断して報告。

2. **CI 状態の取得**
   - `gh pr checks <PR番号> --json name,state,link,workflow,bucket` で全チェックの状態を取得。
   - すべて pass / pending なら「修正不要」と報告して終了。
   - 失敗があれば各 job のログを `gh run view <run_id> --log-failed` で取得（または `gh api` でフォールバック）。

3. **失敗原因の分類**
   - `cargo fmt` 違反 → `cargo fmt --all` をローカルで実行
   - `cargo clippy` warning/error → 該当ファイルを Read して修正
   - `cargo test` 失敗 → テスト名から該当箇所を特定し、実装またはテストを修正
   - `npm run lint`（WebUI）失敗 → ESLint エラーを修正
   - `npm run build`（WebUI / WASM）失敗 → コンパイルエラーを Read して修正
   - 上記以外（環境問題、flaky test、CI 設定）→ **自動修正せず**、原因と推奨対応を報告して終了

4. **ローカル検証**
   - 修正後、以下を全て pass させる:
     - `cargo fmt --all -- --check`
     - `cargo clippy --workspace --all-targets -- -D warnings`
     - `cargo test --workspace`
     - WebUI を触ったなら `cd apps/webui && npm run lint && npm run build`
   - 失敗が残ればそれも修正する。最大 3 周まで。それでも直らなければ報告して終了。

5. **コミット & push**
   - 修正内容をひとまとめにコミット。メッセージは Conventional Commits に従う:
     - 例: `fix(ci): cargo clippy の `redundant_clone` 警告を解消`
     - 例: `fix(parser): 月日リテラルのテスト失敗を修正`
   - **`--no-verify` は使わない。**
   - 現在ブランチに push（`git push`）。force push は禁止。

6. **結果報告**
   - 修正したファイルと commit hash を列挙
   - 残った失敗（あれば）を列挙
   - 次に取れる選択肢を 1〜2 行で提案

## 安全制約

- **PR の作者が自分（keroway）でない場合は中断**。`gh pr view --json author` で確認。
- **CI が「unrelated」(infra / flaky) と判断したら勝手に直さない。** 必ず報告のみで終了。
- **force push 禁止。** ローカルに rebase 中のコミットがあれば中断して報告。
- **base ブランチが main の場合、main への直接コミットは禁止。** PR ブランチであることを確認してから push する。

## 出力フォーマット

```
## 対象 PR
#<番号> "<タイトル>" (branch: <名前>)

## 検出した失敗
- <job名>: <一行サマリ>
- ...

## 修正内容
- <ファイル>: <何を直したか>

## 検証結果
- cargo fmt: OK
- cargo clippy: OK
- cargo test: OK
- (該当時) npm run lint: OK

## コミット
- <hash> <commit message>

## 残課題
- (なし / 列挙)
```

## 引数の使い方

`$ARGUMENTS` をそのまま PR 番号として扱う。空なら現在ブランチから自動検出。
