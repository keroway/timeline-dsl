<!-- markdownlint-disable MD013 MD060 -->

# Plan 010: Harden GitHub Pages deploy against transient "Deployment failed, try again later"

> **Executor instructions**: Follow this plan step by step. Run every verification command and confirm the expected result before moving on. If anything in "STOP conditions" occurs, stop and report — do not improvise. When done, update the status row for this plan in `plans/README.md`.
>
> **Drift check (run first)**: `git diff --stat 1c22dac..HEAD -- .github/workflows/deploy-pages.yml`
> If the workflow changed since this plan was written, compare against the "Current state" excerpt before proceeding; on a mismatch, treat it as a STOP condition.

## Status

- **Priority**: P2
- **Effort**: S
- **Risk**: LOW
- **Depends on**: none
- **Category**: CI reliability
- **Planned at**: commit `1c22dac`, 2026-07-05
- **Tracking issue**: #588

## Why this matters

The **Deploy WebUI to GitHub Pages** run for #586 (run id `28727806736`, 2026-07-05) failed. Investigation shows this is **not a code or build regression**:

- The `build` job succeeded and uploaded the `github-pages` artifact (`artifact_id: 8087797921`).
- The `deploy` job failed only at the final polling step:
  `Getting Pages deployment status... ##[error]Deployment failed, try again later.`
- The live site still returns HTTP 200 (served from the prior successful deploy on 2026-07-04), and `gh api repos/keroway/timeline-dsl/pages` confirms `build_type: "workflow"` with a healthy config.

This is a well-known **transient GitHub Pages backend error** in `actions/deploy-pages`: the deployment is created, but the status-poll returns a server-side failure. It resolves on re-run. The goal of this plan is to make the workflow **self-heal** so a single transient blip does not leave `main` showing a red deploy badge and does not require a manual re-run.

## Current state

`.github/workflows/deploy-pages.yml` — the `deploy` job (excerpt):

```yaml
  deploy:
    environment:
      name: github-pages
      url: ${{ steps.deployment.outputs.page_url }}
    runs-on: ubuntu-latest
    needs: build
    steps:
      - name: Deploy to GitHub Pages
        id: deployment
        uses: actions/deploy-pages@v5
```

Notes:

- The workflow only triggers on `apps/webui/**` and the workflow file itself; that scoping is correct and should stay.
- `concurrency: group: "pages", cancel-in-progress: false` is correct and should stay.
- `permissions` (`pages: write`, `id-token: write`) are correct and should stay.

## Scope

**In scope**:

- `.github/workflows/deploy-pages.yml` (the `deploy` job only).

**Out of scope**:

- The `build` job, PWA config, Vite `base`, or any app code.
- Pages source/settings changes via the API (already correct).
- Migrating away from `actions/deploy-pages`.

## Git workflow

- Suggested branch: `advisor/010-harden-pages-deploy`
- Commit message style: `ci(pages): auto-retry transient deploy-pages failures`
- Do not push or open a PR unless instructed.

## Steps

### Step 1: Add bounded automatic retry to the deploy step

Wrap the `actions/deploy-pages@v5` step with a retry so a single transient
"Deployment failed, try again later" does not fail the run. Use a pinned,
widely-used retry action **or** an inline shell retry. Preferred: keep it simple
and dependency-light with `nick-fields/retry` pinned to a commit SHA, or an
inline approach that re-invokes the action. Because `actions/deploy-pages`
cannot be looped inside a single `uses:` step, implement retry by giving the
step `continue-on-error` on the first attempt and adding a second guarded
attempt. Concretely:

```yaml
  deploy:
    environment:
      name: github-pages
      url: ${{ steps.deployment.outputs.page_url }}
    runs-on: ubuntu-latest
    needs: build
    steps:
      - name: Deploy to GitHub Pages (attempt 1)
        id: deploy1
        continue-on-error: true
        uses: actions/deploy-pages@v5

      - name: Wait before retry
        if: steps.deploy1.outcome == 'failure'
        run: sleep 30

      - name: Deploy to GitHub Pages (attempt 2)
        id: deployment
        if: steps.deploy1.outcome == 'failure'
        uses: actions/deploy-pages@v5

      - name: Surface deploy URL
        if: steps.deploy1.outcome == 'success'
        run: echo "Deployed at ${{ steps.deploy1.outputs.page_url }}"
```

Ensure the `environment.url` still resolves. `actions/deploy-pages` sets
`page_url` on whichever attempt runs; if attempt 1 succeeds, `steps.deployment`
is skipped, so point `environment.url` at a value that exists in both paths — e.g.
change it to `${{ steps.deploy1.outputs.page_url || steps.deployment.outputs.page_url }}`.

**Verify**: workflow YAML is valid — `cd . && python3 -c "import yaml,sys; yaml.safe_load(open('.github/workflows/deploy-pages.yml'))" && echo OK` → prints `OK`.

### Step 2: Confirm the retry does not double-create deployments harmfully

`actions/deploy-pages` is idempotent per artifact within a run; a second attempt
re-polls / re-creates against the same `pages_build_version`. This is safe. Add a
short comment in the workflow above the deploy steps explaining the retry exists
to absorb transient `Deployment failed, try again later` errors, and referencing
this plan.

**Verify**: `git diff .github/workflows/deploy-pages.yml` shows only the deploy job changed plus the explanatory comment.

### Step 3: (Optional, only if the two-attempt pattern is rejected in review)

Alternative single-source approach: replace the two-step pattern with
`nick-fields/retry@<pinned-sha>` invoking a composite that calls the deploy.
Only pursue this if a reviewer objects to `continue-on-error`. Pin any new action
to a full commit SHA (repo convention: all third-party actions are SHA/version pinned).

## Test plan

- YAML lint passes (Step 1 verify).
- Trigger manually via `workflow_dispatch` (or merge to `main` with an `apps/webui/**` change) and confirm:
  - On a healthy backend, attempt 1 succeeds and attempt 2 is skipped.
  - The `environment.url` still points at `https://keroway.github.io/timeline-dsl/`.
- Because the failure is transient and backend-driven, you cannot deterministically force it; rely on the idempotency argument plus a successful real run.

## Done criteria

- [ ] `deploy` job retries once on a failed first attempt with a short backoff.
- [ ] `environment.url` resolves regardless of which attempt runs.
- [ ] Any new third-party action is SHA/version pinned.
- [ ] Workflow YAML parses cleanly.
- [ ] A real `workflow_dispatch` run goes green.
- [ ] `plans/README.md` status row updated.

## STOP conditions

Stop and report if:

- The live Pages config (`gh api repos/keroway/timeline-dsl/pages`) shows `build_type` is **not** `"workflow"` (would indicate a settings problem, not a transient error — needs a different fix: set Pages source to "GitHub Actions").
- The failure recurs on **every** run including re-runs (indicates a real regression — capture `gh run view <id> --log-failed` and report).
- Implementing retry would require unpinned third-party actions.

## Maintenance notes

If GitHub later ships native retry support in `actions/deploy-pages`, replace the
two-attempt shim with the official option. Track transient-failure frequency; if
it becomes common, consider raising the action's `timeout`/`error_count` inputs.
