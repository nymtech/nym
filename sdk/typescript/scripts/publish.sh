#!/usr/bin/env bash

set -o errexit
set -o nounset
set -o pipefail

# -------------------------------------------------------
# ✅ NB: Run this from the root of the repository
# -------------------------------------------------------

cd dist

#packages=(
#chat-app/parcel
#chat-app/plain-html
#chat-app/react-webpack-with-theme-example
#chrome-extension
#firefox-extension
#node-tester/parcel
#node-tester/plain-html
#node-tester/react
#react/mui-theme
#react/sdk-react
#)
packages=(
"wasm/client"
"wasm/mix-fetch"
"wasm/node-tester"
"wasm/extension-storage"

"node/wasm/client"
"node/wasm/mix-fetch"

"ts/sdk/mix-fetch/cjs"
"ts/sdk/mix-fetch/cjs-full-fat"
"ts/sdk/mix-fetch/esm"
"ts/sdk/mix-fetch/esm-full-fat"

"ts/sdk/nodejs-client/cjs"
"ts/sdk/mix-fetch-node/cjs"

"ts/sdk/node-tester/cjs"
"ts/sdk/node-tester/cjs-full-fat"
"ts/sdk/node-tester/esm"
"ts/sdk/node-tester/esm-full-fat"

"ts/sdk/sdk/cjs"
"ts/sdk/sdk/cjs-full-fat"
"ts/sdk/sdk/esm"
"ts/sdk/sdk/esm-full-fat"

"ts/sdk/contract-clients"
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
  (( COUNTER+=1 ))
  pushd "$item"
  echo "🚀 Publishing $item... (${COUNTER} of ${#packages[@]})"
  cat package.json | jq -r '. | .name + " " +.version'
  npm publish --access=public --verbose --workspaces false || true
  popd
  echo ""
done
echo ""
echo "✅ Done"
