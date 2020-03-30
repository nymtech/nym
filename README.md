## The Nym Privacy Platform

This repository contains the full Nym platform.

The platform is composed of multiple Rust crates. Top-level executable binary crates include:

* nym-mixnode - shuffles [Sphinx](https://github.com/nymtech/sphinx) packets together to provide privacy against network-level attackers.
* nym-client - an executable which you can build into your own applications. Use it for interacting with Nym nodes.
* nym-sfw-provider - a store-and-forward service provider. The provider acts sort of like a mailbox for mixnet messages.
* nym-validator - currently just starting development. Handles consensus ordering of transactions, mixmining, and coconut credential generation and validation. 

[![Build Status](https://travis-ci.com/nymtech/nym.svg?branch=develop)](https://travis-ci.com/nymtech/nym)

### Building

Platform build instructions are available on [our docs site](https://nymtech.net/docs/mixnet/installation/).

### Developing

There's a `.env.sample-dev` file provided which you can rename to `.env` if you want convenient logging, backtrace, or other environment variables pre-set. The `.env` file is ignored so you don't need to worry about checking it in.

### Developer chat

You can chat to us in [Keybase](https://keybase.io). Download their chat app, then click **Teams -> Join a team**. Type **nymtech.friends** into the team name and hit **continue**. For general chat, hang out in the **#general** channel. Our development takes places in the **#dev** channel. Node operators should be in the **#node-operators** channel.