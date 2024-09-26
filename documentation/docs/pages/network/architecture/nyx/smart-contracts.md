# Smart Contracts

The Nyx blockchain is based on [CosmWasm](https://cosmwasm.com/).

The following contracts are deployed to the chain:
* the [Mixnet contract](./mixnet-contract.md) which manages the network topology of the mixnet and tracks delegations & rewarding.
* the [Vesting contract](./vesting-contract.md) which manages `NYM` token vesting functionality. This will soon be deprecated.
* the [Quorum Multisig](./multisig.md) used by the subset of the Nyx Validators that generate and validate [zk-nyms](../../cryptography/zk-nym) to manage reward payouts for nodes.
* the [zk-nym contract](./ecash.md) which keeps track of `NYM` deposits used as payment for zk-nym generation.
