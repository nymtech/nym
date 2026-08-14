#!/usr/bin/env bash
#
# End-to-end check of a deployed docs MCP server. Everything here runs over HTTP
# against a real deployment, so it covers what the unit tests cannot: that the
# index was traced into the lambda, that the key is present in that environment,
# that the transport negotiates, and that the live Nym API still returns the
# field names the tools read.
#
# Run it after any deploy that touches retrieval, the tool registry or the build
# pipeline. It is the fastest way to tell a broken deployment from a broken
# branch.
#
# Usage:
#
#   ./check-mcp-server.sh <base-url> [vercel-bypass-token]
#
# <base-url> is the deployment root. A trailing /docs is stripped, so the URL
# from the browser works as-is:
#
#   ./check-mcp-server.sh https://nym.com
#   ./check-mcp-server.sh https://docs-nextra-git-my-branch.vercel.app "$BYPASS"
#
# The bypass token is only needed for Vercel preview deployments, which sit
# behind Deployment Protection. Without it every request is answered with an
# HTML login page and all checks fail. Production needs no token.
#
# Take the value from Vercel: Project Settings -> Deployment Protection ->
# Protection Bypass for Automation. It is a shared secret, so keep it out of
# shell history and out of this repo:
#
#   read -rs BYPASS && export BYPASS
#
# It can also be exported as $BYPASS instead of passed as the second argument.
#
# Requires curl and jq. Exits non-zero if any check fails, so it can gate a
# deploy step.

set -uo pipefail

BASE="${1:-}"
# Second argument wins, then the environment, so a token can be exported once
# and reused across runs without landing in shell history.
BYPASS="${2:-${BYPASS:-}}"

if [[ -z "$BASE" ]]; then
  echo "usage: $0 <base-url> [vercel-bypass-token]" >&2
  echo "  e.g. $0 https://nym.com" >&2
  echo "       $0 https://docs-nextra-git-my-branch.vercel.app \"\$BYPASS\"" >&2
  echo "  the token is only needed for protected Vercel previews;" >&2
  echo "  it may also be exported as \$BYPASS." >&2
  exit 2
fi

BASE="${BASE%/}"
BASE="${BASE%/docs}"
MCP_URL="$BASE/docs/api/mcp"

HDRS=(-H 'Content-Type: application/json' -H 'Accept: application/json, text/event-stream')
[[ -n "$BYPASS" ]] && HDRS+=(-H "x-vercel-protection-bypass: $BYPASS")

PASS=0
FAIL=0
SKIP=0

ok()   { printf '  \033[32mPASS\033[0m  %s\n' "$1"; PASS=$((PASS + 1)); }
bad()  { printf '  \033[31mFAIL\033[0m  %s\n' "$1"; FAIL=$((FAIL + 1)); }
skip() { printf '  \033[33mSKIP\033[0m  %s\n' "$1"; SKIP=$((SKIP + 1)); }
head_() { printf '\n\033[1m%s\033[0m\n' "$1"; }

# Repo root, for checks that compare the deployment against the source tree.
REPO_ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)"

# POST a JSON-RPC call and print the decoded `data:` line from the SSE frame.
rpc() {
  curl -sS -X POST "$MCP_URL" "${HDRS[@]}" -d "$1" | sed -n 's/^data: //p'
}

# Call one tool and return its first text content block.
call() {
  local name="$1" args="$2"
  rpc "$(jq -nc --arg n "$name" --argjson a "$args" \
    '{jsonrpc:"2.0",id:1,method:"tools/call",params:{name:$n,arguments:$a}}')" \
    | jq -r '.result.content[0].text // "<<no text>>"'
}

# Assert a tool's output matches a regex.
expect() {
  local label="$1" name="$2" args="$3" pattern="$4"
  local out
  out="$(call "$name" "$args")"
  if [[ "$out" =~ $pattern ]]; then
    ok "$label"
  else
    bad "$label"
    printf '        wanted /%s/, got: %.180s\n' "$pattern" "${out//$'\n'/ }"
  fi
}

