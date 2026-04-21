---
title: "Loopix Design"
description: "The academic Loopix mixnet design behind Nym: stratified topology, continuous-time mixing with exponential delays, and cover traffic loops for unlinkability and unobservability."
schemaType: "TechArticle"
section: "Network"
lastUpdated: "2026-03-15"
---

# Loopix Design

The Nym mixnet is based on the [Loopix](https://arxiv.org/pdf/1703.00536) design, with modifications for decentralized operation and economic incentives.

## The insight

Traditional mixnets focus on hiding "who messages whom," but this alone is insufficient, as adversaries observing message volume and timing over time can still infer private information. If you always message the same friend at the same time, patterns emerge. If you go silent when traveling, that's information too.

Loopix was designed to provide both **unlinkability** (hiding who talks to whom) and **unobservability** (hiding when and how much communication occurs). The name comes from its use of "loop" cover traffic that circulates through the network.

## Stratified topology

The network uses a layered architecture. Traffic flows through Entry Gateways, three Mix Node layers, and Exit Gateways. Each node connects only to adjacent layers. Path selection is independent per-message, unlike Tor's per-session circuits.

This structure prevents observations about which paths are used together and limits the damage any single compromised node can cause.

## Continuous-time mixing

Unlike batch mixnets that collect messages and release them periodically, Loopix uses continuous-time mixing, where each message is delayed independently according to an exponential distribution and then forwarded as soon as its delay expires.

This approach offers optimal anonymity for a given mean latency. The exponential distribution has a key property: if two messages arrive at different times, they have equal probability of leaving in either order. An adversary watching input and output timing gains no information about which input became which output.

Continuous mixing also means lower latency overall since messages don't wait for batches to fill.

## Cover traffic loops

Connected clients and nodes continuously generate dummy packets that travel in loops through the network back to the sender. These packets are indistinguishable from real traffic: same size, same encryption, same timing distribution.

Loop traffic ensures minimum anonymity even when few users are active, hides when real communication starts and stops, and enables detection of active attacks (if loop packets fail to return, a network fault or active interference is likely).

## Nym's modifications

The Nym implementation extends Loopix in several ways: replacing the trusted directory server with the Nyx blockchain for decentralized topology management, incentivising node operation with NYM token rewards rather than relying on volunteers, and adding zk-nyms for privacy-preserving payment, which the original academic design did not address.

## Security guarantees

The combination of continuous-time mixing and cover traffic provides provable guarantees. The anonymity set (the set of users who could have sent a given message) grows unboundedly over time. Even messages with short delays have large anonymity sets because of the exponential distribution.

An adversary observing the entire network cannot determine who is communicating with whom, cannot tell when real communication is occurring, and gains no advantage from statistical analysis because the traffic patterns are designed to be indistinguishable from random.

For the full formal analysis, see the [Loopix paper](https://arxiv.org/pdf/1703.00536) and the [Nym Whitepaper](https://nym.com/nym-whitepaper.pdf).
