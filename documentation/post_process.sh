#!/usr/bin/env bash

set -o errexit
set -o nounset
set -o pipefail

# this is a script called by the github CI and CD workflows to post process CSS/image/href links for serving
# several mdbooks from a subdirectory

cd scripts/post-process
npm install
node index.mjs
