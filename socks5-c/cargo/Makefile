ANDROID_API=33
ANDROID_TOOLCHAIN=$(NDK_HOME)/toolchains/llvm/prebuilt/linux-x86_64
ANDROID_TARGET_CC=$(ANDROID_TOOLCHAIN)/bin/x86_64-linux-android$(ANDROID_API)-clang
ANDROID_TARGET_AR=$(ANDROID_TOOLCHAIN)/bin/llvm-ar
ANDROID_TARGET_RANLIB=$(ANDROID_TOOLCHAIN)/bin/llvm-ranlib
ANDROID_CARGO_TARGET_X86_64_LINUX_ANDROID_LINKER=$(ANDROID_TOOLCHAIN)/bin/x86_64-linux-android$(ANDROID_API)-clang

# Common environment
ANDROID_ENV = \
	TARGET_CC=$(ANDROID_TARGET_CC) \
	TARGET_AR=$(ANDROID_TARGET_AR) \
	TARGET_RANLIB=$(ANDROID_TARGET_RANLIB) \
	CARGO_TARGET_X86_64_LINUX_ANDROID_LINKER=$(ANDROID_CARGO_TARGET_X86_64_LINUX_ANDROID_LINKER)

ANDROID_TARGET=x86_64-linux-android

none:
	@echo "No default target. Please specify a target."

gen-headers:
	cargo run --features headers --bin generate-headers

#gen-header:
#	cbindgen --lang c --crate socks5-c --output socks5_c.h

android:
	$(ANDROID_ENV) cargo build --lib --target $(ANDROID_TARGET)

android-release:
	$(ANDROID_ENV) cargo build --lib --target $(ANDROID_TARGET) --release
