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

# `pnpm publish` rewrites each package's `workspace:*` deps (mix-fetch/dns/websocket
# all depend on mix-tunnel) to concrete versions at pack time by re-globbing
# pnpm-workspace.yaml for the sibling packages. They are only in the workspace
# while dev mode is on, and build-prod-sdk.sh turns dev mode off when it finishes,
# so enable it here and restore the clean state on exit. This is a YAML edit only;
# no reinstall is needed because pack-time resolution reads sibling package.json
# versions directly.
# Arm the cleanup before mutating the workspace: dev:off filters unconditionally,
# so it is safe even if dev:on fails mid-write, and this closes the window where a
# crash between the two would leave pnpm-workspace.yaml dirty.
#
# The trap must cd back to the repo root first: a failed `pnpm publish` exits
# (errexit) while still inside a pushd'd package dir, and `dev:off` is a script
# defined only in the root package.json. Running it from a package dir fails with
# "Command dev:off not found" and leaves dev mode on.
trap 'cd "$REPO_ROOT" && pnpm dev:off' EXIT
pnpm dev:on

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

# Resolve the npm dist-tag for one package.
#
# NPM_DIST_TAG=latest|next forces that tag on every package. The default,
# `auto`, derives the tag per package from what is already on npm so that no
# manual promote step has to be remembered:
#
#   * version has a prerelease suffix (x.y.z-rc.N) -> next
#       prereleases must never become the default install.
#   * package is not yet on npm (npm view 404s)    -> latest
#       a first publish has to set `latest`, or a bare `npm i` finds no version.
#   * publishing major == current `latest` major   -> latest
#       an ordinary patch/minor of the live line.
#   * publishing major  > current `latest` major   -> next
#       a breaking major: ship under `next`, leave existing users on `latest`,
#       and promote later with `npm dist-tag add <pkg>@<version> latest`.
#       Once `latest` points at the new major, the next patch resolves to
#       `latest` again on its own, with nothing to clean up.
#   * publishing major  < current `latest` major   -> next
#       a backport; never silently move `latest` backwards. Pass an explicit
#       NPM_DIST_TAG if a dedicated legacy tag is wanted.
resolve_tag() {
  local name="$1" version="$2" mode="$3"
  if [[ "$mode" != "auto" ]]; then
    printf '%s' "$mode"
    return
  fi
  if [[ "$version" == *-* ]]; then
    printf 'next'
    return
  fi
  # Find the package's current `latest` version on npm. There are three
  # outcomes, each driving the tag decision below:
  #   - success           -> compare majors (same major: latest; higher: next)
  #   - package not found  -> first ever publish; it has to set `latest`
  #   - any other failure  -> registry/network problem; abort rather than guess
  #
  # The third case is why this is careful. Defaulting to `latest` when the
  # lookup merely failed would move `latest` onto this version even for a
  # package that already exists, and so could push a breaking major onto every
  # current consumer. Only a genuine 404 counts as "new"; anything else aborts.
  #
  # stderr is sent to a file rather than merged into stdout so the success value
  # holds only the version (never an npm warning), while the failure text can
  # still be inspected to recognise a 404.
  local current err
  err="$(mktemp)"
  if current="$(npm view "$name" dist-tags.latest 2>"$err")"; then
    rm -f "$err"
  elif grep -qiE 'E404|404 Not Found' "$err"; then
    current=""
    rm -f "$err"
  else
    printf 'npm view failed for %s (not a 404), refusing to guess a dist-tag: %s\n' "$name" "$(cat "$err")" >&2
    rm -f "$err"
    return 1
  fi

  if [[ -z "$current" ]]; then
    printf 'latest'
  elif [[ "${version%%.*}" == "${current%%.*}" ]]; then
    printf 'latest'
  else
    printf 'next'
  fi
}

NPM_DIST_TAG="${NPM_DIST_TAG:-auto}"

# Resolve every tag up front so the summary shows exactly what each package will
# get. Under DRY_RUN this is the whole point: you see the tag decision before
# anything is uploaded.
declare -a TAGS
echo "Summary of packages to publish (mode: $NPM_DIST_TAG):"
echo
for item in "${packages[@]}"; do
  name="$(jq -r '.name' "$item/package.json")"
  version="$(jq -r '.version' "$item/package.json")"
  tag="$(resolve_tag "$name" "$version" "$NPM_DIST_TAG")"
  TAGS+=("$tag")
  printf '  %-34s %-8s --tag %s\n' "$name" "$version" "$tag"
done

echo
# `pnpm publish` (not `npm publish`) is required because the four packages
# depend on each other via `workspace:*`. pnpm rewrites `workspace:*` to the
# real version in the published tarball at pack time; npm leaves the literal
# string, which produces tarballs that fail to install.
#
# Flags:
#   --access=public   scoped packages default to private; smolmix-family is public
#   --no-git-checks   CI runners can have dirty working trees from the build step
#   --tag <tag>       per-package, resolved above
DRY_RUN_FLAG=""
if [[ "${DRY_RUN:-0}" == "1" ]]; then
  DRY_RUN_FLAG="--dry-run"
  echo "DRY_RUN=1: running pnpm publish in dry-run mode (no tarballs uploaded)"
fi

COUNTER=0
for i in "${!packages[@]}"; do
  item="${packages[$i]}"
  tag="${TAGS[$i]}"
  (( COUNTER+=1 ))
  pushd "$item"
  echo "Publishing $item  (${COUNTER}/${#packages[@]})  --tag $tag"
  pnpm publish --access=public --no-git-checks --tag "$tag" $DRY_RUN_FLAG
  popd
  echo
done
echo
echo "Done."
