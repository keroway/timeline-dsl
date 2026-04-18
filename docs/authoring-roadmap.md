# Authoring Roadmap

`.tdsl` 作成体験を改善するための実装順ロードマップ。

## 実装順（優先度順）

1. `tdsl scaffold wikidata` を追加する  
   Issue: https://github.com/keroway/timeline-dsl/issues/5

2. `import query` を実装して仕様と一致させる  
   Issue: https://github.com/keroway/timeline-dsl/issues/8

3. `tdsl init` + `tdsl import-csv` を追加する（手作業フロー強化）  
   Issue: https://github.com/keroway/timeline-dsl/issues/6

4. `tdsl lint --fix` を追加する（品質チェック自動化）  
   Issue: https://github.com/keroway/timeline-dsl/issues/7

5. `import policy` を lowering に実装する  
   Issue: 未作成

6. Wikipedia URL から QID を解決するコマンドを追加する  
   Issue: 未作成

7. Wikidata 取得キャッシュ（TTL/オフライン連携）を追加する  
   Issue: 未作成

## Epic

- https://github.com/keroway/timeline-dsl/issues/10

## 方針

- 基本運用は `1 issue = 1 PR`
- 実装は `main` から issue 専用ブランチを切って進める
- 統合確認が必要な場合は `feature/authoring-all` を利用する
