# Plan 004 — Harden VS Code publish dependency install

Written against commit `fc7b648`.

## Problem

The VS Code publish workflow installs extension dependencies with `npm install`. For CI / release workflows, `npm ci` is preferable because it uses the committed lockfile exactly and fails if `package.json` and `package-lock.json` drift. `npm install` can update the lockfile during the publish job and makes releases less reproducible.

Evidence:

- `.github/workflows/vscode-publish.yml` installs `editors/vscode` dependencies with `npm install`.
- `editors/vscode/package-lock.json` is committed, so the workflow can use `npm ci`.

## Implementation steps

1. Configure `actions/setup-node` to cache npm dependencies using `editors/vscode/package-lock.json`.
2. Change the extension dependency install step from `npm install` to `npm ci`.
3. Keep the existing `npm test` gate before packaging.

## Done criteria

- VS Code publish workflow uses lockfile-strict install for project dependencies.
- Existing VS Code extension tests still pass locally.
