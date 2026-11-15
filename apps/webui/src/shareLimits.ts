export const MAX_ENCODED_CHARS = 256 * 1024
export const MAX_SOURCE_BYTES = 1024 * 1024
export const SHARE_TIMEOUT_MS = 5000
export type ShareErrorCode = 'invalid' | 'tooLarge' | 'timeout' | 'cancelled'
export class ShareError extends Error {
  readonly code: ShareErrorCode
  constructor(code: ShareErrorCode) {
    super(code)
    this.code = code
  }
}
export function checkEncodedSize(encoded: string): void {
  if (encoded.length > MAX_ENCODED_CHARS) throw new ShareError('tooLarge')
}
export function checkSourceSize(source: string): void {
  if (
    source.length > MAX_SOURCE_BYTES ||
    new TextEncoder().encode(source).length > MAX_SOURCE_BYTES
  )
    throw new ShareError('tooLarge')
}
