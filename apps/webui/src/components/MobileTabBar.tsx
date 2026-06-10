import type { Dispatch, SetStateAction } from 'react'

export type MobileTab = 'editor' | 'preview'

type MobileTabBarProps = {
  mobileTab: MobileTab
  setMobileTab: Dispatch<SetStateAction<MobileTab>>
}

export function MobileTabBar({ mobileTab, setMobileTab }: MobileTabBarProps) {
  return (
    <div className="mobile-tab-bar">
      <button
        className={`mobile-tab${mobileTab === 'editor' ? ' mobile-tab-active' : ''}`}
        onClick={() => setMobileTab('editor')}
      >
        エディタ
      </button>
      <button
        className={`mobile-tab${mobileTab === 'preview' ? ' mobile-tab-active' : ''}`}
        onClick={() => setMobileTab('preview')}
      >
        プレビュー
      </button>
    </div>
  )
}
