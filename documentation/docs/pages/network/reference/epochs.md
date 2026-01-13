# Epochs

Time in the Nym Network is organized into **epochs**—discrete time periods during which certain network operations occur.

## Current Configuration

| Parameter | Value |
|-----------|-------|
| Epoch length | 1 hour |
| Configurable | Yes (network parameter) |

## Epoch Boundaries

At each epoch boundary, the network performs:

### Reward Distribution

- Calculate performance metrics for each node
- Distribute NYM token rewards based on:
  - Routing reliability (packets successfully forwarded)
  - Uptime during the epoch
  - Stake delegated to the node

See the [Operator Tokenomics](../../operators/tokenomics/mixnet-rewards) documentation for reward calculation details.

### Topology Rerandomization

- The arrangement of nodes in each layer is shuffled
- This prevents long-term route prediction attacks
- Nodes may enter or leave the active set based on:
  - Uptime monitoring results
  - Stake changes
  - Operator actions

### Active Set Updates

The "active set" is the subset of registered nodes that actively route traffic:

- Nodes with insufficient uptime may be removed
- Newly bonded nodes may be added
- Set size is limited to maintain network efficiency

## Future: Dynamic Role Assignment

In upcoming releases, epochs will also trigger:

- Automatic role changes (Mix Node ↔ Gateway)
- Based on network demand and performance
- Operators won't need to manually set roles

## SURB Validity

Currently, SURBs (Single Use Reply Blocks) are valid across epoch boundaries. However, future key rotation features will limit SURB validity to specific epochs:

- SURBs will expire when the signing keys rotate
- Applications should handle SURB expiration gracefully
- Exact validity period TBD (will be tied to key epoch)

## Monitoring Epochs

The network monitoring system operates in sync with epochs:

- Test packets are sent throughout each epoch
- Results are aggregated at epoch boundaries
- Node reliability scores are calculated per-epoch

## Querying Epoch Information

Current epoch information is available via:

- Nyx blockchain queries
- Nym API endpoints
- Client SDK methods
