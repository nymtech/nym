#!/usr/bin/env bash
#
# End-to-end check of a deployed docs MCP server. It runs the probe files in
# documentation/mcp-probes/ with the runner of @nymproject/retrieval-indexer
# (retrieval-mcp-check). The mcp-checks workflow of nymtech/retrieval-indexer
# runs the same files against nym.com each day.
#
# Usage:
#
#   ./check-mcp-server.sh <base-url> [vercel-bypass-token]
#
# <base-url> is the deployment root. A trailing /docs is stripped, so the URL
# from the browser works as-is:
#
#   ./check-mcp-server.sh https://nym.com
#   ./check-mcp-server.sh https://<preview-host> "$BYPASS"
#
# A Vercel preview sits behind Deployment Protection and needs its bypass token
# (Project Settings, Deployment Protection, Protection Bypass for Automation).
# Keep it out of shell history:
#
#   read -rs BYPASS && export BYPASS
#
# Requires node and `pnpm install` in documentation/docs. Exits non-zero if any
# check fails.

set -euo pipefail

BASE="${1:-}"
if [[ -z "$BASE" ]]; then
  echo "usage: $0 <base-url> [vercel-bypass-token]" >&2
  echo "  e.g. $0 https://nym.com" >&2
  echo "       $0 https://<preview-host> \"\$BYPASS\"" >&2
  exit 2
fi
BASE="${BASE%/}"
BASE="${BASE%/docs}"

DOCS_ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
BYPASS="${2:-${BYPASS:-}}" exec "$DOCS_ROOT/docs/node_modules/.bin/retrieval-mcp-check" \
  "$BASE/docs/api/mcp" "$DOCS_ROOT/mcp-probes/server.json" "$DOCS_ROOT/mcp-probes/retrieval.json"
