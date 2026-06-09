#!/bin/bash

# Generates TypeDoc markdown API reference for TypeScript SDK packages.
# Each package's typedoc.json sets its own `out` path under
# docs/pages/developers/ (sdk -> typescript/api/sdk, the rest -> <pkg>/api).
#
# Prerequisites: typedoc and typedoc-plugin-markdown must be installed globally
#   pnpm add -g typedoc@0.25.13 typedoc-plugin-markdown@4.0.3
#
# Usage: run from the documentation/ directory, or it will cd there automatically.

set -o errexit
set -o nounset
set -o pipefail

REPO_ROOT="$(cd "$(dirname "$0")/../../.." && pwd)"
SDK_PACKAGES="$REPO_ROOT/sdk/typescript/packages"

# packages to generate docs for (name = directory name under packages/).
# Keep in sync with the typedoc step in .github/workflows/ci-docs.yml.
PACKAGES=("sdk" "mix-tunnel" "mix-fetch" "mix-dns" "mix-websocket")

for pkg in "${PACKAGES[@]}"; do
  echo "Generating TypeDoc for @nymproject/${pkg}..."
  cd "$SDK_PACKAGES/$pkg"
  typedoc --skipErrorChecking
done

# typedoc-plugin-markdown does not emit Nextra sidebar metadata; regenerate the
# _meta.json files from the markdown output so they can't drift or be wiped.
echo "Generating _meta.json for TypeDoc output..."
node "$REPO_ROOT/documentation/scripts/next-scripts/generate-typedoc-meta.mjs"

echo "TypeDoc generation complete."
echo "Output: documentation/docs/pages/developers/{typescript/api/sdk,<pkg>/api}/"
