#!/bin/bash

set -o errexit
set -o nounset
set -o pipefail

# this is run by the `prebuild` and `predev` scripts in docs/package.json
cd ../autodoc && cargo run --release &&

mv autodoc-generated-markdown/nym-cli-commands.md ../docs/pages/developers/tools/nym-cli/commands.mdx &&
mv autodoc-generated-markdown/nym-client-commands.md ../docs/pages/developers/clients/websocket/commands.mdx &&
mv autodoc-generated-markdown/nym-socks5-client-commands.md ../docs/pages/developers/clients/socks5/commands.mdx &&

cd ../docs
