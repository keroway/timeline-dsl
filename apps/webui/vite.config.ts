import { fileURLToPath } from 'node:url'
import react from '@vitejs/plugin-react'
import { defineConfig } from 'vite'
import { VitePWA } from 'vite-plugin-pwa'
import wasm from 'vite-plugin-wasm'

const repoRoot = fileURLToPath(new URL('../..', import.meta.url))

// https://vite.dev/config/
// Note: Vite 8 has native top-level await support, so vite-plugin-top-level-await
// is not needed.
export default defineConfig({
  plugins: [
    react(),
    wasm(),
    VitePWA({
      registerType: 'prompt',
      injectRegister: false,
      manifestFilename: 'manifest.webmanifest',
      workbox: {
        globPatterns: ['**/*.{css,html,js,json,png,svg,wasm}'],
        navigateFallback: 'index.html',
      },
      manifest: {
        name: 'Timeline DSL WebUI',
        short_name: 'TDSL WebUI',
        description:
          'Offline-capable editor and previewer for Timeline DSL files.',
        theme_color: '#13131f',
        background_color: '#13131f',
        display: 'standalone',
        start_url: '.',
        scope: '.',
        icons: [
          {
            src: 'pwa-192.png',
            sizes: '192x192',
            type: 'image/png',
            purpose: 'any maskable',
          },
          {
            src: 'pwa-512.png',
            sizes: '512x512',
            type: 'image/png',
            purpose: 'any maskable',
          },
        ],
      },
    }),
  ],
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
  worker: {
    // The tdsl.worker.ts module worker imports `@keroway/tdsl-wasm` directly, so
    // vite-plugin-wasm must also be registered for the worker's own build
    // pipeline (Vite does not inherit top-level plugins into workers).
    plugins: () => [wasm()],
  },
})
