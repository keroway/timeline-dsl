import { registerSW } from 'virtual:pwa-register'
import { useEffect, useRef, useState } from 'react'
import type { ToastVariant } from '../components/Toast'
import type { Translator } from '../lib/i18n'

type ShowToast = (message: string, variant?: ToastVariant) => void

export type PwaUpdateState = {
  needRefresh: boolean
  updateServiceWorker: ((reloadPage?: boolean) => Promise<void>) | null
}

function formatRegistrationError(error: unknown): string {
  if (error instanceof Error) return error.message
  return String(error)
}

export function usePwaLifecycle(
  showToast: ShowToast,
  t: Translator
): PwaUpdateState {
  const showToastRef = useRef(showToast)
  const tRef = useRef(t)
  const [updateState, setUpdateState] = useState<PwaUpdateState>({
    needRefresh: false,
    updateServiceWorker: null,
  })

  useEffect(() => {
    showToastRef.current = showToast
    tRef.current = t
  }, [showToast, t])

  useEffect(() => {
    let updateServiceWorker: PwaUpdateState['updateServiceWorker'] = null
    updateServiceWorker = registerSW({
      immediate: true,
      onOfflineReady() {
        showToastRef.current(tRef.current('pwaOfflineReady'), 'success')
      },
      onNeedRefresh() {
        setUpdateState({ needRefresh: true, updateServiceWorker })
        showToastRef.current(tRef.current('pwaUpdateAvailable'), 'info')
      },
      onRegisterError(error) {
        showToastRef.current(
          tRef.current.fmt('pwaRegistrationFailed', {
            msg: formatRegistrationError(error),
          }),
          'error'
        )
      },
    })

    const handleOffline = () => {
      showToastRef.current(tRef.current('pwaNetworkOffline'), 'error')
    }
    const handleOnline = () => {
      showToastRef.current(tRef.current('pwaNetworkOnline'), 'success')
    }

    window.addEventListener('offline', handleOffline)
    window.addEventListener('online', handleOnline)

    return () => {
      window.removeEventListener('offline', handleOffline)
      window.removeEventListener('online', handleOnline)
    }
  }, [])

  return updateState
}
