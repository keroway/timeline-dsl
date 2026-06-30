#!/usr/bin/env node
// Generates keyword patterns in tdsl.tmLanguage.json from the single source of truth
// apps/webui/src/lang-tdsl/keywords.json.
// Run via `npm run prebuild` in apps/webui or manually with `node editors/vscode/scripts/gen-grammar-keywords.mjs`.

import { readFileSync, writeFileSync } from "fs";
import { dirname, resolve } from "path";
import { fileURLToPath } from "url";

const __dirname = dirname(fileURLToPath(import.meta.url));
const repoRoot = resolve(__dirname, "../../..");
const keywordsFile = resolve(
	repoRoot,
	"apps/webui/src/lang-tdsl/keywords.json",
);
const tmLanguageFile = resolve(__dirname, "../syntaxes/tdsl.tmLanguage.json");

const keywords = JSON.parse(readFileSync(keywordsFile, "utf8"));

function getArray(name) {
	const arr = keywords[name];
	if (!Array.isArray(arr) || arr.length === 0) {
		throw new Error(`Cannot find non-empty array ${name} in ${keywordsFile}`);
	}
	return arr;
}

const blockKeywords = getArray("BLOCK_KEYWORDS");
const itemKeywords = getArray("ITEM_KEYWORDS");
const miscKeywords = getArray("MISC_KEYWORDS");

const tmJson = JSON.parse(readFileSync(tmLanguageFile, "utf8"));
tmJson.repository["keyword-block"].match = `\\b(${blockKeywords.join("|")})\\b`;
tmJson.repository["keyword-item"].match = `\\b(${itemKeywords.join("|")})\\b`;
tmJson.repository["keyword-misc"].match = `\\b(${miscKeywords.join("|")})\\b`;

writeFileSync(tmLanguageFile, JSON.stringify(tmJson, null, 2) + "\n");
console.log(`gen-grammar-keywords: updated ${tmLanguageFile}`);
