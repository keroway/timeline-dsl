import { snippetCompletion, type CompletionContext, type CompletionResult } from '@codemirror/autocomplete'

// ─── TDSL keyword completions & snippets ─────────────────────────────────────

const TDSL_SNIPPETS = [
  snippetCompletion('timeline "${1:タイトル}" {\n  unit year;\n  range ${2:1900}..${3:2000};\n\n  ${0}\n}', {
    label: 'timeline', detail: '年表ブロック', type: 'keyword', boost: 10,
  }),
  snippetCompletion('lane "${1:レーン名}" as ${2:id} {\n  kind ${3:dynasty};\n  order ${4:10};\n}', {
    label: 'lane', detail: 'レーン定義', type: 'keyword', boost: 9,
  }),
  snippetCompletion('span ${1:lane_id} ${2:1900}..${3:1950} "${4:ラベル}" {};', {
    label: 'span', detail: 'スパン', type: 'keyword', boost: 8,
  }),
  snippetCompletion('event ${1:lane_id} ${2:1900} "${3:ラベル}" {};', {
    label: 'event', detail: 'イベント', type: 'keyword', boost: 8,
  }),
  snippetCompletion('event_range ${1:lane_id} ${2:1900}..${3:1950} "${4:ラベル}" {};', {
    label: 'event_range', detail: 'イベント範囲', type: 'keyword', boost: 7,
  }),
  snippetCompletion('import wikidata as ${1:wd} {\n  entity Q${2:12345} as ${3:alias};\n}', {
    label: 'import', detail: 'Wikidataインポート', type: 'keyword', boost: 7,
  }),
  snippetCompletion('map ${1:wd}.${2:alias} to span {\n  lane ${3:lane_id};\n  start claim(P${4:571}).year;\n  end claim(P${5:576}).year;\n  label label@ja ?? label@en;\n}', {
    label: 'map', detail: 'マッピング', type: 'keyword', boost: 6,
  }),
  snippetCompletion('query "${1:SPARQL}" as ${2:alias};', {
    label: 'query', detail: 'SPARQLクエリ', type: 'keyword', boost: 5,
  }),
  snippetCompletion('color_map {\n  ${1:tag}: "${2:#4682B4}";\n}', {
    label: 'color_map', detail: 'タグ→色マッピング', type: 'keyword', boost: 5,
  }),
]

const STATIC_KEYWORDS = [
  { label: 'unit', type: 'keyword' as const },
  { label: 'range', type: 'keyword' as const },
  { label: 'calendar', type: 'keyword' as const },
  { label: 'title', type: 'keyword' as const },
  { label: 'kind', type: 'keyword' as const },
  { label: 'order', type: 'keyword' as const },
  { label: 'year', type: 'keyword' as const },
  { label: 'policy', type: 'keyword' as const },
  { label: 'label', type: 'property' as const },
  { label: 'start', type: 'property' as const },
  { label: 'end', type: 'property' as const },
  { label: 'time', type: 'property' as const },
  { label: 'source', type: 'property' as const },
  { label: 'tags', type: 'property' as const },
  { label: 'id', type: 'property' as const },
  { label: 'origin', type: 'property' as const },
  { label: 'filter', type: 'property' as const },
  { label: 'lane', type: 'property' as const },
]

export function makeTdslCompletionSource(getSource: () => string) {
  return function tdslCompletions(context: CompletionContext): CompletionResult | null {
    const src = getSource()
    // entity / query エイリアスは map ブロックで `wd.<alias>` の形（import 元.別名）で
    // しか参照されない。ドットを含むトークンでは**ドット以降だけ**を補完対象にし、
    // `from` をドットの直後に置くことで `wd.` プレフィックスを保持する
    // （`from` を語頭に置くと候補挿入時に `wd.` ごと置換されて消えてしまう）。
    // TDSL ident はハイフンを含みうる（grammar.pest: [A-Za-z_][\w-]*）。
    const dotted = context.matchBefore(/[\w-]+\.[\w-]*/)
    if (dotted) {
      const entityAliases = [...src.matchAll(/\bentity\s+Q\d+\s+as\s+([A-Za-z_][\w-]*)/g)].map((m) => ({
        label: m[1], type: 'variable' as const, detail: 'entity alias',
      }))
      const queryAliases = [...src.matchAll(/\bquery\s+"[^"]*"\s+as\s+([A-Za-z_][\w-]*)/g)].map((m) => ({
        label: m[1], type: 'variable' as const, detail: 'query alias',
      }))
      return {
        from: dotted.from + dotted.text.indexOf('.') + 1,
        options: [...entityAliases, ...queryAliases],
      }
    }

    // ドット無しトークン: スニペット・キーワード・lane id・import 元エイリアス
    // （いずれも単独で参照される）。実文法は `import wikidata as wd { … }`。
    // ident 先頭は英字/アンダースコア限定（数値リテラル `-206` 等を拾わない）。
    const word = context.matchBefore(/[A-Za-z_][\w-]*/)
    if (!word || (word.from === word.to && !context.explicit)) return null
    const laneIds = [...src.matchAll(/\blane\s+"[^"]*"\s+as\s+([A-Za-z_][\w-]*)/g)].map((m) => ({
      label: m[1], type: 'variable' as const, detail: 'lane id',
    }))
    const importSources = [...src.matchAll(/\bimport\s+[A-Za-z_][\w-]*\s+as\s+([A-Za-z_][\w-]*)\s*\{/g)].map((m) => ({
      label: m[1], type: 'variable' as const, detail: 'import source',
    }))
    return {
      from: word.from,
      options: [...TDSL_SNIPPETS, ...STATIC_KEYWORDS, ...laneIds, ...importSources],
    }
  }
}
