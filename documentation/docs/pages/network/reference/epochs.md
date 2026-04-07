---
title: "Epochs in the Nym Network"
description: "How epochs organize time in the Nym Network: reward distribution, topology reshuffling, SURB validity windows, and future automatic role assignment."
schemaType: "TechArticle"
section: "Network"
lastUpdated: "2026-03-15"
---

# Epochs

Time in the Nym Network is organized into epochs: discrete periods during which certain network operations occur. The current epoch length is one hour.

## What happens at epoch boundaries

**Reward distribution** calculates performance metrics for each node and distributes NYM token rewards based on routing reliability and uptime. Nodes that successfully forward packets earn more than those with poor performance.

**Topology rerandomization** shuffles the arrangement of nodes in each layer. This prevents long-term route prediction attacks and limits the damage from any compromised nodes. Nodes may also enter or leave the active set based on uptime monitoring and stake changes.

## Future changes

In upcoming releases, epochs will trigger automatic role assignment. Nodes will switch between Mix Node and Gateway roles based on network demand, without operators needing to manually configure roles.

## SURB validity

SURBs are tied to key rotation cycles. Node keys rotate on an odd/even schedule with a default validity of 24 epochs. A SURB remains usable for `(validity_epochs + 1) * epoch_duration`, roughly 25 hours at the current 1-hour epoch. After that, the routing keys it was built with are no longer accepted by the network. Clients automatically purge stale SURBs and request fresh ones.

## Querying epoch information

Current epoch data is available through Nyx blockchain queries and Nym API endpoints.
