import type { EditorView } from '@codemirror/view'

// 非同期処理（Worker 整形・lint 修正・確認ダイアログ）の完了後に、
// 処理開始時のソースから文書が変わっていない場合だけ全文を置換する。
// 処理中の編集・文書切替・エディタ再生成があれば古い結果を捨てて false を返す。
export function applyIfUnchanged(
  view: EditorView,
  getCurrentView: () => EditorView | null,
  expectedSource: string,
  replacement: string
): boolean {
  if (getCurrentView() !== view) return false
  if (view.state.doc.toString() !== expectedSource) return false
  view.dispatch({
    changes: { from: 0, to: view.state.doc.length, insert: replacement },
  })
  return true
}
