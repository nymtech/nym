<!--
Copyright 2020 - Nym Technologies SA <contact@nymtech.net>
SPDX-License-Identifier: Apache-2.0
-->

## The Nym Privacy Platform

The platform is composed of multiple Rust crates. Top-level executable binary crates include:

* `nym-node` - a tool for running a node within the Nym network. Nym Nodes containing functionality such as `mixnode`, `entry-gateway` and `exit-gateway` are fundamental components of Nym Mixnet architecture. Nym Nodes are ran by decentralised node operators. Read more about `nym-node` in [Operators Guide documentation](https://nymtech.net/operators/nodes/nym-node.html). Network functionality of `nym-node` (labeled with `--mode` flag) can be:
	- `mixnode` - shuffles [Sphinx](https://github.com/nymtech/sphinx) packets together to provide privacy against network-level attackers.
	- `gateway` - acts sort of like a mailbox for mixnet messages, which removes the need for direct delivery to potentially offline or firewalled devices. Gateways can be further categorized as `entry-gateway` and `exit-gateway`. The latter has an extra embedded IP packet router and Network requester to route data to the internet.
* `nym-client` - an executable which you can build into your own applications. Use it for interacting with Nym nodes.
* `nym-socks5-client` - a Socks5 proxy you can run on your machine and use with existing applications.
* `nym-explorer` - a (projected) block explorer and (existing) mixnet viewer.
* `nym-wallet` - a desktop wallet implemented using the [Tauri](https://tauri.studio/en/docs/about/intro) framework.
<!-- coming soon
* `nym-network-monitor` - sends packets through the full system to check that they are working as expected, and stores node uptime histories as the basis of a rewards system ("mixmining" or "proof-of-mixing").
-->

```ascii
                      ┌─►mix──┐  mix     mix
                      │       │
            Entry     │       │                   Exit
client ───► Gateway ──┘  mix  │  mix  ┌─►mix ───► Gateway ───► internet
                              │       │
                              │       │
                         mix  └─►mix──┘  mix

```

[![Build Status](https://img.shields.io/github/actions/workflow/status/nymtech/nym/build.yml?branch=develop&style=for-the-badge&logo=github-actions)](https://github.com/nymtech/nym/actions?query=branch%3Adevelop)


### Building

* Platform build instructions are available on Nym [Operators Guide documentation](https://nymtech.net/operators/binaries/building-nym.html).
* Wallet build instructions are available on Nym [Technical docs](https://nymtech.net/docs/wallet/desktop-wallet.html).

### Developing

There's a [`sandbox.env`](https://github.com/nymtech/nym/envs/sandbox.env) file provided which you can rename to `.env` if you want convenient testing environment. Read more about sandbox environment in our [Operators Guide page](https://nymtech.net/operators/sandbox.html).

References for developers:

* [Developers Portal](https://nymtech.net/developers)
* [Typescript SDKs](https://sdk.nymtech.net/)
* [Technical Documentation - Nym network overview](https://nymtech.net/docs/)
* [Release Cycle - git flow](https://nymtech.net/operators/release-cycle.html)

### Developer chat

You can chat to us in two places:
* The #dev channel on [Matrix](https://matrix.to/#/#dev:nymtech.chat)
* The various developer channels on [Discord](https://discord.gg/FaTJb8q8)

### Tokenomics & Rewards

Nym network economic incentives, operator and validator rewards, and scalability of the network are determined according to the principles laid out in the section 6 of [Nym Whitepaper](https://nymtech.net/nym-whitepaper.pdf).
Initial reward pool is set to 250 million Nym, making the circulating supply 750 million Nym.

### Licensing and copyright information

This is a monorepo and components that make up Nym as a system are licensed individually, so for accurate information, please check individual files.

As a general approach, licensing is as follows this pattern:

- applications and binaries are GPLv3
- libraries and components are Apache 2.0 or MIT
- documentation is Apache 2.0 or CC0-1.0

Nym Node Operators and Validators Temrs and Conditions can be found [here](https://nymtech.net/terms-and-conditions/operators/v1.0.0).
