---
title: "Nyx Blockchain & Nym Smart Contracts"
description: "Developer guide for interacting with the Nyx blockchain via Cosmos SDK. Covers CLI wallet setup, Cosmos Registry, Ledger Live, and RPC node deployment."
schemaType: "TechArticle"
section: "Developers"
lastUpdated: "2026-02-11"
---

# Interacting with Nyx Chain and Smart Contracts

There are two options for interacting with the blockchain to send tokens or interact with deployed smart contracts:
* [`Nym-CLI`](./tools/nym-cli) tool
* `nyxd` binary

## Nym-CLI tool (recommended in most cases)
The `nym-cli` tool is a binary offering an interface for interacting with deployed smart contracts (e.g. bonding and unbonding a Mix Node from the CLI), creating and managing accounts and keypairs, sending tokens, and querying the blockchain.

See the [`nym-cli` docs page](./tools/nym-cli) for instructions.

## Nyxd binary
The `nyxd` binary, although harder to compile and use, offers the full range of commands available to users of CosmosSDK chains. Use this when you need more granular queries about transactions from the CLI.

The [`gaiad` docs page](https://hub.cosmos.network/main/delegators/delegator-guide-cli.html#querying-the-state) covers how to do this.
