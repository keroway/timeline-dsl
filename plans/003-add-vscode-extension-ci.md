# Plan 003 — Add CI coverage for VS Code extension tests

Written against commit `9d6d02f`.

## Problem

The VS Code extension has its own TypeScript tests and compile step, but regular CI currently checks only the WebUI Node project. The publish workflow packages/publishes the extension without first running the extension test script.

Evidence:

- `editors/vscode/package.json` defines `compile`, `typecheck`, and `test`.
- `.github/workflows/ci.yml` has a WebUI Node job, but no VS Code extension job.
- `.github/workflows/vscode-publish.yml` installs dependencies and packages, but does not run `npm test` before publishing.

## Implementation steps

1. Add a CI job that runs in `editors/vscode`:
   - setup Node.js 22 with npm cache for `editors/vscode/package-lock.json`;
   - `npm ci`;
   - `npm test`;
   - `npm run compile`.
2. Add `npm test` to `vscode-publish.yml` before `vsce package`.
3. Run `npm test` locally in `editors/vscode`.

## Done criteria

- CI catches TypeScript/test regressions in the VS Code extension.
- Publish workflow refuses to package/publish if extension tests fail.
