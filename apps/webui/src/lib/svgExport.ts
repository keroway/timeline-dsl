// SVG 由来のエクスポート（PNG 変換・ダウンロード）に関する純粋ヘルパー群。

// レンダラーが出力する SVG は基本 CSS を内包している（#1051）。ここで CSS を追加すると
// 詳細度の差で外側ラベル色などを上書きするため（#1059）、SVG をそのまま変換する。
export function svgToPngBlob(svg: string, whiteBg: boolean): Promise<Blob> {
  return new Promise((resolve, reject) => {
    const blob = new Blob([svg],{ type: 'image/svg+xml;charset=utf-8' })
    const url = URL.createObjectURL(blob)
    const img = new Image()
    img.onload = () => {
      try {
        const canvas = document.createElement('canvas')
        canvas.width = img.naturalWidth || img.width || 800
        canvas.height = img.naturalHeight || img.height || 400
        const ctx = canvas.getContext('2d')
        if (!ctx) throw new Error('canvas 2d context unavailable')
        if (whiteBg) {
          ctx.fillStyle = '#ffffff'
          ctx.fillRect(0, 0, canvas.width, canvas.height)
        }
        ctx.drawImage(img, 0, 0)
        URL.revokeObjectURL(url)
        canvas.toBlob((b) => {
          if (b) resolve(b)
          else reject(new Error('canvas.toBlob failed'))
        }, 'image/png')
      } catch (e) {
        URL.revokeObjectURL(url)
        reject(e instanceof Error ? e : new Error(String(e)))
      }
    }
    img.onerror = () => {
      URL.revokeObjectURL(url)
      reject(new Error('SVG load failed'))
    }
    img.src = url
  })
}

// Trigger a browser download for the given blob. Centralizes the
// Blob → object URL → <a download> → revoke dance shared by every export.
export function triggerDownload(blob: Blob, filename: string): void {
  const url = URL.createObjectURL(blob)
  const a = document.createElement('a')
  a.href = url
  a.download = filename
  a.click()
  URL.revokeObjectURL(url)
}
