import { useEffect, useRef, useState, type Dispatch, type RefObject, type SetStateAction } from 'react'
import { DEBOUNCE_MS } from '../lib/constants'
import {
  type Snapshot,
  clearAllHistory,
  deleteManualSnapshot,
  pushAutoSnapshot,
  pushManualSnapshot,
  readAutoSnapshots,
  readManualSnapshots,
  renameManualSnapshot,
  shouldAutoSnapshot,
} from '../history'
import type { ToastVariant } from '../components/Toast'

type Params = {
  source: string
  historyEnabled: boolean
  showToast: (message: string, variant?: ToastVariant) => void
  setSource: Dispatch<SetStateAction<string>>
  setShowHistory: Dispatch<SetStateAction<boolean>>
  skipAutoSaveRef: RefObject<boolean>
}

export type HistoryApi = {
  autoSnaps: Snapshot[]
  manualSnaps: Snapshot[]
  renamingId: string | null
  renameValue: string
  setRenameValue: Dispatch<SetStateAction<string>>
  // テンプレート/ファイル読み込み直前に自動スナップショットを取る
  snapshotBeforeLoad: (label: string) => void
  handleSaveToHistory: () => void
  handleRestoreSnapshot: (src: string) => void
  handleRenameStart: (snap: Snapshot) => void
  handleRenameCommit: () => void
  cancelRename: () => void
  handleDeleteManual: (id: string) => void
  handleClearAllHistory: () => void
}

// 履歴スナップショット（自動 + 手動）の state とすべての操作を所有する。
export function useHistorySnapshots(params: Params): HistoryApi {
  const { source, historyEnabled, showToast, setSource, setShowHistory, skipAutoSaveRef } = params

  const [autoSnaps, setAutoSnaps] = useState<Snapshot[]>(() => readAutoSnapshots())
  const [manualSnaps, setManualSnaps] = useState<Snapshot[]>(() => readManualSnapshots())
  const [renamingId, setRenamingId] = useState<string | null>(null)
  const [renameValue, setRenameValue] = useState('')
  const lastAutoSnapRef = useRef<number>(0)

  // Auto-snapshot on timed interval (5 min + diff present)
  useEffect(() => {
    if (!historyEnabled) return
    const timer = setTimeout(() => {
      if (shouldAutoSnapshot(source, lastAutoSnapRef.current)) {
        pushAutoSnapshot(source, '自動保存')
        lastAutoSnapRef.current = Date.now()
        setAutoSnaps(readAutoSnapshots())
      }
    }, DEBOUNCE_MS)
    return () => clearTimeout(timer)
  }, [source, historyEnabled])

  function snapshotBeforeLoad(label: string) {
    if (historyEnabled && source.trim()) {
      pushAutoSnapshot(source, label)
      lastAutoSnapRef.current = Date.now()
      setAutoSnaps(readAutoSnapshots())
    }
  }

  function handleSaveToHistory() {
    const snap = pushManualSnapshot(source, `手動保存 — ${new Date().toLocaleString('ja-JP')}`)
    setManualSnaps(readManualSnapshots())
    showToast(`履歴に保存しました: ${snap.label}`, 'success')
  }

  function handleRestoreSnapshot(src: string) {
    skipAutoSaveRef.current = true
    setSource(src)
    setShowHistory(false)
  }

  function handleRenameStart(snap: Snapshot) {
    setRenamingId(snap.id)
    setRenameValue(snap.label)
  }

  function handleRenameCommit() {
    if (renamingId && renameValue.trim()) {
      renameManualSnapshot(renamingId, renameValue.trim())
      setManualSnaps(readManualSnapshots())
    }
    setRenamingId(null)
    setRenameValue('')
  }

  function cancelRename() {
    setRenamingId(null)
    setRenameValue('')
  }

  function handleDeleteManual(id: string) {
    deleteManualSnapshot(id)
    setManualSnaps(readManualSnapshots())
  }

  function handleClearAllHistory() {
    clearAllHistory()
    setAutoSnaps([])
    setManualSnaps([])
    showToast('履歴を全件削除しました', 'success')
  }

  return {
    autoSnaps,
    manualSnaps,
    renamingId,
    renameValue,
    setRenameValue,
    snapshotBeforeLoad,
    handleSaveToHistory,
    handleRestoreSnapshot,
    handleRenameStart,
    handleRenameCommit,
    cancelRename,
    handleDeleteManual,
    handleClearAllHistory,
  }
}
