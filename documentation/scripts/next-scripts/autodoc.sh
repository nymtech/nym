#!/bin/bash

# this script is run by the `generate:commands` script in docs/package.json

set -o errexit
set -o nounset
set -o pipefail

# make sure we have all the binaries built
<<<<<<< HEAD
cd ../../ && cargo build --release && cd tools/nym-cli && cargo build --release && cd ../../ &&
=======
cd ../../ && cargo build --release &&
>>>>>>> 276d4318beecdd8eb3a870e4bd6ecbe3edb2f9b0

  # run autodoc script
  cd documentation/autodoc/ && cargo run --release &&
  mv autodoc-generated-markdown/nym-cli-commands.md ../docs/pages/developers/tools/nym-cli/commands.mdx &&
  mv autodoc-generated-markdown/nym-client-commands.md ../docs/pages/developers/clients/websocket/commands.mdx &&
  mv autodoc-generated-markdown/nym-socks5-client-commands.md ../docs/pages/developers/clients/socks5/commands.mdx &&
  mv autodoc-generated-markdown/commands/* ../docs/components/outputs/command-outputs/ &&

  # commit files to git: needed for remote deployment from branch
<<<<<<< HEAD
  if ! git diff --quiet -- "../docs/pages/developers/tools" "../docs/pages/developers/clients/websocket" "../docs/pages/developers/clients/socks5" "../docs/components/outputs/command-outputs/"; then
=======
  if ! git diff --quiet -- "../docs/pages/developers/tools" "../docs/pages/developers/clients/websocket" "../docs/pages/developers/clients/socks5"; then
>>>>>>> 276d4318beecdd8eb3a870e4bd6ecbe3edb2f9b0
    printf "commiting changes"
    git add ../docs/pages/developers/ ../docs/components/outputs/command-outputs/
    git commit -m "auto commit generated command files"
    git push origin HEAD
  else
    printf "nothing to commit"
  fi

cd ../docs
