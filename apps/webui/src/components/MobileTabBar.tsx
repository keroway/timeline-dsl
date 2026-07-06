import { useMemo, type Dispatch, type SetStateAction } from 'react'
import { createTranslator, type Locale } from '../lib/i18n'

export type MobileTab = 'editor' | 'preview'

type MobileTabBarProps = {
  mobileTab: MobileTab
  setMobileTab: Dispatch<SetStateAction<MobileTab>>
  locale: Locale
}

export function MobileTabBar({ mobileTab, setMobileTab, locale }: MobileTabBarProps) {
  const t = useMemo(() => createTranslator(locale), [locale])

  return (
    <div className="mobile-tab-bar">
      <button
        className={`mobile-tab${mobileTab === 'editor' ? ' mobile-tab-active' : ''}`}
        onClick={() => setMobileTab('editor')}
      >
        {t('mobileTabEditor')}
      </button>
      <button
        className={`mobile-tab${mobileTab === 'preview' ? ' mobile-tab-active' : ''}`}
        onClick={() => setMobileTab('preview')}
      >
        {t('mobileTabPreview')}
      </button>
    </div>
  )
}
