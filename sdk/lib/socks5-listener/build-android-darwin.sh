#!/bin/sh

export API=33
export TOOLCHAIN="$NDK_HOME/toolchains/llvm/prebuilt/darwin-x86_64"
export TARGET_CC="$TOOLCHAIN/bin/x86_64-linux-android$API-clang"
export TARGET_AR="$TOOLCHAIN/bin/llvm-ar"
export TARGET_RANLIB="$TOOLCHAIN/bin/llvm-ranlib"
export CARGO_TARGET_X86_64_LINUX_ANDROID_LINKER="$TOOLCHAIN/bin/x86_64-linux-android$API-clang"

cargo build --lib --target x86_64-linux-android --release
