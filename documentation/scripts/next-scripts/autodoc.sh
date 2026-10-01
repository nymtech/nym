#!/bin/bash

# Run by `generate:commands` in docs/package.json, and by the docs-autodoc CI workflow.
# Invoked from documentation/docs.
#
# Builds the three binaries whose --help and build-info the docs embed (nym-node, nym-api,
# nymvisor) in debug, captures them with the autodoc crate, and copies the markdown into
# the command-outputs directory that pages import. The CI workflow owns the commit; this
# script does not touch git.

set -o errexit
set -o nounset
set -o pipefail

# repo root: build only the three needed packages, in debug. The help and build-info text
# is the same as release, and debug is faster.
cd ../../
cargo build -p nym-node -p nym-api -p nymvisor

cd documentation/autodoc/
cargo run
mv autodoc-generated-markdown/commands/* ../docs/components/outputs/command-outputs/

cd ../docs