echo "MCP endpoint: $MCP_URL"
[[ -z "$BYPASS" ]] && echo "(no bypass token; expect HTML login pages if the deployment is protected)"

# --- protocol ---------------------------------------------------------------
head_ "Protocol"

TOOLS_JSON="$(rpc '{"jsonrpc":"2.0","id":1,"method":"tools/list"}')"
TOOL_NAMES="$(jq -r '[.result.tools[].name] | sort | join(",")' <<<"$TOOLS_JSON" 2>/dev/null)"
EXPECTED="chain_status,circulating_supply,get_gateway,get_section,list_gateways,network_summary,search_code,search_docs,validate_sdk_config"

if [[ "$TOOL_NAMES" == "$EXPECTED" ]]; then
  ok "tools/list returns all 9 tools"
else
  bad "tools/list tool set"
  echo "        expected: $EXPECTED"
  echo "        got:      ${TOOL_NAMES:-<<unparseable, is the deployment protected?>>}"
fi

# Every tool needs a description; an agent picks tools by reading them.
UNDOC="$(jq -r '[.result.tools[] | select((.description // "") | length < 20) | .name] | join(",")' <<<"$TOOLS_JSON" 2>/dev/null)"
[[ -z "$UNDOC" ]] && ok "every tool carries a usable description" || bad "thin descriptions: $UNDOC"

# The SDK requires both Accept types; one alone is a 406.
CODE="$(curl -sS -o /dev/null -w '%{http_code}' -X POST "$MCP_URL" \
  -H 'Content-Type: application/json' -H 'Accept: application/json' \
  ${BYPASS:+-H "x-vercel-protection-bypass: $BYPASS"} \
  -d '{"jsonrpc":"2.0","id":1,"method":"tools/list"}')"
[[ "$CODE" == "406" ]] && ok "single Accept header is rejected (406)" || bad "single Accept header gave $CODE, wanted 406"

CODE="$(curl -sS -o /dev/null -w '%{http_code}' "$MCP_URL" \
  ${BYPASS:+-H "x-vercel-protection-bypass: $BYPASS"})"
[[ "$CODE" == "405" ]] && ok "GET is rejected (405, stateless mode has no sessions)" || bad "GET gave $CODE, wanted 405"

# --- retrieval --------------------------------------------------------------
head_ "Retrieval"

DOCS_OUT="$(call search_docs '{"query":"Who is L2 and why does it matter?","topK":5}')"
if grep -q 'threat-model/actors' <<<"$DOCS_OUT"; then
  ok "search_docs ranks the threat-model actors page for a short jargon query"
else
  bad "search_docs missed threat-model/actors"
  printf '        got: %.180s\n' "${DOCS_OUT//$'\n'/ }"
fi

expect "search_docs finds SOCKS5 client setup" \
  search_docs '{"query":"run a socks5 client and point curl at it","topK":5}' 'socks5|SOCKS5'

# No floor here by design: the calling agent judges relevance itself.
OFF="$(call search_docs '{"query":"What is the capital of France?","topK":3}')"
if [[ "$OFF" == *"No documentation matched"* ]]; then
  echo "  NOTE  off-topic query returned nothing (a floor is being applied)"
else
  ok "off-topic query still returns hits (MCP applies no floor, by design)"
fi

# Round-trip a URL from search_docs back through get_section.
REF="$(grep -om1 'https://[^ ]*#[^ ]*' <<<"$DOCS_OUT")"
if [[ -n "$REF" ]]; then
  SECTION="$(call get_section "$(jq -nc --arg r "$REF" '{ref:$r}')")"
  if [[ "$SECTION" == *"No section found"* ]]; then
    bad "get_section round-trip failed for $REF"
  else
    ok "get_section round-trips a URL from search_docs"
  fi
else
  bad "no anchored URL in search_docs output to round-trip"
fi

