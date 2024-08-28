# Integrating with Nyx for payments

If you want to integrate with Nym in order to send `NYM` tokens (for instance, if running a `NYM` <-> `BTC` swap application, or using `NYM` for payments), then you will need to interact with the Nyx blockchain. 

Nyx is the blockchain supporting the Nym network, hosting both the `NYM` and `NYX` cryptocurrencies, the CosmWasm smart contracts keeping track of the network, and (coming soon) facilitating zk-Nym credential generation. It is built with the [Cosmos SDK](https://tendermint.com/sdk/).

### Interacting with the Nyx blockchain 
Check out the integration options in the [Integration FAQ](../faq/integrations-faq.md#how-can-i-use-json-rpc-methods-to-interact-with-the-nyx-blockchain). 

### Chain information and RPC endpoints 
You can find most information required for integration in the [Cosmos Chain Registry](https://github.com/cosmos/chain-registry/blob/master/nyx/chain.json) and [Keplr Chain Registry](https://github.com/chainapsis/keplr-chain-registry/blob/main/cosmos/nyx.json) repositories. 

## Recommended setup 
We recommend that users wanting to integrate with Nyx for cryptocurrency payments set up their own RPC Node, in order to be able to reliably query the blockchain and send transactions without having to worry about relying on 3rd party validators. 

The guide to setting up an RPC node can be found [here](https://nymtech.net/docs/nyx/rpc-node.html). 
