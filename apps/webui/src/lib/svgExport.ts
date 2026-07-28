// SVG 由来のエクスポート（PNG 変換・ダウンロード）に関する純粋ヘルパー群。

const SVG_EMBEDDED_CSS = `
  .tdsl-lane-band-even { fill: #ffffff; }
  .tdsl-lane-band-odd  { fill: #f5f5f7; }
  .tdsl-axis-baseline  { stroke: #888888; stroke-width: 1; }
  .tdsl-axis-tick      { stroke: #e0e0e0; stroke-width: 1; }
  .tdsl-axis-text      { font-size: 11px; fill: #666666; font-family: -apple-system, BlinkMacSystemFont, "Segoe UI", sans-serif; }
  .tdsl-lane-label     { font-size: 13px; fill: #333333; font-weight: 500; font-family: -apple-system, BlinkMacSystemFont, "Segoe UI", sans-serif; }
  .tdsl-item-label     { font-size: 11px; fill: #ffffff; font-family: -apple-system, BlinkMacSystemFont, "Segoe UI", sans-serif; }
  .tdsl-event-stem     { stroke: #666666; stroke-width: 1.5; }
  .tdsl-event-hit      { fill: transparent; }
  .tdsl-span           { fill-opacity: 0.78; }
  .tdsl-event-range    { fill-opacity: 0.75; }
  .tdsl-event-dot      { stroke: #ffffff; stroke-width: 1; }
`

function svgWithEmbeddedStyles(svg: string): string {
  return svg.replace('</style>', `${SVG_EMBEDDED_CSS}</style>`)
}

export function svgToPngBlob(svg: string, whiteBg: boolean): Promise<Blob> {
  return new Promise((resolve, reject) => {
    const enriched = svgWithEmbeddedStyles(svg)
    const blob = new Blob([enriched], { type: 'image/svg+xml;charset=utf-8' })
    const url = URL.createObjectURL(blob)
    const img = new Image()
    img.onload = () => {
      const canvas = document.createElement('canvas')
      canvas.width = img.naturalWidth || img.width || 800
      canvas.height = img.naturalHeight || img.height || 400
      const ctx = canvas.getContext('2d')!
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
