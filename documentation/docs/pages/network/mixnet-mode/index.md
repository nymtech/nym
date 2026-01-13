# Mixnet Mode

Mixnet mode provides maximum privacy by routing traffic through 5 hops with packet mixing, timing delays, and cover traffic. It is available through [NymVPN](https://nymvpn.com) and the [Nym SDKs](../../developers).

## Overview

The mixnet is designed to protect against Global Passive Adversaries (GPAs)—entities capable of observing the entire network. It achieves this through:

- **5-hop routing**: Traffic passes through Entry Gateway → 3 Mix Node layers → Exit Gateway
- **Packet mixing**: Each Mix Node delays packets randomly, destroying timing correlations
- **Cover traffic**: Constant stream of dummy packets hides when real communication occurs
- **Uniform packets**: All packets are identical in size using the Sphinx format

```
┌──────┐     ┌───────┐     ┌─────────┐     ┌─────────┐     ┌─────────┐     ┌──────┐     ┌──────────┐
│ User │────▶│ Entry │────▶│ Mix L1  │────▶│ Mix L2  │────▶│ Mix L3  │────▶│ Exit │────▶│ Internet │
└──────┘     └───────┘     └─────────┘     └─────────┘     └─────────┘     └──────┘     └──────────┘
                               │               │               │
                          Random delay    Random delay    Random delay
                          + mixing        + mixing        + mixing
```

## Privacy Properties

### Unlinkability

An observer cannot link:
- An incoming packet to an outgoing packet at any node
- Successive packets from the same user
- A user's activity across different sessions

### Unobservability

An observer cannot determine:
- When real communication is occurring
- The volume of real traffic vs cover traffic
- Whether a particular user is actively communicating

### Resistance to Traffic Analysis

The mixnet defeats:
- **Timing correlation**: Random delays break timing patterns
- **Volume analysis**: Cover traffic masks real traffic volume
- **Fingerprinting**: Uniform packet sizes prevent content-type inference
- **Long-term statistical attacks**: Per-packet routing and rerandomization

## How It Works

### 1. Route Selection

For each packet, the client independently selects:
- One Entry Gateway (typically the client's registered gateway)
- One node from Mix Layer 1
- One node from Mix Layer 2
- One node from Mix Layer 3
- One Exit Gateway

Routes are selected randomly per-packet, not per-session.

### 2. Sphinx Packet Construction

The client constructs a [Sphinx packet](../cryptography/sphinx) with layered encryption:

```
┌─────────────────────────────────────────────────────────────┐
│ Layer 1: Encrypted for Entry Gateway                        │
│  ┌───────────────────────────────────────────────────────┐  │
│  │ Layer 2: Encrypted for Mix Layer 1                    │  │
│  │  ┌─────────────────────────────────────────────────┐  │  │
│  │  │ Layer 3: Encrypted for Mix Layer 2              │  │  │
│  │  │  ┌───────────────────────────────────────────┐  │  │  │
│  │  │  │ Layer 4: Encrypted for Mix Layer 3        │  │  │  │
│  │  │  │  ┌─────────────────────────────────────┐  │  │  │  │
│  │  │  │  │ Layer 5: Encrypted for Exit Gateway │  │  │  │  │
│  │  │  │  │  ┌───────────────────────────────┐  │  │  │  │  │
│  │  │  │  │  │ Payload (for recipient)       │  │  │  │  │  │
│  │  │  │  │  └───────────────────────────────┘  │  │  │  │  │
│  │  │  │  └─────────────────────────────────────┘  │  │  │  │
│  │  │  └───────────────────────────────────────────┘  │  │  │
│  │  └─────────────────────────────────────────────────┘  │  │
│  └───────────────────────────────────────────────────────┘  │
└─────────────────────────────────────────────────────────────┘
```

Each layer contains:
- Routing information for the next hop
- Integrity verification (HMAC)
- The inner encrypted packet

### 3. Transmission with Cover Traffic

The client maintains a constant rate of packet transmission:

```
Time ─────────────────────────────────────────────────────────▶

     Cover  Cover  Real   Cover  Cover  Real   Cover  Cover
       │      │      │      │      │      │      │      │
       ▼      ▼      ▼      ▼      ▼      ▼      ▼      ▼
     ┌────┐┌────┐┌────┐┌────┐┌────┐┌────┐┌────┐┌────┐
     │ C  ││ C  ││ R  ││ C  ││ C  ││ R  ││ C  ││ C  │
     └────┘└────┘└────┘└────┘└────┘└────┘└────┘└────┘
       │      │      │      │      │      │      │      │
       └──────┴──────┴──────┴──────┴──────┴──────┴──────┘
                              │
                    All packets identical
                    (Indistinguishable)
```

### 4. Mixing at Each Hop

Each Mix Node:
1. Receives incoming packets
2. Decrypts its layer, revealing the next hop
3. Verifies the HMAC
4. Adds a random delay (exponential distribution)
5. Forwards to the next hop

The random delay is critical: it ensures that the order of outgoing packets is unrelated to the order of incoming packets.

### 5. Delivery and Anonymous Replies

At the Exit Gateway:
- Packets destined for external services are forwarded
- Packets for other Nym clients are delivered to their gateways
- [SURBs](./anonymous-replies) enable anonymous replies without revealing the sender

## Performance Characteristics

| Metric | Typical Value |
|--------|---------------|
| Additional latency | 200-500ms |
| Packet size | 2048 bytes (Sphinx payload) |
| Cover traffic rate | Configurable (Poisson process) |

The latency is primarily due to:
- Mixing delays at each of 3 Mix Node layers
- Network round-trip time across 5 hops

## Use Cases

Mixnet mode is designed for scenarios where maximum privacy is required:

- **Journalism**: Protecting sources and communications
- **Activism**: Organizing in surveillance-heavy environments
- **Whistleblowing**: Anonymous disclosure
- **Sensitive research**: Medical, legal, financial privacy
- **High-value communications**: Where metadata exposure is unacceptable

## Access Methods

### NymVPN

The NymVPN application provides mixnet mode as the "Anonymous" or "5-hop" option. This wraps IP traffic for transparent mixnet routing.

### Developer SDKs

The [Nym SDKs](../../developers) provide direct mixnet access for application developers:

- Native Rust SDK
- TypeScript/WASM SDK
- Message-based API

SDK access is currently free and provides the same privacy guarantees as NymVPN's mixnet mode.

## Further Reading

- [Loopix Design](./loopix): The academic foundation for Nym's mixnet
- [Traffic Flow](./traffic-flow): Detailed packet journey through the network
- [Cover Traffic](./cover-traffic): How dummy traffic provides unobservability
- [Packet Mixing](./mixing): How timing delays defeat correlation
- [Anonymous Replies](./anonymous-replies): SURBs for bidirectional anonymous communication
