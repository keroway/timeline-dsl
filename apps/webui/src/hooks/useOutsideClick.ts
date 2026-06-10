import { useEffect, type Dispatch, type RefObject, type SetStateAction } from 'react'

// メニューが開いている間、外側クリックで閉じる。
export function useOutsideClick(
  ref: RefObject<HTMLElement | null>,
  isOpen: boolean,
  setOpen: Dispatch<SetStateAction<boolean>>,
): void {
  useEffect(() => {
    if (!isOpen) return
    function onOutside(e: globalThis.MouseEvent) {
      if (ref.current && !ref.current.contains(e.target as Node)) {
        setOpen(false)
      }
    }
    document.addEventListener('mousedown', onOutside)
    return () => document.removeEventListener('mousedown', onOutside)
  }, [isOpen, ref, setOpen])
}
