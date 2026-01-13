# Loopix Design

The Nym mixnet is based on the [Loopix](https://arxiv.org/pdf/1703.00536) academic design, with modifications for decentralized operation and economic incentives.

## Core Principles

> Hiding "who messages whom" is a necessary mixnet property in terms of metadata protection – but it is not always sufficient to prevent surveillance. Adversaries that observe the volume and timing of sent and received messages over time may still be able to infer private information, even if individual messages are strongly anonymized.
>
> — [Nym Whitepaper](https://nym.com/nym-whitepaper.pdf) §4.6

Loopix was designed to provide both **unlinkability** (hiding who talks to whom) and **unobservability** (hiding when and how much communication occurs).

## Stratified Topology

The network uses a layered (stratified) architecture:

```
        Entry Gateways
              │
              ▼
    ┌─────────────────┐
    │   Mix Layer 1   │
    └─────────────────┘
              │
              ▼
    ┌─────────────────┐
    │   Mix Layer 2   │
    └─────────────────┘
              │
              ▼
    ┌─────────────────┐
    │   Mix Layer 3   │
    └─────────────────┘
              │
              ▼
        Exit Gateways
```

Key properties:
- Traffic flows in one direction through the layers
- Each node only connects to adjacent layers
- Path selection is independent per-message (unlike Tor's circuits)

## Continuous-Time Mixing

Unlike batch mixnets that collect messages and release them periodically, Loopix uses **continuous-time mixing**:

- Each message is delayed independently
- Delays follow an exponential distribution
- Messages are forwarded as soon as their delay expires

This provides:
- Lower, more predictable latency
- Larger anonymity sets
- Better bandwidth utilization

## Cover Traffic Loops

The name "Loopix" comes from its use of **loop cover traffic**:

```
┌────────┐         ┌─────────┐         ┌─────────┐         ┌─────────┐
│ Client │────────▶│ Mix L1  │────────▶│ Mix L2  │────────▶│ Mix L3  │
└────────┘         └─────────┘         └─────────┘         └─────────┘
    ▲                                                           │
    │                                                           │
    └───────────────────────────────────────────────────────────┘
                         Loop back to sender
```

Loop traffic:
- Circulates through the network back to the sender
- Is indistinguishable from real traffic
- Provides a baseline of activity even when idle
- Can detect active attacks on the network

## Modifications in Nym

The Nym implementation extends Loopix with:

### Decentralized Topology

- Original Loopix assumed a trusted directory
- Nym uses the Nyx blockchain for decentralized topology management
- No central authority controls node registration

### Economic Incentives

- Original Loopix relied on volunteer operators
- Nym provides NYM token rewards for node operation
- Incentives ensure long-term network health and growth

### Gateway Architecture

- Loopix used "providers" for message delivery
- Nym separates Entry and Exit Gateway functions
- Gateways handle credential verification (zk-nyms)

### Anonymous Credentials

- Original Loopix had no payment mechanism
- Nym adds zk-nyms for privacy-preserving access control
- Payment cannot be linked to network activity

## Security Analysis

Loopix (and by extension, Nym) provides provable security guarantees:

### Anonymity Set

The anonymity set includes all users who could have sent a given message. With continuous-time mixing:
- The set grows unboundedly over time
- Even messages with short delays have large anonymity sets
- The exponential distribution maximizes entropy

### Resistance to Timing Attacks

> If we observe two messages going into the mix node at times t₀ < t₁, and a message coming out at a later time t₂, the probability that the output is any of the two inputs is equal, regardless of differences in their arrival times t₀ and t₁.
>
> — [Nym Whitepaper](https://nym.com/nym-whitepaper.pdf) §4.4

### Unobservability

Cover traffic ensures that:
- Observers cannot distinguish active from idle users
- Traffic volume does not leak information
- Statistical patterns do not emerge over time

## Academic Reference

For the complete formal analysis, see:

- [Loopix Paper](https://arxiv.org/pdf/1703.00536): Original design and proofs
- [Nym Whitepaper](https://nym.com/nym-whitepaper.pdf): Nym-specific modifications
- [Sphinx Paper](https://cypherpunks.ca/~iang/pubs/Sphinx_Oakland09.pdf): Packet format specification
