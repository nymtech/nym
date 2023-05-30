#!/bin/bash

# This script builds the lib for android and moves the shared
# objects (*.so) into the right app's directories
#
# currently it builds for:
# - aarch64 (arm 64)
# - x86_64 (classic PC 64)
#
# ⚠ to build for release set the env var `RELEASE=true`

set -E
set -o pipefail
trap 'catch $? ${FUNCNAME[0]:-main} $LINENO' ERR

# ANSI style codes
RED="\e[38;5;1m" # red
GRN="\e[38;5;2m" # green
YLW="\e[38;5;3m" # yellow
BLD="\e[1m"      # bold
RS="\e[0m"       # style reset
# bold variants
B_RED="$BLD$RED"
B_GRN="$BLD$GRN"
B_YLW="$BLD$YLW"

catch() {
  echo -e " $B_RED✗$RS unexpected error, $BLD$2$RS [$BLD$1$RS] L#$BLD$3$RS"
  exit 1
}

export API=33
export TOOLCHAIN="$NDK_HOME/toolchains/llvm/prebuilt/linux-x86_64"
export TARGET_AR="$TOOLCHAIN/bin/llvm-ar"
export TARGET_RANLIB="$TOOLCHAIN/bin/llvm-ranlib"
export CARGO_TARGET_X86_64_LINUX_ANDROID_LINKER="$TOOLCHAIN/bin/x86_64-linux-android$API-clang"
export CARGO_TARGET_AARCH64_LINUX_ANDROID_LINKER="$TOOLCHAIN/bin/aarch64-linux-android$API-clang"

output_dir=../../../target
jni_dir=../../../nym-connect/native/android/app/src/main/jniLibs

build () {
  echo -e " $B_YLW⚡$RS building for arch $BLD$1$RS"
  export TARGET_CC="$TOOLCHAIN/bin/$1-linux-android$API-clang"
  if [ "$RELEASE" = true ]; then
    cargo build --lib --target "$1-linux-android" --release
    mv "$output_dir/$1-linux-android/release/libnym_socks5_listener.so" "$jni_dir/$2/"
  else
    cargo build --lib --target "$1-linux-android"
    mv "$output_dir/$1-linux-android/debug/libnym_socks5_listener.so" "$jni_dir/$2/"
  fi
  echo -e " $B_GRN✓$RS lib successfully builded for $BLD$1$RS, moved under app's dir$BLD jniLibs/$2/$RS"
}

# build for x86_64
build x86_64 x86_64

# build for aarch64
build aarch64 arm64-v8a
