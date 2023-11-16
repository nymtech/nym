# Building nymvpn app

## Install build dependencies

### Common for all Platforms

```
cargo install cargo-deb
cargo install cargo-generate-rpm
cargo install --force cargo-make
cargo install sd
cargo install ripgrep
cargo install cargo-about
```

### Linux

```
apt install build-essential \
    pkg-config \
    libgtk-3-dev \
    libssl-dev \
    libsoup2.4-dev \
    libjavascriptcoregtk-4.0-dev \
    libwebkit2gtk-4.0-dev \
    libmnl-dev \
    libnftnl-dev \
    protobuf-compiler \
    zip \

```

Install protoc on x86_64/amd64 machines
```
# x86_64
curl -Lo protoc-3.19.1-linux-x86_64.zip \
    https://github.com/protocolbuffers/protobuf/releases/download/v3.19.1/protoc-3.19.1-linux-x86_64.zip  && \
    unzip protoc-3.19.1-linux-x86_64.zip -d /tmp && \
    mv /tmp/bin/protoc /usr/bin/protoc && \
    rm protoc-3.19.1-linux-x86_64.zip
```

Install protoc on arm64 machines
```
# aarch64
curl -Lo protoc-3.19.1-linux-aarch_64.zip \
    https://github.com/protocolbuffers/protobuf/releases/download/v3.19.1/protoc-3.19.1-linux-aarch_64.zip  && \
    unzip protoc-3.19.1-linux-aarch_64.zip -d /tmp && \
    sudo mv /tmp/bin/protoc /usr/bin/protoc && \
    rm protoc-3.19.1-linux-aarch_64.zip

```

### macOS
TODO

### Windows
TODO

## Build Debian package

```
cargo make deb
```

## Build RPM package

```
cargo make rpm
```

## Build macOS package

```
cargo make pkg
```

To codesign for distribution provide following environment variables:

```
APPLE_TEAM_ID=...
APPLICATION_SIGNING_IDENTITY=...
INSTALLER_SIGNING_IDENTITY=...
cargo make pkg
```

## Build installer for Windows

```
cargo make msi
```

To codesign for distribution:

```
SIGN=true cargo make msi
```

## Building for Production for Linux

### Build the Builder

Build the Docker image to build nymvpn app.
```
cd nymvpn-packages
cargo make builder
```

This will output `tag.txt`, commit it into source code.

### Build .deb and .rpm

This step uses builder Docker image with tag in `nymvpn-packages/tag.txt`. The final rpm and deb packages will be stored in `dist` directory.

```
# For host platform
cargo make linux

# For target platform
cargo make -e TARGET=aarch64-unknown-linux-gnu linux
```
