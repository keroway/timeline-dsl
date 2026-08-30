# Domain Docs

How the engineering skills should consume this repo's domain documentation when exploring the codebase.

This repo is **single-context**（Rust の cargo workspace で複数クレートを持つが、
ドメインは「年表 DSL」ひとつ。`CONTEXT-MAP.md` は不要）。

## Before exploring, read these

- **`CLAUDE.md`** の「アーキテクチャ」節: クレート構成と**依存方向**。
  `cargo metadata` が正で、図はその写し。どのクレートを触るかはここで決まる。
- **[`docs/adr/`](../adr/)**: 触れる領域の ADR を読む。秒精度・タイムゾーン
  (0003 / 0006 / 0007)、PDF/ページ分割 (0002 / 0004 / 0005)、WASM 配布と
  Obsidian 連携 (0001) のように主題ごとにまとまっている。
- **[`docs/dsl-spec.md`](../dsl-spec.md)**: DSL の言語仕様。用語の実質的な正典。
  英語版は `dsl-spec.en.md`。
- **`CONTEXT.md`**: **まだ存在しない。** 無い場合は**黙って先に進む**こと。
  不在を報告したり、先回りして作成を提案したりしない。`/domain-modeling` スキル
  （`/grill-with-docs` と `/improve-codebase-architecture` から到達する）が、
  用語が実際に確定した時点で遅延生成する。

## Use the spec's vocabulary

When your output names a domain concept (in an issue title, a refactor proposal, a
hypothesis, a test name), use the term as `docs/dsl-spec.md` defines it. AST / IR の
型名は `crates/tdsl-parser/src/ast.rs` と `crates/tdsl-core/src/ir.rs` が正典で、
仕様書の用語と対応している。両者がずれていたら、どちらを直すか必ず判断すること。

`CONTEXT.md` を作るなら、`docs/dsl-spec.md` の用語を複製せず**参照**にすること。
文法の正典が2箇所に増えると `grammar.pest` との三重管理になる。

## File structure

```
/
├── docs/
│   ├── dsl-spec.md / dsl-spec.en.md   ← 言語仕様（用語の正典）
│   ├── agents/                        ← this file, issue-tracker.md, triage-labels.md
│   ├── adr/
│   │   └── 0001-….md 〜 0007-….md
│   ├── architecture.md / cli-spec.md / error-catalog.md / …
│   └── reviews/
└── crates/
    ├── tdsl-parser/ tdsl-wikidata/    ← 基底（他クレートに依存しない）
    ├── tdsl-core/ tdsl-render/
    └── tdsl-lsp/ tdsl-cli/ tdsl-wasm/
```

## Flag ADR conflicts

If your output contradicts an existing ADR, surface it explicitly rather than silently
overriding:

> _ADR-0007（IANA タイムゾーン採用）と衝突するが、…の理由で再検討の価値がある_

DSL 文法を変える提案では、`CLAUDE.md` の「DSL文法の変更手順」7ステップ
（**シンタックスハイライトのキーワード更新を含む**）を満たしているかを必ず確認する。
