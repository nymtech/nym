#!/bin/bash

PROJECT_NAME="go"

set -eu
MODE="--release"

GO_DIR="./ffi"
GO_PATH="${GO_DIR}/bindings"
UDL_PATH="./src/bindings.udl"

build_artifacts() {
  # build rust
  cargo build $MODE
  # build go bindings
  printf "building go bindings \n"
  uniffi-bindgen-go $UDL_PATH --out-dir $GO_DIR
  printf "bindings built \n\n"

  # something not right with these - having to add it manually to bindings.go for the moment
  #pushd $GO_DIR/bindings
  #echo $(pwd)
  #LD_LIBRARY_PATH="${LD_LIBRARY_PATH:-}:../../target/release" \
  #	CGO_LDFLAGS="-L../target/release -lnym_go_ffi -lm -ldl" \
  #	CGO_ENABLED=1 \
  #	go run ../main.go
}

clean_artifacts() {
  # the path to the new folder we are including
  GO_DIR="./ffi"
  GO_PATH="${GO_DIR}/bindings"
  # clean up existing things
  rm -rf $GO_PATH
  cargo clean
}


if [ $(pwd | awk -F/ '{print $NF}') != ${PROJECT_NAME} ]
then
  printf "please run from root dir of project"
  exit 1
fi

if [ $# -eq 0 ];
then
  build_artifacts;
else
  arg=$1
  if [ "$arg" == "clean" ]; then
    clean_artifacts;
    build_artifacts;
  else
      printf "unknown optional argument - the only available optional argument is 'clean'"
      exit 1
  fi
fi
