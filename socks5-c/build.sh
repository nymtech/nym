#!/bin/sh

export API=33
export TOOLCHAIN="$NDK_HOME/toolchains/llvm/prebuilt/linux-x86_64"
export TARGET_CC="$TOOLCHAIN/bin/x86_64-linux-android$API-clang"
export TARGET_AR="$TOOLCHAIN/bin/llvm-ar"
export PATH="$TOOLCHAIN/bin/:$PATH"
export RANLIB="$TOOLCHAIN/bin/llvm-ranlib"

cargo build --lib --target x86_64-linux-android
