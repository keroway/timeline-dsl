import { hoverTooltip, type Tooltip } from "@codemirror/view"
import type { Extension } from "@codemirror/state"

// ─── Keyword documentation ───────────────────────────────────────────────────
// Short, one-line descriptions surfaced when hovering a TDSL keyword. Mirrors
// the keyword sets in keywords.ts plus a few connective/property keywords that
// are not in the highlight lists but are still worth explaining on hover.
const KEYWORD_DOCS: Record<string, string> = {
  // Block keywords
  timeline: "年表全体を定義するトップレベルブロック。unit / range / calendar などのメタと各アイテムを内包する。",
  lane: "アイテムを並べる横帯（レーン）を定義する。`lane \"ラベル\" as id { kind …; order …; }`。",
  group: "複数の lane をまとめるグループ。`group \"ラベル\" { lane … }`。",
  import: "外部データソース（Wikidata 等）からの取り込みブロック。`import wikidata as wd { … }`。",
  map: "インポートしたエンティティを span / event / event_range に変換する。`map wd.alias to span { … }`。",
  template: "共通フォーマットを定義して apply で再利用するテンプレート。",
  apply: "template を適用してアイテムを生成する。",
  color_map: "タグ→色のマッピングを宣言的に定義するブロック。",
  policy: "再インポート時の挙動（merge_by_source / overwrite_imported / keep_manual）やフィールド優先度を指定する。",
  // Item keywords
  span: "開始〜終了の期間を持つアイテム（王朝・時代など）。",
  event: "単一時点のイベント。`at` で時点を指定する。",
  event_range: "範囲を持つイベント（start / end）。span と異なりイベント系の見た目になる。",
  // Misc / property / connective keywords
  as: "識別子に別名（エイリアス）を割り当てるキーワード。",
  to: "map の変換先アイテム種別を指定する（`map … to span`）。",
  at: "event の時点を指定するプロパティ。",
  query: "SPARQL クエリで複数エンティティを一括インポートする。`query \"…\" as alias;`。",
  entity: "単一の Wikidata エンティティをインポートする。`entity Q… as alias;`。",
  wikidata: "インポート元データソース名。Wikidata を指す。",
  unit: "時間軸の単位（year / month / day）。",
  range: "年表の表示範囲。`range: 開始 to 終了`。",
  calendar: "暦法。既定は proleptic_gregorian。",
  kind: "lane の種別（dynasty / person / event / era / custom など）。色や見た目に影響する。",
  order: "lane の表示順を決める整数。小さいほど上に並ぶ。",
  tags: "アイテムに付与するタグ配列。color_map と連動して着色される。",
  source: "アイテムの出典。インポート品は `wd:<QID>` が自動付与される。",
  label: "アイテムやレーンの表示名。`label@ja ?? label@en` のフォールバックも可。",
  start: "span / event_range の開始時点。",
  end: "span / event_range の終了時点。",
  time: "map で時点を計算する式を指定するプロパティ。",
  lang: "インポート時に取得する言語の優先順位（`lang: ja, en`）。",
  target_type: "map の変換先種別（span / event / event_range）。",
  id: "識別子。",
  merge_by_source: "再インポートポリシー: 同一 source のアイテムをマージする。",
  overwrite_imported: "再インポートポリシー: インポート品を上書きする。",
  keep_manual: "再インポートポリシー: 手動定義を温存する。",
  proleptic_gregorian: "先発グレゴリオ暦。既定の calendar 値。",
  year: "単位 / プロパティ値としての「年」。`claim(P569).year` など。",
  dynasty: "lane の kind: 王朝。",
  person: "lane の kind: 人物。",
  era: "lane の kind: 時代区分。",
  title: "メタの年表タイトル。",
  field_priority: "フィールド別インポート優先度を定義する policy ブロック。",
  origin: "アイテムの由来（manual / imported）を表す IR フィールド。",
}

// ─── Source analysis (regex-based, mirrors makeTdslCompletionSource) ──────────

interface LaneInfo { label: string; kind?: string; order?: string }
interface EntityInfo { qid: string; importAlias?: string }
interface QueryInfo { importAlias?: string }
interface ImportSourceInfo { sourceName: string }

interface SourceModel {
  lanes: Map<string, LaneInfo>
  entities: Map<string, EntityInfo>
  queries: Map<string, QueryInfo>
  importSources: Map<string, ImportSourceInfo>
}

