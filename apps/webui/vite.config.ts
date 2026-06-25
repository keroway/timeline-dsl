import { fileURLToPath } from 'node:url'
import { defineConfig } from 'vite'
import react from '@vitejs/plugin-react'
import wasm from 'vite-plugin-wasm'

const repoRoot = fileURLToPath(new URL('../..', import.meta.url))

// https://vite.dev/config/
// Note: Vite 8 has native top-level await support, so vite-plugin-top-level-await
// is not needed.
export default defineConfig({
  plugins: [react(), wasm()],
  base: './',
  build: {
    rollupOptions: {
      output: {
        // Split heavy vendors into their own cacheable chunks so the single
        // entry chunk no longer exceeds the 500KB warning threshold and the
        // browser can cache React / CodeMirror independently of app code.
        manualChunks(id: string) {
          if (!id.includes('node_modules')) return undefined
          if (id.includes('/react') || id.includes('/scheduler/')) {
            return 'react-vendor'
          }
          if (
            id.includes('/@codemirror/') ||
            id.includes('/@uiw/') ||
            id.includes('/@lezer/')
          ) {
            return 'codemirror-vendor'
          }
          return undefined
        },
      },
    },
  },
  server: {
    fs: {
      allow: [repoRoot],
    },
  },
})
