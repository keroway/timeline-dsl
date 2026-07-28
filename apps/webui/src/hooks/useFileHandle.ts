import { useMemo, useState } from 'react'
import type { ToastVariant } from '../components/Toast'
import type { Translator } from '../lib/i18n'
import { triggerDownload } from '../lib/svgExport'

const TDSL_FILE_TYPES = [
  {
    description: 'Timeline DSL files',
    accept: {
      'text/plain': ['.tdsl', '.txt'],
    },
  },
]

const DEFAULT_TDSL_FILENAME = 'timeline.tdsl'

type FilePickerHost = {
  showOpenFilePicker?: Window['showOpenFilePicker']
  showSaveFilePicker?: Window['showSaveFilePicker']
}

export type OpenFileSuccess = {
  status: 'opened'
  name: string
  text: string
  handle: FileSystemFileHandle
}

export type FileOperationResult =
  | OpenFileSuccess
  | {
      status: 'saved'
      name: string
      handle: FileSystemFileHandle | null
      mode: 'overwrite' | 'save-as' | 'download'
    }
  | { status: 'unsupported' }
  | { status: 'canceled' }

export type FileHandleApi = {
  supported: boolean
  fileName: string | null
  hasWritableHandle: boolean
  openWithPicker: () => Promise<FileOperationResult>
  markLegacyFileOpened: (name: string) => void
  saveSource: (source: string) => Promise<FileOperationResult>
}

export function isFileSystemAccessSupported(
  host: FilePickerHost = window
): boolean {
  return (
    typeof host.showOpenFilePicker === 'function' &&
    typeof host.showSaveFilePicker === 'function'
  )
}

function isAbortError(error: unknown): boolean {
  return error instanceof DOMException && error.name === 'AbortError'
}

export async function openTdslFile(
  host: FilePickerHost = window
): Promise<FileOperationResult> {
  if (!isFileSystemAccessSupported(host) || !host.showOpenFilePicker) {
    return { status: 'unsupported' }
  }

  try {
    const [handle] = await host.showOpenFilePicker({
      multiple: false,
      types: TDSL_FILE_TYPES,
    })
    if (!handle) return { status: 'canceled' }
    const file = await handle.getFile()
    return {
      status: 'opened',
      name: file.name || handle.name,
      text: await file.text(),
      handle,
    }
  } catch (error: unknown) {
    if (isAbortError(error)) return { status: 'canceled' }
    throw error
  }
}

type SaveTdslFileParams = {
  source: string
  handle: FileSystemFileHandle | null
  suggestedName?: string | null
  host?: FilePickerHost
  downloadFallback?: (blob: Blob, filename: string) => void | Promise<void>
}

export async function saveTdslFile(
  params: SaveTdslFileParams
): Promise<FileOperationResult> {
  const {
    source,
    handle,
    suggestedName,
    host = window,
    downloadFallback = triggerDownload,
  } = params
  const filename = suggestedName || handle?.name || DEFAULT_TDSL_FILENAME

  if (handle) {
    const writable = await handle.createWritable()
    await writable.write(source)
    await writable.close()
    return {
      status: 'saved',
      name: handle.name || filename,
      handle,
      mode: 'overwrite',
    }
  }

  if (!isFileSystemAccessSupported(host) || !host.showSaveFilePicker) {
    await downloadFallback(new Blob([source], { type: 'text/plain' }), filename)
    return { status: 'saved', name: filename, handle: null, mode: 'download' }
  }

  try {
    const saveHandle = await host.showSaveFilePicker({
      suggestedName: filename,
      types: TDSL_FILE_TYPES,
    })
    const writable = await saveHandle.createWritable()
    await writable.write(source)
    await writable.close()
    return {
      status: 'saved',
      name: saveHandle.name || filename,
      handle: saveHandle,
      mode: 'save-as',
    }
  } catch (error: unknown) {
    if (isAbortError(error)) return { status: 'canceled' }
    throw error
  }
}

export function useFileHandle(
  showToast: (message: string, variant?: ToastVariant) => void,
  t: Translator
): FileHandleApi {
  const supported = isFileSystemAccessSupported()
  const [handle, setHandle] = useState<FileSystemFileHandle | null>(null)
  const [fileName, setFileName] = useState<string | null>(null)

  return useMemo(
    () => ({
      supported,
      fileName,
      hasWritableHandle: handle !== null,
      async openWithPicker() {
        const result = await openTdslFile()
        if (result.status === 'opened') {
          setHandle(result.handle)
          setFileName(result.name)
        } else if (result.status === 'unsupported') {
          showToast(t('fileAccessUnsupported'), 'info')
        }
        return result
      },
      markLegacyFileOpened(name: string) {
        setHandle(null)
        setFileName(name)
      },
      async saveSource(source: string) {
        try {
          const result = await saveTdslFile({
            source,
            handle,
            suggestedName: fileName,
          })
          if (result.status === 'saved') {
            setHandle(result.handle)
            setFileName(result.name)
            if (result.mode === 'download') {
              showToast(t('fileAccessDownloadFallback'), 'info')
            } else if (result.mode === 'overwrite') {
              showToast(
                t.fmt('fileAccessSaved', { name: result.name }),
                'success'
              )
            } else {
              showToast(
                t.fmt('fileAccessSavedAs', { name: result.name }),
                'success'
              )
            }
          }
          return result
        } catch (error: unknown) {
          const msg = error instanceof Error ? error.message : String(error)
          showToast(t.fmt('fileAccessSaveFailed', { msg }), 'error')
          throw error
        }
      },
    }),
    [fileName, handle, showToast, supported, t]
  )
}
