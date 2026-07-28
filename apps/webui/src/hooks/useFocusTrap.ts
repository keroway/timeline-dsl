import { useEffect, useRef } from 'react'

type Options = {
  active: boolean
  onEscape?: () => void
}

const FOCUSABLE_SELECTOR = [
  'a[href]',
  'button:not([disabled])',
  'textarea:not([disabled])',
  'input:not([disabled])',
  'select:not([disabled])',
  '[tabindex]:not([tabindex="-1"])',
].join(',')

// モーダル内に Tab フォーカスを閉じ込め、閉じたあと呼び出し元へフォーカスを戻すフック。
// onEscape を渡すと Escape キーでも閉じられる。
export function useFocusTrap<T extends HTMLElement>({
  active,
  onEscape,
}: Options) {
  const containerRef = useRef<T | null>(null)

  useEffect(() => {
    if (!active) return
    const node = containerRef.current
    if (!node) return

    const previouslyFocused =
      document.activeElement instanceof HTMLElement
        ? document.activeElement
        : null

    const getFocusables = (): HTMLElement[] =>
      Array.from(node.querySelectorAll<HTMLElement>(FOCUSABLE_SELECTOR)).filter(
        (el) => !el.hasAttribute('aria-hidden') && el.offsetParent !== null
      )

    const initialTargets = getFocusables()
    const initial = initialTargets[0] ?? node
    initial.focus()

    const onKeyDown = (e: KeyboardEvent) => {
      if (e.key === 'Escape') {
        if (onEscape) {
          e.preventDefault()
          e.stopPropagation()
          onEscape()
        }
        return
      }
      if (e.key !== 'Tab') return
      const list = getFocusables()
      if (list.length === 0) {
        e.preventDefault()
        return
      }
      const first = list[0]
      const last = list[list.length - 1]
      const current =
        document.activeElement instanceof HTMLElement
          ? document.activeElement
          : null
      if (e.shiftKey) {
        if (current === first || !node.contains(current)) {
          e.preventDefault()
          last.focus()
        }
      } else {
        if (current === last || !node.contains(current)) {
          e.preventDefault()
          first.focus()
        }
      }
    }

    node.addEventListener('keydown', onKeyDown)
    return () => {
      node.removeEventListener('keydown', onKeyDown)
      if (previouslyFocused && document.contains(previouslyFocused)) {
        previouslyFocused.focus()
      }
    }
  }, [active, onEscape])

  return containerRef
}
