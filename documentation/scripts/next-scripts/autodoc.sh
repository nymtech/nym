#!/bin/bash

set -o errexit
set -o nounset
set -o pipefail

# this is run by the `generate:commands` script in docs/package.json
cd ../autodoc && cargo run --release &&

mv autodoc-generated-markdown/nym-cli-commands.md ../docs/pages/developers/tools/nym-cli/commands.mdx &&
mv autodoc-generated-markdown/nym-client-commands.md ../docs/pages/developers/clients/websocket/commands.mdx &&
mv autodoc-generated-markdown/nym-socks5-client-commands.md ../docs/pages/developers/clients/socks5/commands.mdx &&

# commit files to git: needed for remote deployment from branch
if ! git diff --quiet -- "../docs/pages/developers/tools" "../docs/pages/developers/clients/websocket" "../docs/pages/developers/clients/socks5"; then
    printf "commiting changes"
    git add ../docs/pages/developers/
    git commit -m "auto commit generated command files"
    git push origin HEAD
else
    printf "nothing to commit"
fi

cd ../docs
