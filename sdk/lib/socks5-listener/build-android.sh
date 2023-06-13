#!/bin/bash
#
# Usage
#
# build-android.sh [ARCH ...]
#
# This script builds the lib for android and moves the shared
# objects (*.so) into the right app's directories
#
# ARCH:
# - aarch64 (arm 64)
# - x86_64  (classic PC 64)
# - i686    (x86)
# - armv7
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

# arch mapping between Rust targets (keys) and Android ABIs (values)
# https://developer.android.com/ndk/guides/abis.html
declare -A arch_map=([x86_64]="x86_64" [aarch64]="arm64-v8a" [armv7]="armeabi-v7a" [i686]="x86")

output_dir=../../../target
jni_dir=../../../nym-connect/native/android/app/src/main/jniLibs
lib=libnym_socks5_listener.so

build () {
  abi="${arch_map[$1]}"
  echo -e " $B_YLW⚡$RS building for arch $BLD$1$RS"
  export TARGET_CC="$TOOLCHAIN/bin/$1-linux-android$API-clang"
  if [ -a "$jni_dir/$abi/$lib" ]; then
    # remove any previously built library
    rm "$jni_dir/$abi/$lib"
  fi
  if [ "$RELEASE" = true ]; then
    cargo build --lib --target "$1-linux-android" --release
    mv "$output_dir/$1-linux-android/release/$lib" "$jni_dir/$abi/"
  else
    cargo build --lib --target "$1-linux-android"
    mv "$output_dir/$1-linux-android/debug/$lib" "$jni_dir/$abi/"
  fi
  echo -e " $B_GRN✓$RS lib built successfully for $BLD$1$RS, moved under app's dir$BLD jniLibs/$abi/$RS"
}

for arch in "$@"; do
  if [ "${arch_map[$arch]}" ]; then
    build "$arch"
  else
    echo -e " $B_RED✗$RS unknown arch $BLD$arch$RS"
    exit 1
  fi
done
