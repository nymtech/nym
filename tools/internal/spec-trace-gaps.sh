#!/usr/bin/env bash
# Lists the gaps in the spec trace (spec-traceability) as Markdown, and exits 1 if it finds a gap:
# - each requirement that Tracey knows with no impl marker or no verify marker;
# - each requirement block with no r[id] line, in a spec that has r[id] lines. Tracey cannot see these blocks.
# The CI canary runs this script on planted gaps, and the CI gate runs it on the tree.
set -euo pipefail
cd "$(git rev-parse --show-toplevel)"

out=$(mktemp -d)
trap 'rm -rf "$out"' EXIT
for query in uncovered untested; do
  tracey query --json "$query" > "$out/$query.json"
done
gaps=$(jq -r '.bySection[].rules[].id.base' "$out/uncovered.json" "$out/untested.json" | sort -u)

no_id=""
traced_specs=$(grep -rl --include=spec.md '^r\[' openspec || true)
if [ -n "$traced_specs" ]; then
  # A requirement's r[id] line comes before its first scenario heading.
  no_id=$(echo "$traced_specs" | xargs awk '
    function flush() { if (req != "") print file ": " req; req = "" }
    FNR == 1 { flush(); file = FILENAME }
    /^### Requirement:/ { flush(); req = substr($0, 18); next }
    /^r\[/ { req = ""; next }
    /^#+ / { flush() }
    END { flush() }
  ')
fi

if [ -z "$gaps" ] && [ -z "$no_id" ]; then
  exit 0
fi
if [ -n "$gaps" ]; then
  echo "### Requirements with no impl marker or no verify marker"
  echo "$gaps" | sed 's/^/- /'
fi
if [ -n "$no_id" ]; then
  echo "### Requirements with no r[id] line"
  echo "$no_id" | sed 's/^/- /'
fi
exit 1
