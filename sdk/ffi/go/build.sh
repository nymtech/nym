#!/bin/bash

set -eu
MODE="--release"

GO_DIR="./ffi"
UDL_PATH="./src/bindings.udl"

# build rust
cargo build $MODE
# build go bindings
printf "building go bindings... \n"
uniffi-bindgen-go $UDL_PATH --out-dir $GO_DIR
printf "...bindings built \n\n"
