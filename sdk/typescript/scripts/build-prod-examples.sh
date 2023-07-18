#!/usr/bin/env bash

set -o errexit
set -o nounset
set -o pipefail

rm -rf dist/ts/examples || true
mkdir -p dist/ts/examples

##---------------------------------------------------------------------------
## ✅ Run this script from the root of the repository using `yarn sdk:build`
##---------------------------------------------------------------------------

packages=(
"chat-app/parcel"
"chat-app/plain-html"
"chat-app/react-webpack-with-theme-example"

"chrome-extension"
"firefox-extension"

"node-tester/parcel"
"node-tester/plain-html"
"node-tester/react"

"react/mui-theme"
"react/sdk-react"
)

pushd () {
    command pushd "$@" > /dev/null
}

popd () {
    command popd "$@" > /dev/null
}

echo "Summary of versions of examples to build:"
echo ""
pushd "sdk/typescript/examples"
for item in "${packages[@]}"
do
  pushd "$item"
  cat package.json | jq -r '. | "📦 " + .version + "   " +.name'
  popd
done
