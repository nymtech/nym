# Multisig Contract

The multisig contract used by the [NymAPI Quroum]() - a subset of the Nyx Validator set taking on the additional work of generating and validating zk-nyms - to execute certain actions in the [zk-nym](./ecash.md) contract.

It is essentially an instance of the [canonical](https://github.com/CosmWasm/cw-plus/tree/main/contracts) `cw3-flex-multisig` using the `cw4-group` contract, with one minor change to restrict the addresses allowed to submit proposals.
