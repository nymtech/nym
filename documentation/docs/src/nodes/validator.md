# Validators

> The validator setup and maintenance guide has moved to the [Operator Guides book](https://nymtech.net/operators/nodes/validator-setup.html).

Validators secure the Nyx blockchain via Proof of Stake consensus. The Nyx blockchain records the ledger of `NYM` transactions and executes the smart contracts for distributing `NYM` rewards. The Nyx validators are run via the `nyxd` binary ([codebase](https://github.com/nymtech/nyxd)), maintaining a CosmWasm- and IBC-enabled blockchain. 

The blockchain plays a supporting but fundamental role in the mixnet: the `NYM` token used to incentivise node operators is one of two native tokens of the chain, and the chain is where the [Mixnet](../../../dev-portal/src/nyx/mixnet-contract.md) and [Vesting](../../../dev-portal/src/nyx/vesting-contract.md) smart contracts are deployed. 

## Further Reading 
* Detailed info on Nyx Validators and token flow can be found in [Nym Reward Sharing for Mixnets document](https://nymtech.net/nym-cryptoecon-paper.pdf) in section 2.3 and 2.4.
* Our [quarterly update](https://blog.nymtech.net/quarterly-token-economic-parameter-update-b2862948710f) on token economics from July 2023.
* [Nym Whitepaper](https://nymtech.net/nym-whitepaper.pdf) section 3.1 