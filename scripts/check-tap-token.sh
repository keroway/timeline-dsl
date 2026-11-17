#!/usr/bin/env bash
set -euo pipefail
: "${GH_TOKEN:?TAP_BUMP_TOKEN is required}"
: "${GITHUB_RUN_ID:?run ID is required}"
: "${GITHUB_RUN_ATTEMPT:?run attempt is required}"
case "$GITHUB_RUN_ID$GITHUB_RUN_ATTEMPT" in
  *[!0-9]*) echo 'Invalid run identity' >&2; exit 1 ;;
esac
REPO=keroway/homebrew-tap
BRANCH="token-check/tdsl-${GITHUB_RUN_ID}-${GITHUB_RUN_ATTEMPT}"
remote=$(git remote get-url origin)
case "$remote" in
  "https://github.com/$REPO"|"https://github.com/$REPO.git") ;;
  *) echo 'Unexpected tap remote' >&2; exit 1 ;;
esac
status=$(git status --porcelain)
if [ -n "$status" ]; then echo 'Tap checkout is dirty' >&2; exit 1; fi
existing=$(git ls-remote --heads origin "refs/heads/$BRANCH")
if [ -n "$existing" ]; then echo 'Probe branch already exists' >&2; exit 1; fi
BASE=$(gh repo view "$REPO" --json defaultBranchRef --jq '.defaultBranchRef.name')
test -n "$BASE"
PUSHED=false
cleanup() {
  result=$?
  trap - EXIT
  failed=false
  # Discover by our unique head even if PR creation succeeded but its response was lost.
  if [ "$PUSHED" = true ]; then
    if prs=$(gh api "repos/$REPO/pulls" --method GET -f "head=keroway:$BRANCH" -f state=open --jq '.[].number'); then
      while IFS= read -r pr; do
        if [ -z "$pr" ]; then continue; fi
        case "$pr" in
          *[!0-9]*) failed=true; continue ;;
        esac
        if ! gh pr close "$pr" --repo "$REPO"; then
          echo "Token check cleanup failed: close $REPO PR $pr" >&2
          failed=true
        fi
      done <<< "$prs"
    else
      echo "Token check cleanup uncertain: inspect PRs for $BRANCH in $REPO" >&2
      failed=true
    fi
  fi
  # Never delete the branch while a known probe PR could still be open.
  if [ "$PUSHED" = true ] && [ "$failed" = false ]; then
    if ! git push origin --delete "$BRANCH"; then
      echo "Token check cleanup failed: delete $BRANCH in $REPO" >&2
      failed=true
    fi
  fi
  if [ "$failed" = true ]; then
    echo "Manual cleanup required: inspect $BRANCH in $REPO" >&2
    result=1
  fi
  if [ "$result" = 0 ]; then
    echo 'TAP_BUMP_TOKEN verified: push, draft PR creation/closure and branch deletion.'
  fi
  exit "$result"
}
trap cleanup EXIT
trap 'exit 130' INT TERM
git config user.name 'github-actions[bot]'
git config user.email 'github-actions[bot]@users.noreply.github.com'
git switch -c "$BRANCH"
mkdir -p .token-check
printf 'TAP_BUMP_TOKEN verification run %s attempt %s\n' "$GITHUB_RUN_ID" "$GITHUB_RUN_ATTEMPT" > .token-check/tdsl.txt
git add .token-check/tdsl.txt
git commit -m 'chore: verify TAP_BUMP_TOKEN [skip ci]'
git push origin "HEAD:refs/heads/$BRANCH"
PUSHED=true
gh pr create --repo "$REPO" --head "$BRANCH" --base "$BASE" --draft \
  --title 'chore: verify TAP_BUMP_TOKEN (temporary; do not merge)' \
  --body 'Automated permission check from timeline-dsl. This draft PR is closed and its branch deleted by the same run. No Formula or release changes; do not merge.'
