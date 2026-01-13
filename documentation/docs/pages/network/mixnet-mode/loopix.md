# Loopix Design

The Nym mixnet is based on the [Loopix](https://arxiv.org/pdf/1703.00536) academic design, with modifications for decentralized operation and economic incentives.

## The insight

Traditional mixnets focus on hiding "who messages whom"—but this alone is insufficient. Adversaries observing message volume and timing over time can still infer private information. If you always message the same friend at the same time, patterns emerge. If you go silent when traveling, that's information too.

Loopix was designed to provide both **unlinkability** (hiding who talks to whom) and **unobservability** (hiding when and how much communication occurs). The name comes from its use of "loop" cover traffic that circulates through the network.

## Stratified topology

The network uses a layered architecture. Traffic flows through Entry Gateways, three Mix Node layers, and Exit Gateways. Each node connects only to adjacent layers. Path selection is independent per-message, unlike Tor's per-session circuits.

This structure prevents observations about which paths are used together and limits the damage any single compromised node can cause.

Topology management: [`common/topology`](https://github.com/nymtech/nym/tree/develop/common/topology)

## Continuous-time mixing

Unlike batch mixnets that collect messages and release them periodically, Loopix uses continuous-time mixing. Each message is delayed independently according to an exponential distribution, then forwarded as soon as its delay expires.

This approach offers optimal anonymity for a given mean latency. The exponential distribution has a key property: if two messages arrive at different times, they have equal probability of leaving in either order. An adversary watching input and output timing gains no information about which input became which output.

Continuous mixing also means lower latency overall since messages don't wait for batches to fill.

Delay configuration: [`common/nymsphinx/routing`](https://github.com/nymtech/nym/tree/develop/common/nymsphinx/routing)

## Cover traffic loops

Connected clients and nodes continuously generate dummy packets that travel in loops through the network back to the sender. These packets are indistinguishable from real traffic—same size, same encryption, same timing distribution.

Loop traffic ensures minimum anonymity even when few users are active. It hides when real communication starts and stops. And it can detect active attacks: if your loop packets don't return, something is interfering with the network.

Cover traffic generation: [`common/nymsphinx/cover`](https://github.com/nymtech/nym/tree/develop/common/nymsphinx/cover)

## Nym's modifications

The Nym implementation extends Loopix in several ways. The original design assumed a trusted directory server; Nym uses the Nyx blockchain for decentralized topology management. The original relied on volunteers; Nym provides NYM token rewards to ensure sustainable operation. And Nym adds zk-nyms for privacy-preserving payment—something the original academic design didn't address.

## Security guarantees

The combination of continuous-time mixing and cover traffic provides provable guarantees. The anonymity set—the set of users who could have sent a given message—grows unboundedly over time. Even messages with short delays have large anonymity sets because of the exponential distribution.

An adversary observing the entire network cannot determine who is communicating with whom. They cannot tell when real communication is occurring. And statistical analysis provides no advantage because the traffic patterns are designed to be indistinguishable from random.

For the full formal analysis, see the [Loopix paper](https://arxiv.org/pdf/1703.00536) and the [Nym Whitepaper](https://nym.com/nym-whitepaper.pdf).
