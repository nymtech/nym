## Nym Connect for Android

### Prerequisites

_TODO_

### Getting started

Install Android Studio and open the project.

### Lib Nym socks5

This Application needs the native [nym-socks5-listener](https://github.com/nymtech/nym/blob/develop/sdk/lib/socks5-listener/Cargo.toml)
library in order to work.

To build it, from the root of the repo run

```shell
cd sdk/lib/socks5-listener/
./build-android.sh
```

To build in release mode

```shell
RELEASE=true ./build-android.sh
```

The shared library for each ABIs will be automatically moved into 
`app/src/main/jniLibs/*` directories.

### Run _TODO_