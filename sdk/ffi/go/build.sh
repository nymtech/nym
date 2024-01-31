#!/bin/bash

set -eu
MODE="--release"

# the path to the new folder we are including
# TODO CHANGE
#GO_DIR="./test-go"
GO_DIR="./ffi"
#UDL_PATH="./src/math.udl"
UDL_PATH="./src/bindings.udl"

# build rust
cargo build $MODE
# build go bindings
printf "building go bindings... \n"
uniffi-bindgen-go $UDL_PATH --out-dir $GO_DIR
printf "...bindings built \n\n"