function analyzeSource(src: string): SourceModel {
  const lanes = new Map<string, LaneInfo>()
  // lane "label" as id { kind X; order N; }
  for (const m of src.matchAll(/\blane\s+"([^"]*)"\s+as\s+(\w+)\s*\{([^}]*)\}/g)) {
    const [, label, id, body] = m
    const kind = body.match(/\bkind\s+(\w+)/)?.[1]
    const order = body.match(/\border\s+(-?\d+)/)?.[1]
    lanes.set(id, { label, kind, order })
  }

  const importSources = new Map<string, ImportSourceInfo>()
  // import wikidata as wd {
  for (const m of src.matchAll(/\bimport\s+(\w+)\s+as\s+(\w+)\s*\{/g)) {
    const [, sourceName, alias] = m
    importSources.set(alias, { sourceName })
  }

  const entities = new Map<string, EntityInfo>()
  // entity Q123 as alias;
  for (const m of src.matchAll(/\bentity\s+(Q\d+)\s+as\s+(\w+)/g)) {
    const [, qid, alias] = m
    entities.set(alias, { qid })
  }

  const queries = new Map<string, QueryInfo>()
  // query "..." as alias;
  for (const m of src.matchAll(/\bquery\s+"[^"]*"\s+as\s+(\w+)/g)) {
    queries.set(m[1], {})
  }

  return { lanes, entities, queries, importSources }
}

// ─── Tooltip DOM ──────────────────────────────────────────────────────────────

function makeTooltipDom(kind: string, title: string, body: string): HTMLElement {
  const dom = document.createElement("div")
  dom.className = "tdsl-hover-tooltip"

  const head = document.createElement("div")
  head.className = "tdsl-hover-head"

  const tag = document.createElement("span")
  tag.className = "tdsl-hover-kind"
  tag.textContent = kind
  head.appendChild(tag)

  const name = document.createElement("span")
  name.className = "tdsl-hover-title"
  name.textContent = title
  head.appendChild(name)

  dom.appendChild(head)

  const desc = document.createElement("div")
  desc.className = "tdsl-hover-body"
  desc.textContent = body
  dom.appendChild(desc)

  return dom
}

// ─── Hover extension ──────────────────────────────────────────────────────────

/**
 * CodeMirror hover tooltip for TDSL. Surfaces:
 * - lane id → ラベル / kind / order
 * - import エイリアス・entity・query エイリアス → インポート元
 * - キーワード → 簡潔な説明
 *
 * Analysis is purely client-side (regex over the current source), mirroring the
 * completion source. No WASM round-trip is needed. Mobile (touch) is not
 * targeted — hover requires a pointer, which is acceptable per the issue.
 */
export function tdslHover(getSource: () => string): Extension {
  return hoverTooltip((view, pos): Tooltip | null => {
    const { text, from } = view.state.doc.lineAt(pos)
    const rel = pos - from
    // Identify the identifier token under the cursor.
    let start = rel
    let end = rel
    const isWord = (c: string) => /[A-Za-z0-9_]/.test(c)
    while (start > 0 && isWord(text[start - 1])) start--
    while (end < text.length && isWord(text[end])) end++
    if (start === end) return null
    const word = text.slice(start, end)
    if (!/^[A-Za-z_]\w*$/.test(word)) return null

    const tipFrom = from + start
    const tipTo = from + end
    const model = analyzeSource(getSource())

    // User-defined identifiers take precedence over keyword docs.
    const lane = model.lanes.get(word)
    if (lane) {
      const meta = [
        lane.kind ? `kind: ${lane.kind}` : null,
        lane.order ? `order: ${lane.order}` : null,
      ].filter(Boolean).join(" / ")
      const body = meta ? `「${lane.label}」（${meta}）` : `「${lane.label}」`
      return mk(tipFrom, tipTo, makeTooltipDom("lane", word, body))
    }

    const entity = model.entities.get(word)
    if (entity) {
      const body = `Wikidata エンティティ ${entity.qid} をインポート`
      return mk(tipFrom, tipTo, makeTooltipDom("entity", word, body))
    }

    const query = model.queries.get(word)
    if (query) {
      return mk(tipFrom, tipTo, makeTooltipDom("query", word, "SPARQL クエリの結果セット"))
    }

    const imp = model.importSources.get(word)
    if (imp) {
      return mk(tipFrom, tipTo, makeTooltipDom("import", word, `インポート元: ${imp.sourceName}`))
    }

    const doc = KEYWORD_DOCS[word]
    if (doc) {
      return mk(tipFrom, tipTo, makeTooltipDom("keyword", word, doc))
    }

    return null
  }, { hoverTime: 300 })
}

function mk(from: number, end: number, dom: HTMLElement): Tooltip {
  return { pos: from, end, above: true, create: () => ({ dom }) }
}
