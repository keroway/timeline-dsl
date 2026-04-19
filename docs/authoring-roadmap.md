# Authoring Roadmap

`.tdsl` 作成体験を改善するための実装順ロードマップ。

## 実装順（優先度順）

1. [x] `tdsl scaffold wikidata` を追加する  
   Issue: https://github.com/keroway/timeline-dsl/issues/5

2. [x] `import query` を実装して仕様と一致させる  
   Issue: https://github.com/keroway/timeline-dsl/issues/8

3. [x] `tdsl init` + `tdsl import-csv` を追加する（手作業フロー強化）  
   Issue: https://github.com/keroway/timeline-dsl/issues/6

4. [x] `tdsl lint --fix` を追加する（品質チェック自動化）  
   Issue: https://github.com/keroway/timeline-dsl/issues/7

5. [x] `import policy` を lowering に実装する  
   PR: https://github.com/keroway/timeline-dsl/pull/18

6. [x] Wikipedia URL から QID を解決するコマンドを追加する  
   PR: https://github.com/keroway/timeline-dsl/pull/19

7. [ ] Wikidata 取得キャッシュ（TTL/オフライン連携）を追加する  
   Issue: https://github.com/keroway/timeline-dsl/issues/28

## その他の改善 Issue

- #29 テスト一時ファイルの flaky test 修正
- #30 `validate.rs` に `start > end` チェックを追加
- #31 SPARQL クエリ結果の QID 抽出改善

## Epic

- https://github.com/keroway/timeline-dsl/issues/10 （完了・クローズ済み）

## 方針

- 基本運用は `1 issue = 1 PR`
- 実装は `main` から issue 専用ブランチを切って進める
- 統合確認が必要な場合は `feature/authoring-all` を利用する
