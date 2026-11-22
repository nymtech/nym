#!/usr/bin/env bash
# Write a release version into the workspace dependency pins, before
# `cargo workspaces version` bumps the crates themselves. The real bump
# (ci-crates-version-bump.yml) and the dry run (ci-crates-publish-dry-run.yml)
# both call this, so the dry run tests the same bump that the release makes.
#
# Usage: tools/internal/bump-manifest-versions.sh <version>
# Run from the repository root.
set -euo pipefail

[ "$#" -eq 1 ] || { echo "usage: $0 <version>" >&2; exit 2; }
version=$1

# Root workspace: every pin on a `path = ` line. `[0-9][^"]*` also matches
# pre-release suffixes (1.21.5-rc.3) and drifted pins (nym-sqlx-pool-guard at
# 1.2.0); a strict X.Y.Z pattern silently skips both.
sed -i "/path = /s/version = \"[0-9][^\"]*\"/version = \"${version}\"/g" Cargo.toml

# Contracts workspace: it is not part of the release, but nym-api reaches
# contracts/coconut-dkg by path, so its pins on main-workspace crates must
# follow the release or a major bump fails to resolve.
sed -i "\|path = \"../common/|s/version = \"[0-9][^\"]*\"/version = \"${version}\"/g" contracts/Cargo.toml
sed -i -E "/path = /! s/^(nym-[a-z0-9-]+ = )\"[0-9][^\"]*\"/\1\"${version}\"/" contracts/Cargo.toml
sed -i -E "/path = /! s/^(nym-[a-z0-9-]+ = \{ version = )\"[0-9][^\"]*\"/\1\"${version}\"/" contracts/Cargo.toml
sed -i -E "s/^((contracts-common|mixnet-contract-common|vesting-contract-common|nym-node-families-contract-common) = \{ version = )\"[0-9][^\"]*\"/\1\"${version}\"/" contracts/Cargo.toml
