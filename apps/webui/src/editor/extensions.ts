import { EditorView, Decoration, ViewPlugin, type ViewUpdate, type DecorationSet } from '@codemirror/view'
import { StateEffect, StateField } from '@codemirror/state'
import { linter, type Diagnostic as CmDiagnostic } from '@codemirror/lint'
import type { Diagnostic } from '../wasmLoader'

// ─── CodeMirror line highlight effect (プレビュー→エディタ方向) ──────────────────

/** 一時ハイライトをセットする StateEffect。行番号（1-based）を受け取る。 */
export const setLineHighlight = StateEffect.define<number | null>()

/** アクティブなハイライト行を保持する StateField。 */
export const lineHighlightField = StateField.define<DecorationSet>({
  create() { return Decoration.none },
  update(deco, tr) {
    deco = deco.map(tr.changes)
    for (const effect of tr.effects) {
      if (effect.is(setLineHighlight)) {
        if (effect.value === null) {
          deco = Decoration.none
        } else {
          try {
            const line = tr.state.doc.line(effect.value)
            deco = Decoration.set([
              Decoration.line({ class: 'cm-jump-highlight' }).range(line.from),
            ])
          } catch {
            deco = Decoration.none
          }
        }
      }
    }
    return deco
  },
  provide: (f) => EditorView.decorations.from(f),
})

/**
 * CodeMirror extension: カーソル行を外部コールバックへ通知する。
 * debounce 16ms (次の animation frame 相当) でパフォーマンスを確保する。
 */
export function makeCursorLineExtension(onCursorLine: (line: number) => void) {
  let debounceId: ReturnType<typeof setTimeout> | null = null
  return ViewPlugin.fromClass(
    class {
      update(update: ViewUpdate) {
        if (!update.selectionSet && !update.docChanged) return
        if (debounceId !== null) clearTimeout(debounceId)
        debounceId = setTimeout(() => {
          debounceId = null
          const pos = update.state.selection.main.head
          try {
            const line = update.state.doc.lineAt(pos).number
            onCursorLine(line)
          } catch {
            // ignore out-of-range
          }
        }, 16)
      }
    }
  )
}

/**
 * CodeMirror linter: 渡された ref から最新の WASM Diagnostic[] を読み取り、
 * 行全体（line.from → line.to）にハイライトを描画する。
 * checkSource の二重実行を避けるため、ソースを参照せず ref に依存する。
 */
export function makeTdslLinter(diagnosticsRef: { current: Diagnostic[] }) {
  return linter(
    (view): CmDiagnostic[] => {
      const doc = view.state.doc
      const out: CmDiagnostic[] = []
      for (const d of diagnosticsRef.current) {
        if (d.line < 1 || d.line > doc.lines) continue
        const line = doc.line(d.line)
        out.push({
          from: line.from,
          to: line.to,
          severity: d.severity,
          message: d.message,
        })
      }
      return out
    },
    { delay: 0 },
  )
}
