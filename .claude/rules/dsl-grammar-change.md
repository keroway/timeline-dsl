---
paths:
  - "crates/tdsl-parser/**"
  - "crates/tdsl-core/**"
  - "crates/tdsl-lsp/**"
  - "apps/webui/src/lang-tdsl/**"
  - "editors/vscode/**"
---

# DSL文法の変更手順

CLAUDE.md を補完するパス限定ルール。矛盾があれば CLAUDE.md を正とする。

1. `crates/tdsl-parser/src/grammar.pest` を編集
2. `crates/tdsl-parser/src/ast.rs` にAST型を追加/変更
3. `crates/tdsl-parser/src/builder.rs` に変換ロジックを実装
4. `crates/tdsl-core/src/lower/` にloweringロジックを追加（宣言なら `declarations.rs`、静的アイテムなら `static_items.rs`、import 解決なら `imports.rs`）
5. 必要に応じて `crates/tdsl-core/src/ir.rs` のIR型を更新
6. `cargo test --workspace` で全テスト通過を確認
7. **シンタックスハイライトのキーワードを更新すること**（手順下記参照）

## シンタックスハイライトのキーワード管理

キーワードの**単一真実源**は `apps/webui/src/lang-tdsl/keywords.json` です。
VS Code 拡張の `editors/vscode/syntaxes/tdsl.tmLanguage.json` は `npm run build` 時に自動生成されます。

- `apps/webui/src/lang-tdsl/keywords.json` — `BLOCK_KEYWORDS` / `ITEM_KEYWORDS` / `MISC_KEYWORDS` を編集する。`apps/webui/src/lang-tdsl/keywords.ts` は `keywords.json` を型付きで re-export するだけの生成物寄りファイルであり、手編集しない
- `npm run build`（または `node editors/vscode/scripts/gen-grammar-keywords.mjs`）を実行すると `tdsl.tmLanguage.json` が自動更新される

詳細は `apps/webui/README.md` の「シンタックスハイライトのキーワード管理」セクションを参照。

Rust LSP（`crates/tdsl-lsp/src/keywords.rs`）も `keywords.json` をミラーし、ドリフト防止テストで同期を保証する。

`README.md` / `README.ja.md` / `editors/vscode/README.md` の「Syntax Highlighting」節では、キーワードをハードコード列挙**しない**（列挙すると `keywords.json` 更新時にドリフトする。実例: #665）。代わりに `apps/webui/src/lang-tdsl/keywords.json` へのリンクで済ませる。
