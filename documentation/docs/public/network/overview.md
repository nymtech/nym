---
title: Nym Network Overview
description: Introduction to the Nym Network, a privacy infrastructure that protects metadata including who communicates with whom, when, and how often.
url: https://nym.com/docs/network/overview
---

# Overview

The Nym Network is a privacy infrastructure that protects metadata: not just message content, but who is talking to whom, when, and how often. This section explains what the network does, why it exists, and how it compares to other approaches.

## In this section

- [The Privacy Problem](/network/overview/privacy-problem): what metadata is, why it matters, and what adversary models Nym is designed against
- [Choose a Defence](/network/threat-model/choose-config): compare configurations against each adversary, with use-case guidance
- [Nym vs Other Systems](/network/threat-model/comparisons): how Nym compares to VPNs, Tor, I2P, and E2EE

## Network Components

All traffic-routing infrastructure runs on [Nym Nodes](/network/infrastructure/nym-nodes), a single binary that operators configure to serve as an Entry Gateway, Mix Node, or Exit Gateway depending on their setup. Network coordination, token bonding, and the distributed credential system all live on the [Nyx blockchain](/network/infrastructure/nyx), a Cosmos SDK chain whose on-chain topology registry removes the need for a centralised directory server.
