<!--
Copyright 2020 - Nym Technologies SA <contact@nymtech.net>
SPDX-License-Identifier: Apache-2.0
-->

## The Nym Privacy Platform

The platform is composed of multiple Rust crates. Top-level executable binary crates include:

* nym-mixnode - shuffles [Sphinx](https://github.com/nymtech/sphinx) packets together to provide privacy against network-level attackers.
* nym-client - an executable which you can build into your own applications. Use it for interacting with Nym nodes.
* nym-socks5-client - a Socks5 proxy you can run on your machine and use with existing applications.
* nym-gateway - acts sort of like a mailbox for mixnet messages, which removes the need for direct delivery to potentially offline or firewalled devices.
* nym-network-monitor - sends packets through the full system to check that they are working as expected, and stores node uptime histories as the basis of a rewards system ("mixmining" or "proof-of-mixing").
* nym-explorer - a (projected) block explorer and (existing) mixnet viewer.
* nym-wallet - a desktop wallet implemented using the [Tauri](https://tauri.studio/en/docs/about/intro) framework.

[![License](https://img.shields.io/badge/License-Apache%202.0-blue.svg?style=for-the-badge)](https://opensource.org/licenses/Apache-2.0)
[![Build Status](https://img.shields.io/github/actions/workflow/status/nymtech/nym/build.yml?branch=develop&style=for-the-badge&logo=github-actions)](https://github.com/nymtech/nym/actions?query=branch%3Adevelop)


### Building

Platform build instructions are available on [our docs site](https://nymtech.net/docs/binaries/pre-built-binaries.html).
Wallet build instructions are also available on [our docs site](https://nymtech.net/docs/wallet/desktop-wallet.html).

### Developing

There's a `.env.sample-dev` file provided which you can rename to `.env` if you want convenient logging, backtrace, or other environment variables pre-set. The `.env` file is ignored so you don't need to worry about checking it in.

For Typescript components, please see [ts-packages](./ts-packages).

### Developer chat

> We used to use Keybase for developer chats, but we have since migrated to Matrix and Discord. We no longer check the old **nymtech.friends** Keybase team.

You can chat to us in two places:
* The #dev channel on [Matrix](https://matrix.to/#/#dev:nymtech.chat)
* The various developer channels on [Discord](https://discord.gg/nym)

### Rewards

Node, node operator and delegator rewards are determined according to the principles laid out in the section 6 of [Nym Whitepaper](https://nymtech.net/nym-whitepaper.pdf). Below is a TLDR of the variables and formulas involved in calculating the epoch rewards. Initial reward pool is set to 250 million Nym, making the circulating supply 750 million Nym.

|Symbol|Definition|
|---|---|
|<img src="https://render.githubusercontent.com/render/math?math=R#gh-light-mode-only"><img src="https://render.githubusercontent.com/render/math?math=\color{white}R#gh-dark-mode-only">|global share of rewards available, starts at 2% of the reward pool.
|<img src="https://render.githubusercontent.com/render/math?math=R_{i}#gh-light-mode-only"><img src="https://render.githubusercontent.com/render/math?math=\color{white}R_{i}#gh-dark-mode-only">|node reward for mixnode `i`.
|<img src="https://render.githubusercontent.com/render/math?math=\sigma_{i}#gh-light-mode-only"><img src="https://render.githubusercontent.com/render/math?math=\color{white}\sigma_{i}#gh-dark-mode-only">|ratio of total node stake (node bond + all delegations) to the token circulating supply.
|<img src="https://render.githubusercontent.com/render/math?math=\lambda_{i}#gh-light-mode-only"><img src="https://render.githubusercontent.com/render/math?math=\color{white}\lambda_{i}#gh-dark-mode-only">|ratio of stake operator has pledged to their node to the token circulating supply.
|<img src="https://render.githubusercontent.com/render/math?math=\omega_{i}#gh-light-mode-only"><img src="https://render.githubusercontent.com/render/math?math=\color{white}\omega_{i}#gh-dark-mode-only">|fraction of total effort undertaken by node `i`, set to `1/k`.
|<img src="https://render.githubusercontent.com/render/math?math=k#gh-light-mode-only"><img src="https://render.githubusercontent.com/render/math?math=\color{white}k#gh-dark-mode-only">|number of nodes stakeholders are incentivised to create, set by the validators, a matter of governance. Currently determined by the `reward set` size, and set to 720 in testnet Sandbox.
|<img src="https://render.githubusercontent.com/render/math?math=\alpha#gh-light-mode-only"><img src="https://render.githubusercontent.com/render/math?math=\color{white}\alpha#gh-dark-mode-only">|A Sybil attack resistance parameter - the higher this parameter is set, the stronger the reduction in competitiveness for a Sybil attacker.
|<img src="https://render.githubusercontent.com/render/math?math=PM_{i}#gh-light-mode-only"><img src="https://render.githubusercontent.com/render/math?math=\color{white}PM_{i}#gh-dark-mode-only">|declared profit margin of operator `i`, defaults to 10%.
|<img src="https://render.githubusercontent.com/render/math?math=PF_{i}#gh-light-mode-only"><img src="https://render.githubusercontent.com/render/math?math=\color{white}PF_{i}#gh-dark-mode-only">|uptime of node `i`, scaled to 0 - 1, for the rewarding epoch
|<img src="https://render.githubusercontent.com/render/math?math=PP_{i}#gh-light-mode-only"><img src="https://render.githubusercontent.com/render/math?math=\color{white}PP_{i}#gh-dark-mode-only">|cost of operating node `i` for the duration of the rewarding epoch, set to 40 NYMs.

Node reward for node `i` is determined as:

<img src="https://render.githubusercontent.com/render/math?math=R_{i}=PF_{i} \cdot R \cdot (\sigma^'_{i} \cdot \omega_{i} \cdot k %2b \alpha \cdot \lambda^'_{i} \cdot \sigma^'_{i} \cdot k)/(1 %2b \alpha)#gh-light-mode-only">
<img src="https://render.githubusercontent.com/render/math?math=\color{white}R_{i}=PF_{i} \cdot R \cdot (\sigma^'_{i} \cdot \omega_{i} \cdot k %2b \alpha \cdot \lambda^'_{i} \cdot \sigma^'_{i} \cdot k)/(1 %2b \alpha)#gh-dark-mode-only">
where:

<img src="https://render.githubusercontent.com/render/math?math=\sigma^'_{i} = min\{\sigma_{i}, 1/k\}#gh-light-mode-only">
<img src="https://render.githubusercontent.com/render/math?math=\color{white}\sigma^'_{i} = min\{\sigma_{i}, 1/k\}#gh-dark-mode-only">

and

<img src="https://render.githubusercontent.com/render/math?math=\lambda^'_{i} = min\{\lambda_{i}, 1/k\}#gh-light-mode-only">
<img src="https://render.githubusercontent.com/render/math?math=\color{white}\lambda^'_{i} = min\{\lambda_{i}, 1/k\}#gh-dark-mode-only">

Operator of node `i` is credited with the following amount:

<img src="https://render.githubusercontent.com/render/math?math=min\{PP_{i},R_{i})\} %2b max\{0, (PM_{i} %2b (1 - PM_{i}) \cdot \lambda_{i}/\delta_{i}) \cdot (R_{i} - PP_{i})\}#gh-light-mode-only">
<img src="https://render.githubusercontent.com/render/math?math=\color{white}min\{PP_{i},R_{i})\} %2b max\{0, (PM_{i} %2b (1 - PM_{i}) \cdot \lambda_{i}/\delta_{i}) \cdot (R_{i} - PP_{i})\}#gh-dark-mode-only">

Delegate with stake `s` receives:

<img src="https://render.githubusercontent.com/render/math?math=max\{0, (1-PM_{i}) \cdot (s^'/\sigma_{i}) \cdot (R_{i} - PP_{i})\}#gh-light-mode-only">
<img src="https://render.githubusercontent.com/render/math?math=\color{white}max\{0, (1-PM_{i}) \cdot (s^'/\sigma_{i}) \cdot (R_{i} - PP_{i})\}#gh-dark-mode-only">

where `s'` is stake `s` scaled over total token circulating supply.

### Licensing and copyright information

This program is available as open source under the terms of the Apache 2.0 license. However, some elements are being licensed under CC0-1.0 and MIT. For accurate information, please check individual files.
