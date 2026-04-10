#!/bin/bash

# Generates TypeDoc markdown API reference for TypeScript SDK packages.
# Output goes into docs/pages/developers/typescript-sdk/api/
#
# Prerequisites: typedoc and typedoc-plugin-markdown must be installed globally
#   pnpm add -g typedoc@0.25.13 typedoc-plugin-markdown@4.0.3
#
# Usage: run from the documentation/ directory, or it will cd there automatically.

set -o errexit
set -o nounset
set -o pipefail

REPO_ROOT="$(cd "$(dirname "$0")/../.." && pwd)"
SDK_PACKAGES="$REPO_ROOT/sdk/typescript/packages"

# packages to generate docs for (name = directory name under packages/)
PACKAGES=("sdk" "mix-fetch")

for pkg in "${PACKAGES[@]}"; do
  echo "Generating TypeDoc for @nymproject/${pkg}..."
  cd "$SDK_PACKAGES/$pkg"
  typedoc --skipErrorChecking
done

echo "TypeDoc generation complete."
echo "Output: documentation/docs/pages/developers/typescript-sdk/api/"
