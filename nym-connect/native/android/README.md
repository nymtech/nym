## Nym Connect for Android

### Prerequisites

_TODO_

### Getting started

Install Android Studio and open the project.

### Lib Nym socks5

This Application needs the native [nym-socks5-listener](https://github.com/nymtech/nym/blob/develop/sdk/lib/socks5-listener/Cargo.toml)
library in order to work.

To build it for arch x64 and arm (x64 is mainly for android emulator, arm 
for APK distribution), from the root of the repo run

```shell
cd sdk/lib/socks5-listener/
./build-android.sh aarch64 x86_64
```

To build in release mode run

```shell
RELEASE=true ./build-android.sh aarch64
```

The shared library for each ABIs will be automatically moved into 
`app/src/main/jniLibs/*` directories.

### Run _TODO_