# Phrased the way a developer would ask it, with none of the page's own words in
# the query: this checks the page is reachable from the intent, not from its title.
expect "search_docs reaches the service-provider page from a build-it question" \
  search_docs '{"query":"how do I write a backend that receives requests over the mixnet and replies","topK":5}' \
  'developers/service-providers'

expect "get_section reports a miss legibly" \
  get_section '{"ref":"not-a-real-chunk-id"}' 'No section found'

expect "get_section handles an empty ref" \
  get_section '{"ref":""}' 'No section found'

# --- code search ------------------------------------------------------------
head_ "Code search"

CODE_OUT="$(call search_code '{"query":"MixnetClient connect_new","topK":5}')"
if grep -q 'connect_new' <<<"$CODE_OUT" && grep -q 'github.com/nymtech/nym/blob' <<<"$CODE_OUT"; then
  ok "search_code finds connect_new with a GitHub deep link"
else
  bad "search_code did not return connect_new with a deep link"
  printf '        got: %.180s\n' "${CODE_OUT//$'\n'/ }"
fi

# Symbol-name search is the case the path+symbol embedding prefix was added for.
expect "search_code resolves a bare type name" \
  search_code '{"query":"NetworkRequesterSelector","topK":5}' 'NetworkRequesterSelector'

# --- index coverage ---------------------------------------------------------
head_ "Index coverage"

# The docs make claims about these crates, so search_code has to be able to cite
# them. Each check asserts a hit whose path is inside the expected root, not just
# a mention of the term, because prose elsewhere in the corpus discusses all of
# these by name.
#
# A failure here usually means the index is stale rather than the tool is broken:
# ROOTS in generate-code-index.mjs was widened after the deployment under test was
# built. Rebuild with VOYAGE_API_KEY set and redeploy.
#
# A root missing from the source tree is skipped rather than failed. The index is
# built from whichever branch the deployment was built from, so a crate that has
# not reached that branch yet cannot be in it, and asserting otherwise tests the
# branch's merge state rather than the retrieval pipeline. The local checkout is a
# proxy for the deployed branch, which is close enough to be useful and worth
# remembering when the two diverge.
covers() {
  local label="$1" query="$2" pathfrag="$3"
  local out
  if [[ ! -d "$REPO_ROOT/${pathfrag%/}" ]]; then
    skip "$label (${pathfrag%/} is not in this checkout)"
    return
  fi
  out="$(call search_code "$(jq -nc --arg q "$query" '{query:$q,topK:8}')")"
  if grep -qE "blob/[^ ]*$pathfrag" <<<"$out"; then
    ok "$label"
  else
    bad "$label (no hit under $pathfrag)"
    printf '        paths returned: %s\n' \
      "$(grep -oE 'blob/develop/[^ ]+' <<<"$out" | sed 's|blob/develop/||' | head -4 | tr '\n' ' ')"
  fi
}

# Prefer symbols over prose for these. A description of what a crate does competes
# with every other crate that does something similar, and the field keeps growing:
# an earlier query using "chunk" lost to common/nymsphinx/chunking, and its
# replacement, "checkpoint snapping and start jitter to obfuscate a resume
# height", later lost to smoldvpn/examples/zcash-sync.rs once that arrived. Both
# times the crate was fully indexed and the query had simply stopped
# discriminating. A symbol only one crate defines does not decay that way.
covers "sdk/rust: nym-swizzle start obfuscation" \
  "obfuscated_start snap grid floor jitter" 'sdk/rust/nym-swizzle'
covers "smolmix: the userspace TCP/UDP tunnel" \
  "Tunnel new_with_ipr get_best_ipr from_stream" 'smolmix/'
covers "smoldvpn: the WireGuard datapath" \
  "wireguard peer configuration and tunnel engine setup" 'smoldvpn/'
covers "common/nymsphinx: packet construction" \
  "sphinx packet header construction and layer encryption" 'common/nymsphinx'
