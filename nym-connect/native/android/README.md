## Nym Connect for Android

### Prerequisites

_TODO_

### Getting started

[Install](https://developer.android.com/studio/install) Android Studio and open
the project.\
Setup an android emulator using AVD.\
[Run](https://developer.android.com/studio/run/emulator) the project.

**⚠ NOTE**: be sure
to [set](https://developer.android.com/studio/run#changing-variant)
the build variant to `x86_64Debug` when running on emulator

### Lib Nym socks5

This Application needs the
native [nym-socks5-listener](https://github.com/nymtech/nym/blob/develop/sdk/lib/socks5-listener/Cargo.toml)
library in order to work.

To build it for x86_64 and arm64 arch, from the root of the repo run

```shell
cd sdk/lib/socks5-listener/
./build-android.sh aarch64 x86_64
```

To build in release mode run

```shell
RELEASE=true ./build-android.sh aarch64 x86_64
```

The shared library for each ABIs will be automatically moved into
`app/src/main/jniLibs/*` directories.

### APK/AAB build (from terminal)

This project is setup with multiple [product flavors](app/build.gradle) to
build for specific architectures.\
Supported archs:

- arm64
- arm (old arm)
- x86_64
- x86

For example to build an APK for _arm64_ in _release_ mode use

```shell
./gradlew :app:assembleArm64Release
```

Instead of building an APK, to build an app bundle (`.aab`) run

```shell
./gradlew :app:bundleArm64Release
```

**NOTE**: you likely want _arch64_ (`arm64` & `x86_64`) for APK distribution

To build a _universal_ APK which includes all ABI use

```shell
./gradlew :app:assembleUniversalRelease
```

**⚠ WARNING**: APK size will be multiplied
