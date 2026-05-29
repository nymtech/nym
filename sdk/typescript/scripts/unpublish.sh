#!/usr/bin/env bash

set -o errexit
set -o nounset
set -o pipefail

cd dist

packages=(
"ts/sdk/nodejs-client/cjs"
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

for item in "${packages[@]}"
do
  pushd "$item"
  echo "🚀 Publishing $item..."
  VERSION_SPEC=$(cat package.json | jq -r '. | .name + "@" +.version')
  npm publish --access=public
  popd
  echo ""
done
echo ""
echo "✅ Done"
