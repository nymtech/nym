#!/usr/bin/env bash

set -o errexit
set -o nounset
set -o pipefail

# -------------------------------------------------------
# NB: Run this from the root of the repository.
# Each smolmix-based SDK package is published in place from
# its source directory after `pnpm build:ci:sdk` produces
# the dist/ output inside each package.
# -------------------------------------------------------

REPO_ROOT="$(git rev-parse --show-toplevel)"
cd "$REPO_ROOT"

packages=(
  "sdk/typescript/packages/mix-tunnel"
  "sdk/typescript/packages/mix-fetch"
  "sdk/typescript/packages/mix-dns"
  "sdk/typescript/packages/mix-websocket"
)

pushd () {
  command pushd "$@" > /dev/null
}

popd () {
  command popd > /dev/null
}

echo "Summary of versions to publish:"
echo
for item in "${packages[@]}"; do
  pushd "$item"
  jq -r '. | "  " + .version + "  " + .name' < package.json
  popd
done

echo
COUNTER=0
for item in "${packages[@]}"; do
  (( COUNTER+=1 ))
  pushd "$item"
  echo "Publishing $item  (${COUNTER}/${#packages[@]})"
  npm publish --access=public --verbose --workspaces false || true
  popd
  echo
done
echo
echo "Done."
