#!/usr/bin/env bash

set -o errexit
set -o nounset
set -o pipefail

rm -rf ../../../dist/ts/docs/sdk/typescript || true

# run the build
npm run build

# move the output outside of the yarn/npm workspaces
mkdir -p ../../../dist/ts/docs/sdk
mv out ../../../dist/ts/docs/sdk/typescript

echo "Output can be found in:"
realpath ../../../dist/ts/docs/sdk/typescript
