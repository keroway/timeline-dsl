type TooltipProps = {
  tooltip: { text: string; x: number; y: number } | null
}

export function Tooltip({ tooltip }: TooltipProps) {
  if (!tooltip) return null
  return (
    <div
      className="tdsl-tooltip"
      style={{ left: tooltip.x + 12, top: tooltip.y + 12 }}
    >
      {tooltip.text.split('\n').map((line, i) => (
        // biome-ignore lint/suspicious/noArrayIndexKey: line list is derived fresh from a fixed string each render, order never changes
        <div key={i}>{line}</div>
      ))}
    </div>
  )
}
