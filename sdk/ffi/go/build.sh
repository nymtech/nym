#!/bin/bash

PROJECT_NAME="go"

set -eu
MODE="--release"

GO_DIR="./go-nym"
UDL_PATH="./src/bindings.udl"

build_artifacts() {
  # build rust
  cargo build $MODE
  # build go bindings
  printf "building go bindings \n"
  uniffi-bindgen-go $UDL_PATH --out-dir $GO_DIR
  printf "bindings built \n\n"
  # TODO pull in auto binding from https://github.com/NordSecurity/uniffi-bindgen-go/blob/main/test_bindings.sh (removes need for manual addition of cgo flags)
}

clean_artifacts() {
  # clean up existing things
  rm -rf $GO_DIR
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
