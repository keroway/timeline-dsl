#!/usr/bin/env node
// Generates keyword patterns in tdsl.tmLanguage.json from apps/webui/src/lang-tdsl/keywords.ts.
// Run via `npm run prebuild` in apps/webui or manually with `node editors/vscode/scripts/gen-grammar-keywords.mjs`.

import { readFileSync, writeFileSync } from "fs"
import { dirname, resolve } from "path"
import { fileURLToPath } from "url"

const __dirname = dirname(fileURLToPath(import.meta.url))
const repoRoot = resolve(__dirname, "../../..")
const keywordsFile = resolve(repoRoot, "apps/webui/src/lang-tdsl/keywords.ts")
const tmLanguageFile = resolve(__dirname, "../syntaxes/tdsl.tmLanguage.json")

function extractArray(src, name) {
  const re = new RegExp(`export const ${name}\\s*=\\s*\\[([\\s\\S]*?)\\]`)
  const m = src.match(re)
  if (!m) throw new Error(`Cannot find ${name} in ${keywordsFile}`)
  const items = m[1].match(/"([^"]+)"/g)
  if (!items) throw new Error(`No string values found in ${name}`)
  return items.map(s => s.slice(1, -1))
}

const src = readFileSync(keywordsFile, "utf8")
const blockKeywords = extractArray(src, "BLOCK_KEYWORDS")
const itemKeywords = extractArray(src, "ITEM_KEYWORDS")
const miscKeywords = extractArray(src, "MISC_KEYWORDS")

const tmJson = JSON.parse(readFileSync(tmLanguageFile, "utf8"))
tmJson.repository["keyword-block"].match = `\\b(${blockKeywords.join("|")})\\b`
tmJson.repository["keyword-item"].match = `\\b(${itemKeywords.join("|")})\\b`
tmJson.repository["keyword-misc"].match = `\\b(${miscKeywords.join("|")})\\b`

writeFileSync(tmLanguageFile, JSON.stringify(tmJson, null, 2) + "\n")
console.log(`gen-grammar-keywords: updated ${tmLanguageFile}`)
