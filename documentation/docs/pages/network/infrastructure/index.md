# Infrastructure

The Nym Network is supported by blockchain infrastructure that provides coordination, incentives, and credential management.

## Components

- **[Nyx Blockchain](./nyx)**: The Cosmos SDK chain that coordinates the network
- **[Nym Nodes](./nym-nodes)**: The unified binary that runs all network infrastructure

## Architecture

```
                    ┌─────────────────────────────────────────┐
                    │           Nyx Blockchain                │
                    │  ┌─────────────────────────────────┐    │
                    │  │ Mixnet Contract (topology)      │    │
                    │  │ Vesting Contract (tokens)       │    │
                    │  │ zk-nym Contract (credentials)   │    │
                    │  └─────────────────────────────────┘    │
                    └───────────────────┬─────────────────────┘
                                        │
                    ┌───────────────────┼───────────────────┐
                    │                   │                   │
                    ▼                   ▼                   ▼
            ┌───────────────┐   ┌───────────────┐   ┌───────────────┐
            │   Nym API     │   │   Nym API     │   │   Nym API     │
            │   (Quorum)    │   │   (Quorum)    │   │   (Quorum)    │
            └───────────────┘   └───────────────┘   └───────────────┘
                    │                   │                   │
                    └───────────────────┼───────────────────┘
                                        │
                                        ▼
            ┌─────────────────────────────────────────────────────┐
            │                   Nym Network                       │
            │  Gateways ◄──► Mix Nodes ◄──► Mix Nodes ◄──► Gateways │
            └─────────────────────────────────────────────────────┘
```

## Decentralization Model

The Nym infrastructure achieves decentralization through:

| Component | Decentralization Method |
|-----------|------------------------|
| Nyx Validators | Proof-of-Stake consensus |
| Nym API Quorum | Threshold cryptography (subset of validators) |
| Nym Nodes | Independent operators worldwide |
| Topology | On-chain registry (no central directory) |

## Economic Incentives

The NYM token aligns incentives across the network:

- **Node operators**: Earn rewards for routing traffic reliably
- **Validators**: Earn fees for securing the blockchain
- **Delegators**: Share in rewards by staking with operators
- **Users**: Pay for privacy-preserving network access

See the [Operator Documentation](../../operators) for details on running infrastructure.
