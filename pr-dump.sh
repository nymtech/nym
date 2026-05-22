#!/usr/bin/env bash
# Dump a GitHub PR's overview + three comment streams as JSON files.
#
# Usage: pr-dump.sh <github-pr-url> [output-dir]
#   output-dir defaults to ./pr-<number>

set -euo pipefail

usage() {
  echo "Usage: $(basename "$0") <github-pr-url> [output-dir]" >&2
  echo "  e.g. $(basename "$0") https://github.com/nymtech/nym/pull/6784" >&2
  exit 1
}

[[ $# -ge 1 ]] || usage

url="$1"
if [[ ! "$url" =~ ^https://github\.com/([^/]+)/([^/]+)/pull/([0-9]+) ]]; then
  echo "Not a GitHub PR URL: $url" >&2
  exit 1
fi

owner="${BASH_REMATCH[1]}"
repo="${BASH_REMATCH[2]}"
number="${BASH_REMATCH[3]}"
slug="$owner/$repo"

outdir="${2:-./pr-$number}"
mkdir -p "$outdir"
cd "$outdir"

fields="number,title,body,state,author,createdAt,updatedAt,headRefName,baseRefName,url,isDraft,mergeable,reviewDecision,labels,additions,deletions,changedFiles,files,commits"

count() {
  if command -v jq >/dev/null; then
    jq length "$1"
  else
    echo '?'
  fi
}

echo "Dumping $slug PR #$number into $(pwd)"

gh pr view "$number" --repo "$slug" --json "$fields" > overview.json
echo "  overview.json"

gh api --paginate -X GET "repos/$slug/pulls/$number/comments" > inline-comments.json
echo "  inline-comments.json ($(count inline-comments.json) items)"

gh api --paginate -X GET "repos/$slug/issues/$number/comments" > conversation-comments.json
echo "  conversation-comments.json ($(count conversation-comments.json) items)"

gh api --paginate -X GET "repos/$slug/pulls/$number/reviews" > reviews.json
echo "  reviews.json ($(count reviews.json) items)"

echo "Done."
