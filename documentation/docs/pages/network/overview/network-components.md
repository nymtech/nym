# Network Components

The Nym Network is a decentralized infrastructure consisting of several types of components working together to provide privacy.

## Architecture Overview

```
                                    ┌─────────────────────────────────────────┐
                                    │           Nyx Blockchain                │
                                    │  (Topology, Staking, Credentials)       │
                                    └─────────────────────────────────────────┘
                                                       │
                                                       ▼
┌──────────┐      ┌───────────────────────────────────────────────────────────────────┐      ┌──────────┐
│          │      │                        Nym Network                                │      │          │
│   User   │─────▶│  Entry GW ───▶ Mix L1 ───▶ Mix L2 ───▶ Mix L3 ───▶ Exit GW       │─────▶│ Internet │
│          │      │                                                                   │      │          │
└──────────┘      └───────────────────────────────────────────────────────────────────┘      └──────────┘
                                                       │
                                                       ▼
                                    ┌─────────────────────────────────────────┐
                                    │              Nym API                    │
                                    │   (Monitoring, Credential Issuance)    │
                                    └─────────────────────────────────────────┘
```

## Nym Nodes

All traffic-routing infrastructure runs on **Nym Nodes**—a unified binary that can operate in different modes. This simplifies operation and enables future dynamic role assignment.

### Entry Gateways

Entry Gateways are the user's first point of contact with the network.

**Responsibilities:**
- Accept client connections via WebSocket
- Verify zk-nym credentials (proof of payment)
- Store messages for offline clients (up to 24 hours)
- Forward traffic into the mix node layers

**Properties:**
- Knows the client's IP address
- Cannot see message contents or final destination
- Cannot link client identity to payment (due to zk-nyms)

### Mix Nodes

Mix Nodes form the core privacy layer of the network, arranged in three layers.

**Responsibilities:**
- Receive Sphinx packets from the previous hop
- Remove one layer of encryption
- Apply a random delay (exponential distribution)
- Forward to the next hop

**Properties:**
- Cannot see packet contents
- Cannot determine position in route
- Cannot link incoming packets to outgoing packets (due to mixing)

### Exit Gateways

Exit Gateways are the final hop before traffic reaches external services.

**Responsibilities:**
- Receive traffic from Mix Layer 3
- Communicate with external internet services
- Return responses through the network
- For mixnet mode: store messages for receiving clients

**Properties:**
- Can see destination addresses (like a Tor exit node)
- Cannot see the original sender
- Cannot link requests to specific users

## Nyx Blockchain

Nyx is a Cosmos SDK blockchain that provides coordination services for the network.

**Functions:**
- **Topology registry**: Maintains the list of active nodes and their public keys
- **Staking**: Manages NYM token bonding for node operators
- **Rewards**: Distributes rewards based on node performance
- **Credential contracts**: Manages zk-nym deposit and redemption

**Key Contracts:**
- Mixnet Contract: Node registration and topology
- Vesting Contract: Token vesting schedules
- zk-nym Contract: Credential payment tracking

## Nym API

The Nym API is operated by a subset of Nyx validators (the "Quorum").

**Functions:**
- **Network monitoring**: Measures node reliability and performance
- **Credential issuance**: Generates partial signatures for zk-nyms
- **Double-spend protection**: Maintains global bloom filter of spent credentials
- **Reward calculation**: Determines node operator payouts

## Decentralization Properties

| Component | Trust Requirement |
|-----------|-------------------|
| Entry Gateway | Knows your IP, not your activity |
| Mix Nodes | No single node can deanonymize |
| Exit Gateway | Sees destination, not source |
| Nyx Blockchain | Decentralized via validator set |
| Nym API Quorum | Threshold signature (no single authority) |

**Key insight**: No single component has enough information to break privacy. Even if some nodes are malicious, the network remains secure as long as at least one honest node exists on each route.

## Node Selection

For mixnet mode, routes are selected randomly and independently for each packet:
- One Entry Gateway (typically client's registered gateway)
- One node from each of the three Mix Node layers
- One Exit Gateway

This means successive packets from the same user take different paths, preventing traffic analysis based on route observation.

## Network Scale

As of the current deployment:
- 600+ active nodes
- ~60 countries
- Independent operators worldwide
- Decentralized operation with no central authority

For information on running a node, see the [Operator Documentation](../../operators).
