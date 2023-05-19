#!/bin/sh

arch=x86_64

# TODO to compile for arm64 v8a uncomment this
# arch=aarch64

export API=33
export TOOLCHAIN="$NDK_HOME/toolchains/llvm/prebuilt/linux-x86_64"
export TARGET_CC="$TOOLCHAIN/bin/$arch-linux-android$API-clang"
export TARGET_AR="$TOOLCHAIN/bin/llvm-ar"
export TARGET_RANLIB="$TOOLCHAIN/bin/llvm-ranlib"
export CARGO_TARGET_X86_64_LINUX_ANDROID_LINKER="$TOOLCHAIN/bin/x86_64-linux-android$API-clang"
export CARGO_TARGET_AARCH64_LINUX_ANDROID_LINKER="$TOOLCHAIN/bin/aarch64-linux-android$API-clang"

cargo build --lib --target $arch-linux-android --release