covers "common/client-core: the client internals" \
  "client topology refresh and gateway selection" 'common/client-core'
covers "service-providers: the IP packet router" \
  "ip packet router exit policy and tunnel address allocation" 'service-providers/ip-packet-router'
covers "service-providers: the network requester" \
  "network requester socks5 connect request handling" 'service-providers/network-requester'
covers "clients: the native and socks5 clients" \
  "socks5 client listener accepting a connection" 'clients/'
covers "common/gateway-requests: the gateway protocol" \
  "gateway client authentication and registration handshake" 'common/gateway-requests'
covers "common/credentials: bandwidth credentials" \
  "bandwidth credential issuance and verification" 'common/credentials'
covers "nym-node: the node implementation" \
  "nym-node configuration and mode selection on startup" 'nym-node/'

# --- config validation ------------------------------------------------------
head_ "Config validation"

expect "type mismatch is an error" \
  validate_sdk_config '{"config":{"preferredIpr":123}}' 'should be string'

expect "typo is a warning with a suggestion" \
  validate_sdk_config '{"config":{"disableCoverTrafic":true}}' 'Did you mean'

expect "a clean config passes" \
  validate_sdk_config '{"config":{}}' 'valid'

# --- live network data ------------------------------------------------------
head_ "Live network data"

expect "network_summary returns counts" \
  network_summary '{}' '[0-9]+ total, [0-9]+ gateways, [0-9]+ mixnodes'

# The counts do not sum (80 entry + 100 exit is not 603 gateways, and gateways +
# mixnodes is not the total), so the output has to say so or an agent will do the
# arithmetic and report a number that is wrong.
expect "network_summary warns that the counts do not sum" \
  network_summary '{}' 'do not sum'

expect "circulating_supply returns NYM figures" \
  circulating_supply '{}' 'Circulating: [0-9,]+'

expect "chain_status names the connected nyxd" \
  chain_status '{}' 'Connected nyxd: http'

expect "list_gateways paginates" \
  list_gateways '{"size":3}' 'gateways total'

# Nothing should reach the caller as undefined or NaN; that means a field rename.
for t in network_summary circulating_supply chain_status; do
  OUT="$(call "$t" '{}')"
  if grep -qE 'undefined|NaN' <<<"$OUT"; then
    bad "$t leaked undefined/NaN (upstream field rename?)"
    printf '        %.180s\n' "${OUT//$'\n'/ }"
  else
    ok "$t has no undefined/NaN fields"
  fi
done

# --- error handling ---------------------------------------------------------
head_ "Error handling"

is_error() {
  rpc "$(jq -nc --arg n "$1" --argjson a "$2" \
    '{jsonrpc:"2.0",id:1,method:"tools/call",params:{name:$n,arguments:$a}}')" \
    | jq -r '.result.isError // false'
}

[[ "$(is_error no_such_tool '{}')" == "true" ]] \
  && ok "unknown tool is an error result" || bad "unknown tool did not set isError"

[[ "$(is_error get_gateway '{"identity":"NOT_A_REAL_IDENTITY"}')" == "true" ]] \
  && ok "bad gateway identity is an error result" || bad "bad gateway identity did not set isError"

# Wrong argument type must be caught by the schema, before it reaches the embedder.
expect "wrong argument type is rejected at the schema" \
  search_docs '{"query":123}' 'Invalid arguments'

# Missing required argument, likewise.
expect "missing required argument is rejected" \
  search_docs '{}' 'Invalid arguments'


# --- summary ----------------------------------------------------------------
# Skips do not fail the run: they mean a check could not apply to this checkout,
# not that the deployment is wrong.
if [[ "$SKIP" -gt 0 ]]; then
  printf '\n\033[1m%d passed, %d failed, %d skipped\033[0m\n' "$PASS" "$FAIL" "$SKIP"
else
  printf '\n\033[1m%d passed, %d failed\033[0m\n' "$PASS" "$FAIL"
fi
[[ "$FAIL" -eq 0 ]]
