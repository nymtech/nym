#!/usr/bin/env bash

set -o errexit
set -o nounset
set -o pipefail

# -------------------------------------------------------
# NB: Run this from the root of the repository.
# Each smolmix-based SDK package is published in place from
# its source directory after `pnpm build:ci:sdk` produces
# the dist/ output inside each package.
#
# Scope: only the four smolmix-family packages. The legacy
# sdk/typescript/packages/{sdk,sdk-react,nodejs-client}
# directories exist on disk but are not in pnpm-workspace.yaml
# and are not built or published by this flow.
#
# @nymproject/smolmix-wasm is workspace-internal: its bytes
# are base64-inlined into @nymproject/mix-tunnel at build
# time, so it must never be published. The smolmix Makefile
# marks pkg/package.json as `private: true` to enforce this.
# -------------------------------------------------------

REPO_ROOT="$(git rev-parse --show-toplevel)"
cd "$REPO_ROOT"

packages=(
  "sdk/typescript/packages/mix-tunnel"
  "sdk/typescript/packages/mix-fetch"
  "sdk/typescript/packages/mix-dns"
  "sdk/typescript/packages/mix-websocket"
)

pushd () {
  command pushd "$@" > /dev/null
}

popd () {
  command popd > /dev/null
}

echo "Summary of versions to publish:"
echo
for item in "${packages[@]}"; do
  pushd "$item"
  jq -r '. | "  " + .version + "  " + .name' < package.json
  popd
done

echo
# `pnpm publish` (not `npm publish`) is required because the four packages
# depend on each other via `workspace:*`. pnpm rewrites `workspace:*` to the
# real version in the published tarball at pack time; npm leaves the literal
# string, which produces tarballs that fail to install.
#
# Flags:
#   --access=public      scoped packages default to private; smolmix-family is public
#   --no-git-checks      CI runners can have dirty working trees from the build step
#   --tag $NPM_DIST_TAG  defaults to `latest`; set to `next` for pre-release shipping
NPM_DIST_TAG="${NPM_DIST_TAG:-latest}"
DRY_RUN_FLAG=""
if [[ "${DRY_RUN:-0}" == "1" ]]; then
  DRY_RUN_FLAG="--dry-run"
  echo "DRY_RUN=1 — running pnpm publish in dry-run mode (no tarballs uploaded)"
fi

COUNTER=0
for item in "${packages[@]}"; do
  (( COUNTER+=1 ))
  pushd "$item"
  echo "Publishing $item  (${COUNTER}/${#packages[@]})  --tag $NPM_DIST_TAG"
  pnpm publish --access=public --no-git-checks --tag "$NPM_DIST_TAG" $DRY_RUN_FLAG
  popd
  echo
done
echo
echo "Done."
