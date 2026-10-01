#!/bin/bash

# Run by `generate:commands` in docs/package.json, and by the docs-autodoc CI workflow.
# Invoked from documentation/docs.
#
# Builds the three binaries whose --help and build-info the docs embed (nym-node, nym-api,
# nymvisor) in debug, then produces every committed command snippet from them: the
# top-level/run --help via direct capture here, and build-info plus per-subcommand help via
# the autodoc crate. The CI workflow owns the commit; this script does not touch git.
# predev and `pnpm dev` read the committed files and never build Rust.

set -o errexit
set -o nounset
set -o pipefail

# repo root: build only the three needed packages, in debug. The help and build-info text
# is the same as release, and debug is faster.
cd ../../
cargo build -p nym-node -p nym-api -p nymvisor

# Top-level and `run` --help, captured straight from the debug binaries. Moved here from
# python-prebuild.sh so predev builds no Rust. The fences match the old shell capture, so
# the committed files are unchanged.
OUT=documentation/docs/components/outputs/command-outputs
echo '```sh' > "$OUT/nym-node-help.md"
./target/debug/nym-node --help >> "$OUT/nym-node-help.md"
echo '```' >> "$OUT/nym-node-help.md"

echo '```sh' > "$OUT/nym-node-run-help.md"
./target/debug/nym-node run --help >> "$OUT/nym-node-run-help.md"
echo '```' >> "$OUT/nym-node-run-help.md"

echo '```sh' > "$OUT/nymvisor-help.md"
./target/debug/nymvisor --help >> "$OUT/nymvisor-help.md"
echo '```' >> "$OUT/nymvisor-help.md"

echo '```sh' > "$OUT/nym-api-help.md"
./target/debug/nym-api --help >> "$OUT/nym-api-help.md"
echo '```' >> "$OUT/nym-api-help.md"

# build-info and per-subcommand help via the autodoc crate.
cd documentation/autodoc/
cargo run
mv autodoc-generated-markdown/commands/* ../docs/components/outputs/command-outputs/

cd ../docs
