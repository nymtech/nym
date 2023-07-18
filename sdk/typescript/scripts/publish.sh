#!/usr/bin/env bash

set -o errexit
set -o nounset
set -o pipefail

# -------------------------------------------------------
# ✅ NB: Run this from the root of the repository
# -------------------------------------------------------

cd dist

packages=(
chat-app/parcel
chat-app/plain-html
chat-app/react-webpack-with-theme-example
chrome-extension
firefox-extension
node-tester/parcel
node-tester/plain-html
node-tester/react
react/mui-theme
react/sdk-react
)

pushd () {
    command pushd "$@" > /dev/null
}

popd () {
    command popd "$@" > /dev/null
}

echo "Summary of versions of packages to publish:"
echo ""
for item in "${packages[@]}"
do
  pushd "$item"
  cat package.json | jq -r '. | "📦 " + .version + "   " +.name'
  popd
done

echo ""
echo ""

for item in "${packages[@]}"
do
  pushd "$item"
  echo "🚀 Publishing $item..."
  cat package.json | jq -r '. | .name + " " +.version'
  npm publish --access=public
  popd
  echo ""
done
echo ""
echo "✅ Done"

