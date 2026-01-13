# Epochs

Time in the Nym Network is organized into epochs—discrete periods during which certain network operations occur. The current epoch length is one hour.

## What happens at epoch boundaries

**Reward distribution** calculates performance metrics for each node and distributes NYM token rewards based on routing reliability and uptime. Nodes that successfully forward packets earn more than those with poor performance.

**Topology rerandomization** shuffles the arrangement of nodes in each layer. This prevents long-term route prediction attacks and limits the damage from any compromised nodes. Nodes may also enter or leave the active set based on uptime monitoring and stake changes.

## Future changes

In upcoming releases, epochs will trigger automatic role assignment. Nodes will switch between Mix Node and Gateway roles based on network demand, without operators needing to manually configure roles.

## SURB validity

Currently, SURBs remain valid across epoch boundaries since node keys don't change. When key rotation is implemented, SURBs will expire at epoch boundaries, and applications will need to handle this gracefully.

## Querying epoch information

Current epoch data is available through Nyx blockchain queries and Nym API endpoints.
