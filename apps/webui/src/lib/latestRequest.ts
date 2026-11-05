// 「最後に開始した要求だけが結果を適用できる」ための世代管理。
// begin() で新しい世代を得て、完了時に isCurrent(id) が false なら結果を捨てる。
// 別経路で文書を切り替えたときは invalidate() で保留中の要求をすべて無効化する。
export type LatestRequestGuard = {
  begin: () => number
  isCurrent: (id: number) => boolean
  invalidate: () => void
}

export function createLatestRequestGuard(): LatestRequestGuard {
  let latest = 0
  return {
    begin: () => ++latest,
    isCurrent: (id) => id === latest,
    invalidate: () => {
      latest++
    },
  }
}
