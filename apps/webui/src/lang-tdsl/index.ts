import { StreamLanguage, LanguageSupport } from "@codemirror/language"
import { tags } from "@lezer/highlight"

interface TdslState {
  inBlockComment: boolean
}

const BLOCK_KEYWORDS = new Set([
  "timeline", "lane", "import", "map", "template", "apply", "color_map", "policy",
])

const ITEM_KEYWORDS = new Set([
  "span", "event", "event_range",
])

const MISC_KEYWORDS = new Set([
  "as", "query", "wikidata", "unit", "range", "calendar", "kind", "order",
  "tags", "source", "label", "start", "end", "time", "id", "target_type",
  "target_lane", "merge_by_source", "overwrite_imported", "keep_manual",
  "proleptic_gregorian", "year", "dynasty", "person", "era", "title",
  "field_priority", "origin",
])

const tdslLanguage = StreamLanguage.define<TdslState>({
  name: "tdsl",
  tokenTable: {
    keyword:           tags.keyword,
    definitionKeyword: tags.definitionKeyword,
    modifier:          tags.modifier,
    string:            tags.string,
    number:            tags.number,
    atom:              tags.atom,
    lineComment:       tags.lineComment,
    blockComment:      tags.blockComment,
    punctuation:       tags.punctuation,
    special:           tags.special(tags.variableName),
  },
  startState(): TdslState {
    return { inBlockComment: false }
  },
  copyState(state): TdslState {
    return { inBlockComment: state.inBlockComment }
  },
  token(stream, state): string | null {
    // ブロックコメント継続
    if (state.inBlockComment) {
      if (stream.match("*/")) {
        state.inBlockComment = false
        return "blockComment"
      }
      stream.next()
      return "blockComment"
    }

    if (stream.eatSpace()) return null

    // 行コメント
    if (stream.match("//")) {
      stream.skipToEnd()
      return "lineComment"
    }

    // ブロックコメント開始
    if (stream.match("/*")) {
      state.inBlockComment = true
      while (!stream.eol()) {
        if (stream.match("*/")) { state.inBlockComment = false; break }
        stream.next()
      }
      return "blockComment"
    }

    // 文字列リテラル
    if (stream.peek() === '"') {
      stream.next()
      while (!stream.eol()) {
        const c = stream.next()
        if (c === '\\') { stream.next(); continue }
        if (c === '"') break
      }
      return "string"
    }

    // claim(...).xxx 式（関数呼び出し＋プロパティアクセス）
    if (stream.match(/^claim\s*\(/)) {
      let depth = 1
      while (!stream.eol() && depth > 0) {
        const c = stream.next()
        if (c === '(') depth++
        if (c === ')') depth--
      }
      stream.match(/^(\.\w+)*/)
      return "special"
    }

    // label@lang 式
    if (stream.match(/^label@[a-z]{2,3}/)) {
      return "special"
    }

    // wd:QXX（Wikidata エンティティ参照）
    if (stream.match(/^wd:[A-Z][0-9]+/)) {
      return "atom"
    }

    // 数値（負の年含む）— 識別子の前にチェック
    if (stream.match(/^-?\d+(\.\d+)?/)) {
      return "number"
    }

    // 識別子・キーワード
    const wordMatch = stream.match(/^[a-zA-Z_][a-zA-Z0-9_]*/)
    if (wordMatch) {
      const word = Array.isArray(wordMatch) ? wordMatch[0] : ""
      if (!word) return null
      // QID / PID: 識別子パターンに引っかかった場合のフォールバック
      if (/^Q\d+$/.test(word) || /^P\d+$/.test(word)) return "atom"
      if (BLOCK_KEYWORDS.has(word)) return "keyword"
      if (ITEM_KEYWORDS.has(word)) return "definitionKeyword"
      if (MISC_KEYWORDS.has(word)) return "modifier"
      return null
    }

    // punctuation
    const ch = stream.peek()
    if (ch && "{}[];,".includes(ch)) {
      stream.next()
      return "punctuation"
    }
    if (stream.match("..")) return "punctuation"
    if (ch === '.') { stream.next(); return "punctuation" }

    stream.next()
    return null
  },
})

export function tdsl(): LanguageSupport {
  return new LanguageSupport(tdslLanguage)
}
