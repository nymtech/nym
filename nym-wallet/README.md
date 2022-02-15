<!--
Copyright 2020 - Nym Technologies SA <contact@nymtech.net>
SPDX-License-Identifier: Apache-2.0
-->

# Nym Wallet

Nym is an open-source, decentralized and permissionless privacy system. It provides full-stack privacy, allowing other applications, services or blockchains to provide their users with strong metadata protection, at both the network level (mixnet), and the application level (anonymous credentials) without the need to build privacy from scratch.

The Nym desktop wallet enables you to use the Nym network and take advantage of its key capabilities

## Installation prerequisites - Linux / Mac

- `Yarn`
- `NodeJS >= v16.8.0`
- `Rust & cargo >= v1.56`

## Installation prerequisites - Windows

- When running on Windows you will need to install c++ build tools
- An easy guide to get rust up and running [Installation]("http://kennykerr.ca/2019/11/18/rust-getting-started/")
- When installing NodeJS please use the `current features` version
- Using a package manager like [Chocolatey]("chocolatey.org") is recommended
- The nym wallet requires you to have `Webview2` installed, please head to the [Installer](https://developer.microsoft.com/en-us/microsoft-edge/webview2/#download-section), this will ensure a smooth app launch

## Installation

Inside the `nym-wallet` directory, run the following command:
```
yarn install
```

## Populate environment variables

The wallet requires you to supply a `.env` file, this populates values in the wallet once it's compiled.

In the project roots there's a `.env.sample` file, these values currently match what the `.env` file should be populated with. However, if you want to change these values you can do so accordingly.

- In the root directory, create a new file named `.env` 
- Input the values against the variables

## Development mode

You can compile the wallet in development mode by running the following command inside the `nym-wallet` directory:

```
yarn dev
```
This will produce a binary in - `nym-wallet/target/debug/` named `nym-wallet`

To launch the wallet, navigate to the directory and run the following command: `./nym-wallet`

## Production mode

Run the following command from the `nym-wallet` folder
```
yarn build
```
The output will compile different types of binaries dependent on your hardware / OS system. Once the binaries are built, they can be located as follows:

### Binary output directory structure 
```
**macos**
|
└─── target/release
|   |─ nym-wallet
└───target/release/bundle/dmg
│   │─ bundle_dmg.sh
│   │─ nym-wallet.*.dmg
└───target/release/bundle/macos/MacOs
│   │─ nym-wallet
|
**Linux**
└─── target/release
|   │─  nym-wallet
└───target/release/bundle/appimage
│   │─  nym-wallet_*_.AppImage
│   │─  build_appimage.sh
└───target/release/bundle/deb
│   │─  nym-wallet_*_.deb
|
**Windows**
└─── target/release
|   │─  nym-wallet.exe
└───target/release/bundle/msi
│   │─  nym-wallet_*_.msi
```

For instructions on how to release the wallet, please see [RELEASE.md](./docs/release/RELEASE.md).