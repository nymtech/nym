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

# something not right with these - having to add it manually to bindings.go for the moment 
pushd $GO_DIR/bindings
echo $(pwd)
LD_LIBRARY_PATH="${LD_LIBRARY_PATH:-}:../../target/release" \
	CGO_LDFLAGS="-L../target/release -lnym_go_ffi -lm -ldl" \
	CGO_ENABLED=1 \
	go run ../main.go 



