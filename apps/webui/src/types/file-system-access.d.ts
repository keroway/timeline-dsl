// Ambient types for the File System Access API entry points
// (window.showOpenFilePicker / window.showSaveFilePicker). TypeScript's bundled
// DOM lib already declares FileSystemFileHandle / FileSystemWritableFileStream,
// but not the picker functions themselves.
// Spec: https://wicg.github.io/file-system-access/

export {}

declare global {
  interface FilePickerAcceptType {
    description?: string
    accept: Record<string, string[]>
  }

  interface OpenFilePickerOptions {
    multiple?: boolean
    excludeAcceptAllOption?: boolean
    types?: FilePickerAcceptType[]
  }

  interface SaveFilePickerOptions {
    suggestedName?: string
    excludeAcceptAllOption?: boolean
    types?: FilePickerAcceptType[]
  }

  interface Window {
    showOpenFilePicker?(options?: OpenFilePickerOptions): Promise<FileSystemFileHandle[]>
    showSaveFilePicker?(options?: SaveFilePickerOptions): Promise<FileSystemFileHandle>
  }
}
