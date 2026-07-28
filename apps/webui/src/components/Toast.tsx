import {
  createContext,
  type ReactNode,
  useCallback,
  useEffect,
  useMemo,
  useRef,
  useState,
} from 'react'
import { createTranslator } from '../lib/i18n'
import { readSettings } from '../lib/settings'

export type ToastVariant = 'success' | 'error' | 'info'
export type ToastItem = { id: number; message: string; variant: ToastVariant }

export type ToastContextValue = {
  showToast: (message: string, variant?: ToastVariant) => void
}

// eslint-disable-next-line react-refresh/only-export-components
export const ToastContext = createContext<ToastContextValue | null>(null)

const MAX_TOASTS = 3
const AUTO_DISMISS_MS = 3000

export function ToastProvider({ children }: { children: ReactNode }) {
  const [toasts, setToasts] = useState<ToastItem[]>([])
  const idRef = useRef(0)
  const timersRef = useRef(new Map<number, ReturnType<typeof setTimeout>>())
  const hoveredRef = useRef(false)
  // ToastProvider sits above App in the tree (main.tsx) so it cannot receive
  // `settings.locale` as a prop; read the persisted locale directly. This is
  // re-evaluated on every render, which is sufficient since a locale change
  // in Settings triggers a re-render of this subtree via toast state updates
  // close in time to user actions.
  const translate = useMemo(() => createTranslator(readSettings().locale), [])

  const clearTimer = useCallback((id: number) => {
    const tm = timersRef.current.get(id)
    if (tm) {
      clearTimeout(tm)
      timersRef.current.delete(id)
    }
  }, [])

  const dismiss = useCallback(
    (id: number) => {
      clearTimer(id)
      setToasts((prev) => prev.filter((t) => t.id !== id))
    },
    [clearTimer]
  )

  const scheduleDismiss = useCallback(
    (id: number) => {
      clearTimer(id)
      const tm = setTimeout(() => dismiss(id), AUTO_DISMISS_MS)
      timersRef.current.set(id, tm)
    },
    [clearTimer, dismiss]
  )

  const showToast = useCallback(
    (message: string, variant: ToastVariant = 'info') => {
      idRef.current += 1
      const id = idRef.current
      const droppedIds: number[] = []
      setToasts((prev) => {
        const next = [...prev, { id, message, variant }]
        while (next.length > MAX_TOASTS) {
          const removed = next.shift()
          if (removed) droppedIds.push(removed.id)
        }
        return next
      })
      droppedIds.forEach(clearTimer)
      if (!hoveredRef.current) scheduleDismiss(id)
    },
    [clearTimer, scheduleDismiss]
  )

  function handleMouseEnter() {
    hoveredRef.current = true
    timersRef.current.forEach((tm) => {
      clearTimeout(tm)
    })
    timersRef.current.clear()
  }

  function handleMouseLeave() {
    hoveredRef.current = false
    toasts.forEach((t) => {
      scheduleDismiss(t.id)
    })
  }

  useEffect(() => {
    const timers = timersRef.current
    return () => {
      timers.forEach((tm) => {
        clearTimeout(tm)
      })
      timers.clear()
    }
  }, [])

  return (
    <ToastContext.Provider value={{ showToast }}>
      {children}
      {toasts.length > 0 && (
        <div
          className="toast-container"
          onMouseEnter={handleMouseEnter}
          onMouseLeave={handleMouseLeave}
        >
          {toasts.map((t) => (
            <div
              key={t.id}
              className={`toast toast-${t.variant}`}
              role={t.variant === 'error' ? 'alert' : 'status'}
              aria-live={t.variant === 'error' ? 'assertive' : 'polite'}
            >
              <span className="toast-message">{t.message}</span>
              <button
                type="button"
                className="toast-close"
                onClick={() => dismiss(t.id)}
                aria-label={translate('toastCloseLabel')}
              >
                ✕
              </button>
            </div>
          ))}
        </div>
      )}
    </ToastContext.Provider>
  )
}
