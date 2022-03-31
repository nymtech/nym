<!--
Copyright 2020 - Nym Technologies SA <contact@nymtech.net>
SPDX-License-Identifier: Apache-2.0
-->

# Nym Mixnode

A Rust Gravity bridge wallet address translator

## Requirements
Rust version >= 1.59

## Usage

You have two options; you can either build the project

In the root directory -
- `cargo build --release`

- Which will create the binary in `rootpathofproject/target/release`
- Navigate to this directory

To execute the binary run the following

* `./gravity-address-translator --address {nym-wallet-address}`

Alternatively you can run the following

* `cargo run -- --address {nym-wallet-address}`

Which will output the following:

* `Your gravity bridge address is: {nym-wallet-address}`