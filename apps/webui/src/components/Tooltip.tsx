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
        <div key={i}>{line}</div>
      ))}
    </div>
  )
}
