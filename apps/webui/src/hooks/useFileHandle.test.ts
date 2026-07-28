import { describe, expect, it, vi } from 'vitest'
import {
  isFileSystemAccessSupported,
  openTdslFile,
  saveTdslFile,
} from './useFileHandle'

function createMockWritable(
  onClose: (value: string) => void
): FileSystemWritableFileStream {
  let value = ''
  const writable = {
    write: vi.fn(async (chunk: FileSystemWriteChunkType) => {
      value +=
        typeof chunk === 'string'
          ? chunk
          : await new Blob([chunk as BlobPart]).text()
    }),
    seek: vi.fn(async () => undefined),
    truncate: vi.fn(async () => undefined),
    close: vi.fn(async () => onClose(value)),
  }
  return writable as unknown as FileSystemWritableFileStream
}

function createMockFileHandle(
  name: string,
  initialText: string
): FileSystemFileHandle {
  let text = initialText
  const handle = {
    kind: 'file' as const,
    name,
    getFile: vi.fn(async () => new File([text], name, { type: 'text/plain' })),
    createWritable: vi.fn(async () =>
      createMockWritable((value) => {
        text = value
      })
    ),
    isSameEntry: vi.fn(async () => false),
  }
  return handle as unknown as FileSystemFileHandle
}

type PickerHost = Parameters<typeof openTdslFile>[0]

describe('File System Access helpers', () => {
  it('reports unsupported browsers and uses download fallback when saving', async () => {
    const downloads: Array<{ filename: string; text: string }> = []
    const host: PickerHost = {}

    expect(isFileSystemAccessSupported(host)).toBe(false)

    const result = await saveTdslFile({
      source: 'timeline { title "Fallback"; }',
      handle: null,
      host,
      downloadFallback: async (blob, filename) => {
        downloads.push({ filename, text: await blob.text() })
      },
    })

    expect(result).toMatchObject({
      status: 'saved',
      mode: 'download',
      name: 'timeline.tdsl',
    })
    expect(downloads).toEqual([
      { filename: 'timeline.tdsl', text: 'timeline { title "Fallback"; }' },
    ])
  })

  it('opens, overwrites, and re-opens the same mocked file handle', async () => {
    const handle = createMockFileHandle(
      'project.tdsl',
      'timeline { title "Before"; }'
    )
    const host: PickerHost = {
      showOpenFilePicker: vi.fn(async () => [handle]),
      showSaveFilePicker: vi.fn(),
    }

    const opened = await openTdslFile(host)
    expect(opened).toMatchObject({
      status: 'opened',
      name: 'project.tdsl',
      text: 'timeline { title "Before"; }',
    })
    if (opened.status !== 'opened') throw new Error('expected opened result')

    const saved = await saveTdslFile({
      source: 'timeline { title "After"; }',
      handle: opened.handle,
      host,
    })
    expect(saved).toMatchObject({
      status: 'saved',
      mode: 'overwrite',
      name: 'project.tdsl',
    })

    const reopened = await openTdslFile(host)
    expect(reopened).toMatchObject({
      status: 'opened',
      name: 'project.tdsl',
      text: 'timeline { title "After"; }',
    })
    expect(host.showSaveFilePicker).not.toHaveBeenCalled()
  })

  it('uses showSaveFilePicker when supported and no writable handle exists', async () => {
    const handle = createMockFileHandle('new-project.tdsl', '')
    const host: PickerHost = {
      showOpenFilePicker: vi.fn(),
      showSaveFilePicker: vi.fn(async () => handle),
    }

    const saved = await saveTdslFile({
      source: 'timeline { title "New"; }',
      handle: null,
      suggestedName: 'new-project.tdsl',
      host,
    })

    expect(saved).toMatchObject({
      status: 'saved',
      mode: 'save-as',
      name: 'new-project.tdsl',
    })
    const reopenedHost: PickerHost = {
      showOpenFilePicker: vi.fn(async () => [handle]),
      showSaveFilePicker: vi.fn(),
    }
    await expect(openTdslFile(reopenedHost)).resolves.toMatchObject({
      status: 'opened',
      text: 'timeline { title "New"; }',
    })
  })
})
