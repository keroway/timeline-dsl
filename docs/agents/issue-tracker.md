# Issue tracker: GitHub

Issues and specs for this repo live as GitHub issues in `keroway/timeline-dsl`. Use the `gh` CLI for all operations.

## Conventions

- **Create an issue**: `gh issue create --title "..." --body "..."`. Use a heredoc for multi-line bodies.
- **Read an issue**: `gh issue view <number> --comments`, filtering comments by `jq` and also fetching labels.
- **List issues**: `gh issue list --state open --json number,title,body,labels,comments --jq '[.[] | {number, title, body, labels: [.labels[].name], comments: [.comments[].body]}]'` with appropriate `--label` and `--state` filters.
- **Comment on an issue**: `gh issue comment <number> --body "..."`
- **Apply / remove labels**: `gh issue edit <number> --add-label "..."` / `--remove-label "..."`
- **Close**: `gh issue close <number> --comment "..."`

Infer the repo from `git remote -v`; `gh` does this automatically when run inside a clone.

## Labels and lifecycle

The triage roles and the exclusion list the unmanned loops gate on are in
[`triage-labels.md`](triage-labels.md). Beyond those, this repo uses:

- **`area:*`** (`area:testing` / `area:editor` / `area:webui`): which layer the work
  lands in. `CLAUDE.md` の「クレート構成」に依存方向があるので、`area:` を付けるときは
  変更が波及するクレートの側に寄せる。
- **`dependencies`**: バージョン追従の Renovate（`.github/renovate.json5`）が付ける
  （2026-07-24 に dependabot.yml から移行済み。`rust` / `javascript` / `github_actions`
  のような生態系別ラベルは付与していない）。脆弱性由来の Dependabot security update PR
  には `dependencies` は付かず、`chore(deps): bump ...` の PR タイトルで判別する。

An Issue moves `needs-refinement` → (split by `issue-refinement apply`) →
`ready-for-agent`（スコープ確定・受け入れ条件定義済み）→ `in-progress`
(claimed by `issue-sprint`) → closed by its PR.

`.github/agent-automation.yml` has `enabled: true` and `allow_merge: true`, so anything
written here is read by headless sessions that can merge their own PRs, not only by
interactive ones.

DSL 文法に触る issue は、着手前に `CLAUDE.md` の「DSL文法の変更手順」を読むこと。
**シンタックスハイライトのキーワード更新**が手順に含まれており、受け入れ条件から
落ちやすい。

## Pull requests as a triage surface

**PRs as a request surface: no.** _(Set to `yes` if this repo treats external PRs as feature requests; `/triage` reads this flag.)_

Single-maintainer repo with no external contributors; PRs are outputs of the issue
lifecycle, not inputs to it.

## When a skill says "publish to the issue tracker"

Create a GitHub issue.

## When a skill says "fetch the relevant ticket"

Run `gh issue view <number> --comments`.

## Wayfinding operations

`/wayfinder` is not used in this workspace, so no map/child-ticket convention is defined
here. If it is ever adopted, the seed template in the `setup-matt-pocock-skills` skill
folder carries the GitHub sub-issue + native-dependency recipe to start from.
