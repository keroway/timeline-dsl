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
  server: {
    fs: {
      allow: [repoRoot],
    },
  },
})
