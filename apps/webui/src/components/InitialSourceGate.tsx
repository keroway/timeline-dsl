import { type ReactNode, useEffect, useState } from 'react'
import { EXAMPLES } from '../examples'
import { useSettings } from '../hooks/useSettings'
import { createTranslator } from '../lib/i18n'
import { readInitialSource, readSavedSource } from '../lib/initialSource'
import { ShareError, type ShareErrorCode } from '../shareLimits'

export function InitialSourceGate({
  children,
}: {
  children: (source: string) => ReactNode
}) {
  const [source, setSource] = useState<string | null>(null)
  const [error, setError] = useState<ShareErrorCode | null>(null)
  const [controller] = useState(() => new AbortController())
  const { settings } = useSettings()
  const t = createTranslator(settings.locale)
  useEffect(() => {
    // A fresh controller also supports React StrictMode effect replay.
    const request = new AbortController()
    const abort = () => request.abort()
    controller.signal.addEventListener('abort', abort)
    let active = true
    readInitialSource(request.signal).then(
      (value) => {
        if (active) setSource(value)
      },
      (reason) => {
        if (active)
          setError(reason instanceof ShareError ? reason.code : 'invalid')
      }
    )
    return () => {
      active = false
      request.abort()
      controller.signal.removeEventListener('abort', abort)
    }
  }, [controller])
  if (source !== null) return children(source)
  return (
    <main className="initial-source-gate">
      <p role={error ? 'alert' : 'status'}>
        {error
          ? t(
              error === 'tooLarge'
                ? 'shareTooLarge'
                : error === 'timeout'
                  ? 'shareTimeout'
                  : error === 'cancelled'
                    ? 'shareCancelled'
                    : 'shareInvalid'
            )
          : t('shareLoading')}
      </p>
      {error ? (
        <>
          <button type="button" onClick={() => setSource(readSavedSource())}>
            {t('shareOpenSaved')}
          </button>
          <button type="button" onClick={() => setSource(EXAMPLES[0].source)}>
            {t('shareOpenSample')}
          </button>
        </>
      ) : (
        <button type="button" onClick={() => controller.abort()}>
          {t('confirmCancel')}
        </button>
      )}
    </main>
  )
}
