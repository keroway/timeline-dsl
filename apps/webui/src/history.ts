export type SnapshotKind = 'auto' | 'manual'

export type Snapshot = {
  id: string
  kind: SnapshotKind
  source: string
  label: string
  createdAt: number
}

const HISTORY_AUTO_KEY = 'tdsl:editor:history'
const HISTORY_MANUAL_KEY = 'tdsl:editor:history:manual'
const AUTO_MAX = 5
const AUTO_INTERVAL_MS = 5 * 60 * 1000

function readSnapshots(key: string): Snapshot[] {
  try {
    const raw = localStorage.getItem(key)
    if (!raw) return []
    return JSON.parse(raw) as Snapshot[]
  } catch {
    return []
  }
}

function writeSnapshots(key: string, snaps: Snapshot[]): void {
  try {
    localStorage.setItem(key, JSON.stringify(snaps))
  } catch {
    // quota or private browsing
  }
}

function makeId(): string {
  return `${Date.now()}-${Math.random().toString(36).slice(2, 7)}`
}

function formatDate(ts: number): string {
  const d = new Date(ts)
  const pad = (n: number) => String(n).padStart(2, '0')
  return `${d.getFullYear()}/${pad(d.getMonth() + 1)}/${pad(d.getDate())} ${pad(d.getHours())}:${pad(d.getMinutes())}`
}

export function readAutoSnapshots(): Snapshot[] {
  return readSnapshots(HISTORY_AUTO_KEY)
}

export function readManualSnapshots(): Snapshot[] {
  return readSnapshots(HISTORY_MANUAL_KEY)
}

export function pushAutoSnapshot(source: string, reason: string): void {
  const snaps = readSnapshots(HISTORY_AUTO_KEY)
  const snap: Snapshot = {
    id: makeId(),
    kind: 'auto',
    source,
    label: `${reason} — ${formatDate(Date.now())}`,
    createdAt: Date.now(),
  }
  const updated = [snap, ...snaps].slice(0, AUTO_MAX)
  writeSnapshots(HISTORY_AUTO_KEY, updated)
}

export function shouldAutoSnapshot(source: string, lastAutoMs: number): boolean {
  if (Date.now() - lastAutoMs < AUTO_INTERVAL_MS) return false
  const snaps = readSnapshots(HISTORY_AUTO_KEY)
  if (snaps.length === 0) return true
  return snaps[0].source !== source
}

export function pushManualSnapshot(source: string, label: string, fallbackPrefix: string): Snapshot {
  const snaps = readSnapshots(HISTORY_MANUAL_KEY)
  const snap: Snapshot = {
    id: makeId(),
    kind: 'manual',
    source,
    label: label || `${fallbackPrefix} — ${formatDate(Date.now())}`,
    createdAt: Date.now(),
  }
  writeSnapshots(HISTORY_MANUAL_KEY, [snap, ...snaps])
  return snap
}

export function renameManualSnapshot(id: string, label: string): void {
  const snaps = readSnapshots(HISTORY_MANUAL_KEY)
  writeSnapshots(HISTORY_MANUAL_KEY, snaps.map((s) => s.id === id ? { ...s, label } : s))
}

export function deleteManualSnapshot(id: string): void {
  const snaps = readSnapshots(HISTORY_MANUAL_KEY)
  writeSnapshots(HISTORY_MANUAL_KEY, snaps.filter((s) => s.id !== id))
}

export function clearAllHistory(): void {
  try {
    localStorage.removeItem(HISTORY_AUTO_KEY)
    localStorage.removeItem(HISTORY_MANUAL_KEY)
  } catch {
    // ignore
  }
}
