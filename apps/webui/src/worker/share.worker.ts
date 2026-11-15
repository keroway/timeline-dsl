import { ShareError } from '../shareLimits'
import { inflateSource } from './shareDecode'
self.onmessage = ({ data }: MessageEvent<string>) => {
  try {
    self.postMessage({ source: inflateSource(data) })
  } catch (error) {
    self.postMessage({
      error: error instanceof ShareError ? error.code : 'invalid',
    })
  }
}
