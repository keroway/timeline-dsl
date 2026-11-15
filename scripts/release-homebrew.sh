#!/usr/bin/env bash
set -euo pipefail
: "${TARGET_TAG:?verified release tag is required}"
BRANCH="bump-tdsl-${TARGET_TAG#v}"
# Reuse an existing branch (including an interrupted run before PR creation).
# Never rewrite history or force-push a user's/another run's branch.
git config user.name 'github-actions[bot]'
git config user.email 'github-actions[bot]@users.noreply.github.com'
expected=$(mktemp)
trap 'rm -f "$expected"' EXIT
cp Formula/tdsl.rb "$expected"
remote=$(git ls-remote --heads origin "refs/heads/$BRANCH")
if [ -n "$remote" ]; then
  git fetch origin "$BRANCH"
  git show FETCH_HEAD:Formula/tdsl.rb > Formula/tdsl.rb
  if ! cmp -s "$expected" Formula/tdsl.rb; then
    echo 'Existing Homebrew branch differs; manual review required.' >&2
    exit 1
  fi
else
  git switch -c "$BRANCH"
  git add Formula/tdsl.rb
  if git diff --cached --quiet; then exit 0; fi
  git commit -m "chore: update tdsl formula to ${TARGET_TAG}"
  git push origin "$BRANCH"
fi
existing=$(gh pr list --repo keroway/homebrew-tap --head "$BRANCH" --state open --json url --jq '.[0].url // empty')
if [ -n "$existing" ]; then
  echo "Reusing $existing"
else
  gh pr create --repo keroway/homebrew-tap --head "$BRANCH" \
    --title "chore: update tdsl formula to ${TARGET_TAG}" \
    --body "Verified archive checksums from keroway/timeline-dsl ${TARGET_TAG}. Merge is required before this version is available from the tap."
fi
