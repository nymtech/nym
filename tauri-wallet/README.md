<!--
Copyright 2020 - Nym Technologies SA <contact@nymtech.net>
SPDX-License-Identifier: Apache-2.0
-->

# Nym Tauri Wallet

A Rust and Tauri desktop wallet implementation.

## Installation prerequisites

- `Yarn`
- `NodeJS >= v16.8.0`
- `Rust & cargo >= v1.51`

## Installation

Inside of the `tauri-wallet` folder, run the following commands

- `yarn install`

## Development mode

You can run the wallet without having to install it in development mode by running the following terminal command from the `tauri-wallet` folder

`yarn dev`

## Production mode

To build and install the wallet, run the following terminal command from the `tauri-wallet` folder

`$ yarn build`

This will build an executable file that you can use to install the wallet on your machine

## Install the wallet

Once the the building process is complete an installation file can be found in the following location `tauri-wallet/target/release/nym_wallet`
``
