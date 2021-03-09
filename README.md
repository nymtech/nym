<!--
Copyright 2020 - Nym Technologies SA <contact@nymtech.net>
SPDX-License-Identifier: Apache-2.0
-->

## The Nym Privacy Platform

This repository contains the Nym mixnet.

The platform is composed of multiple Rust crates. Top-level executable binary crates include:

* nym-mixnode - shuffles [Sphinx](https://github.com/nymtech/sphinx) packets together to provide privacy against network-level attackers.
* nym-client - an executable which you can build into your own applications. Use it for interacting with Nym nodes.
* nym-socks5-client - a Socks5 proxy you can run on your machine, and use with existing applications
* nym-gateway - acts sort of like a mailbox for mixnet messages, removing the need for directly delivery to potentially offline or firewalled devices.
* nym-network-monitor - sends packets through the full system to check that they are working as expected, and stores node uptime histories as the basis of a rewards system ("mixmining" or "proof-of-mixing").
* nym-explorer - a (projected) block explorer and (existing) mixnet viewer.

[![License](https://img.shields.io/badge/License-Apache%202.0-blue.svg?style=for-the-badge)](https://opensource.org/licenses/Apache-2.0)
[![Build Status](https://img.shields.io/github/workflow/status/nymtech/nym/Continuous%20integration/develop?style=for-the-badge&logo=github-actions)](https://github.com/nymtech/nym/actions?query=branch%3Adevelop)


### Building

Platform build instructions are available on [our docs site](https://nymtech.net/docs).

### Developing

There's a `.env.sample-dev` file provided which you can rename to `.env` if you want convenient logging, backtrace, or other environment variables pre-set. The `.env` file is ignored so you don't need to worry about checking it in.

### Developer chat

You can chat to us in [Keybase](https://keybase.io). Download their chat app, then click **Teams -> Join a team**. Type **nymtech.friends** into the team name and hit **continue**. For general chat, hang out in the **#general** channel. Our development takes places in the **#dev** channel. Node operators should be in the **#node-operators** channel.

### Licensing and copyright information

This program is available as open source under the terms of the Apache 2.0 license. However, some elements are being licensed under CC0-1.0 and MIT. For accurate information, please check individual files.
