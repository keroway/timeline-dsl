import { fileURLToPath } from 'node:url'
import { defineConfig } from 'vitest/config'

const repoRoot = fileURLToPath(new URL('../..', import.meta.url))

export default defineConfig({
  server: {
    fs: {
      allow: [repoRoot],
    },
  },
  test: {
    environment: 'jsdom',
    // Node 26's experimental global localStorage shadows jsdom's implementation.
    // Keep jsdom as the browser-storage source used by WebUI tests.
    execArgv: ['--no-experimental-webstorage'],
    include: [
      'src/**/*.test.ts',
      'src/**/*.spec.ts',
      'src/**/*.test.tsx',
      'src/**/*.spec.tsx',
    ],
  },
})
