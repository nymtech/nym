<!--
Copyright 2020 - Nym Technologies SA <contact@nymtech.net>
SPDX-License-Identifier: Apache-2.0
-->

# Nym Connect

Nym is an open-source, decentralized and permissionless privacy system. It provides full-stack privacy, allowing other applications, services or blockchains to provide their users with strong metadata protection, at both the network level (mixnet), and the application level (anonymous credentials) without the need to build privacy from scratch.

Nym Connects sets up a SOCKS5 proxy for local applications to use.

## Installation prerequisites - Linux / Mac

- `Yarn`
- `NodeJS >= v16.8.0`
- `Rust & cargo >= v1.56`

## Installation prerequisites - Windows

- When running on Windows you will need to install c++ build tools
- An easy guide to get rust up and running [Installation]("http://kennykerr.ca/2019/11/18/rust-getting-started/")
- When installing NodeJS please use the `current features` version
- Using a package manager like [Chocolatey]("chocolatey.org") is recommended
- Nym connect requires you to have `Webview2` installed, please head to the [Installer](https://developer.microsoft.com/en-us/microsoft-edge/webview2/#download-section), this will ensure a smooth app launch

## Installation

Inside the `nym-connect` directory, run the following command:
```
yarn install
```

## Development mode

You can compile nym-connect in development mode by running the following command inside the `nym-connect` directory:

```
yarn dev
```
This will produce a binary in - `nym-connect/target/debug/` named `nym-connect`

To launch, navigate to the directory and run the following command: `./nym-connect`

## Production mode

Run the following command from the `nym-connect` folder
```
yarn build
```
The output will compile different types of binaries dependent on your hardware / OS system. Once the binaries are built, they can be located as follows:

### Binary output directory structure 
```
**macos**
|
└─── target/release
|   |─ nym-connect
└───target/release/bundle/dmg
│   │─ bundle_dmg.sh
│   │─ nym-connect.*.dmg
└───target/release/bundle/macos/MacOs
│   │─ nym-connect
|
**Linux**
└─── target/release
|   │─  nym-connect
└───target/release/bundle/appimage
│   │─  nym-connect_*_.AppImage
│   │─  build_appimage.sh
└───target/release/bundle/deb
│   │─  nym-connect_*_.deb
|
**Windows**
└─── target/release
|   │─  nym-connect.exe
└───target/release/bundle/msi
│   │─  nym-connect_*_.msi
```

For instructions on how to release nym-connect, please see [RELEASE.md](./docs/release/RELEASE.md).

# Storybook

Run storybook with:

```
yarn storybook
```

And build storybook static site with:

```
yarn storybook:build
```

