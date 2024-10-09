# Interacting with Nyx Chain and Smart Contracts

There are two options for interacting with the blockchain to send tokens or interact with deployed smart contracts:
* [`Nym-CLI`](../tools/nym-cli.md) tool
* `nyxd` binary

## Nym-CLI tool (recommended in most cases)
The `nym-cli` tool is a binary offering a simple interface for interacting with deployed smart contract (for instance, bonding and unbonding a mix node from the CLI), as well as creating and managing accounts and keypairs, sending tokens, and querying the blockchain.

Instructions on how to do so can be found on the [`nym-cli` docs page](./tools/nym-cli)

## Nyxd binary
The `nyxd` binary, although more complex to compile and use, offers the full range of commands availiable to users of CosmosSDK chains. Use this if you are (e.g.) wanting to perform more granular queries about transactions from the CLI.

You can use the instructions on how to do this on from the [`gaiad` docs page](https://hub.cosmos.network/main/delegators/delegator-guide-cli.html#querying-the-state).
