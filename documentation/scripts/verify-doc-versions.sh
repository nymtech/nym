#!/usr/bin/env bash
# Verify that hardcoded crate versions in MDX fenced code blocks
# match the canonical constants in components/versions.ts.
#
# Usage: ./scripts/verify-doc-versions.sh
# Returns: 0 if all versions match, 1 if any mismatch is found.

set -euo pipefail

DOCS_DIR="$(cd "$(dirname "$0")/../docs" && pwd)"
VERSIONS_FILE="$DOCS_DIR/components/versions.ts"

# Extract canonical versions from versions.ts
sdk_version=$(sed -n 's/.*NYM_SDK_VERSION *= *"\([^"]*\)".*/\1/p' "$VERSIONS_FILE")
smolmix_version=$(sed -n 's/.*SMOLMIX_VERSION *= *"\([^"]*\)".*/\1/p' "$VERSIONS_FILE")

echo "Canonical versions:"
echo "  NYM_SDK_VERSION  = $sdk_version"
echo "  SMOLMIX_VERSION  = $smolmix_version"
echo ""

errors=0

# Check a grep match against an expected version.
# Args: file:linenum:content expected_version
check_line() {
    local match="$1"
    local expected="$2"
    local file="${match%%:*}"
    local rest="${match#*:}"
    local linenum="${rest%%:*}"
    local content="${rest#*:}"
    local found
    found=$(echo "$content" | sed -n 's/.*"\([0-9]\+\.[0-9]\+\.[0-9]\+\)".*/\1/p')

    if [ -n "$found" ] && [ "$found" != "$expected" ]; then
        echo "MISMATCH: $file:$linenum"
        echo "  found:    $found"
        echo "  expected: $expected"
        echo "  line:     $content"
        echo ""
        errors=$((errors + 1))
    fi
}

# Crates that should track NYM_SDK_VERSION
while IFS= read -r match; do
    check_line "$match" "$sdk_version"
done < <(grep -rn --include='*.mdx' -E '(nym-sdk|nym-bin-common|nym-network-defaults)\s*=' "$DOCS_DIR/pages")

# smolmix version
while IFS= read -r match; do
    check_line "$match" "$smolmix_version"
done < <(grep -rn --include='*.mdx' -E 'smolmix\s*=' "$DOCS_DIR/pages")

if [ "$errors" -gt 0 ]; then
    echo "FAILED: $errors version mismatch(es) found."
    echo "Update the hardcoded versions or bump components/versions.ts."
    exit 1
else
    echo "OK: all hardcoded versions match."
fi
