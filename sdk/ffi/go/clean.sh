#!/bin/bash

set -eu

# the path to the new folder we are including
GO_DIR="./nymffi"
GO_PATH="${GO_DIR}/bindings"

# clean up existing things
rm -rf $GO_PATH
cargo clean

# linker path - TODO move manual commands here + move this to build.sh

