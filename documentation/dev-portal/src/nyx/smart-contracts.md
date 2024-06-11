# Smart Contracts

The Nyx blockchain is based on [CosmWasm](https://cosmwasm.com/). It allows users to code smart contracts in a safe subset of the Rust programming language, easily export them to WebAssembly, and upload them to the blockchain. Information about the chain can be found on the [Nyx blockchain explorer](https://nym.explorers.guru/). 

There are currently two smart contracts on the Nyx chain: 
* the [Mixnet contract](mixnet-contract.md) which manages the network topology of the mixnet, tracking delegations and rewarding. 
* the [Vesting contract](vesting-contract.md) which manages `NYM` token vesting functionality.  

> Users will soon be able to create and upload their own CosmWasm smart contracts to Nyx and take advantage of applications such as the Coconut Credential Scheme - more to be announced regarding this very soon.
