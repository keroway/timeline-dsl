// 単一真実源は `keywords.json`。本ファイルはそれを TS から型付きで再エクスポートするだけ。
// grammar 生成（editors/vscode/scripts/gen-grammar-keywords.mjs）と
// Rust 側ミラー（crates/tdsl-lsp/src/keywords.rs のドリフト防止テスト）も
// 同じ keywords.json を参照する。キーワード追加・変更は keywords.json のみ編集する。
import keywords from "./keywords.json"

export const BLOCK_KEYWORDS: readonly string[] = keywords.BLOCK_KEYWORDS
export const ITEM_KEYWORDS: readonly string[] = keywords.ITEM_KEYWORDS
export const MISC_KEYWORDS: readonly string[] = keywords.MISC_KEYWORDS
