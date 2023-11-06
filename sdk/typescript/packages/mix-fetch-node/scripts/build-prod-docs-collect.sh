#!/usr/bin/env bash

set -o errexit
set -o nounset
set -o pipefail

rm -rf ../../../../dist/ts/docs/tsdoc/nymproject/mix-fetch-node || true

# run the build
yarn docs:generate:prod

# move the output outside of the yarn/npm workspaces
mkdir -p ../../../../dist/ts/docs/tsdoc/nymproject
mv docs ../../../../dist/ts/docs/tsdoc/nymproject/mix-fetch-node

echo "Output can be found in:"
realpath ../../../../dist/ts/docs/tsdoc/nymproject/mix-fetch-node
