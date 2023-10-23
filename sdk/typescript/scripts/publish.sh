#!/usr/bin/env bash

set -o errexit
set -o nounset
set -o pipefail

# -------------------------------------------------------
# ✅ NB: Run this from the root of the repository
# -------------------------------------------------------

cd dist

packages=(
"ts/sdk/nodejs-client/cjs"
"ts/sdk/mix-fetch-node/cjs"
)

pushd () {
    command pushd "$@" > /dev/null
}

popd () {
    command popd > /dev/null
}

echo "Summary of versions of packages to publish:"
echo ""
for item in "${packages[@]}"
do
  pushd "$item"
  jq -r '. | "📦 " + .version + "   " +.name' < package.json
  popd 
done

echo ""
echo ""

COUNTER=0

for item in "${packages[@]}"
do
  (( COUNTER++ ))
  pushd "$item"
  echo "🚀 Publishing $item... (${COUNTER} of ${#packages[@]})"
  jq -r '. | .name + " " +.version' < package.json
  npm publish --access=public --verbose
  popd
  echo ""
done
echo ""
echo "✅ Done"

