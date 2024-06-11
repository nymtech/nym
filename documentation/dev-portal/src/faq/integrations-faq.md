# Integrations FAQ

On this page, you'll find links and frequently asked questions on how to get started on integrating your project with Nym's Mixnet and its blockchain, Nyx. 

## Links
### General Info 
* [Nym Website](https://nymtech.net/)
* [Nym Mixnet Explorer](https://explorer.nymtech.net/)
* [Nyx Block Explorer](https://nym.explorers.guru/)

### Codebase Info
* [Nym Platform Monorepo](https://github.com/nymtech/nym/)
* [Nym Project](https://github.com/nymtech/)

### Documentation Info
* [Documentation](https://nymtech.net/docs/) 
* Developer Portal - you are currently viewing the Developer Portal 

## Wallet Installation 
The Nym wallet can be downloaded [here](https://nymtech.net/download/). 

You can find all the instructions related to setting up your wallet in the [docs](https://nymtech.net/docs/wallet/desktop-wallet.html), as well as instructions on how to build the wallet if there is not a downloadable version built for your operating system. 

### What are the machine hardware requirements for Nym Wallet?
About 16GB of RAM is recommended for the wallet. However you can expect an average memory usage of ~100MB.


## Interacting with the Nyx blockchain 
### Where can I find information on the blockchain, such as RPC endpoints? 
You can find most information required for integration in the [Cosmos Chain Registry](https://github.com/cosmos/chain-registry/blob/master/nyx/chain.json) and [Keplr Chain Registry](https://github.com/chainapsis/keplr-chain-registry/blob/main/cosmos/nyx.json) repositories. 

### How can I use `JSON-RPC` methods to interact with the Nyx blockchain?
There are multiple ways to use `JSON-RPC` methods to interact with the Nyx blockchain. Which method you use will depend on the type of application you are integrating Nyx interactions into. 

1. The standalone `nyxd` binary can be used for CLI wallets, interacting with smart contracts via the CLI, setting up RPC nodes, and even running validators. This is a version of the Cosmos Hub's `gaiad` binary compiled with Nyx chain configuration, and is written in `Go`. Instructions on setting up the `nyxd` binary can be found [here](https://nymtech.net/docs/nyx/interacting-with-chain.html). This is recommended for more complex commands. For full documentation check the [`gaiad documentation`](https://hub.cosmos.network/main/hub-overview/overview.html#). 

2. `CosmJS` is a Typescript library allowing for developers to interact with CosmosSDK blockchains from a Javascript or Typescript project. You can find it on Github [here](https://github.com/cosmos/cosmjs) and an explainer of its functionality [in the Cosmos Developer Portal](https://tutorials.cosmos.network/tutorials/7-cosmjs/1-cosmjs-intro.html). You can find a list of example apps which use CosmJS [here](https://codesandbox.io/examples/package/@cosmjs/stargate). 

3. The `Nym-CLI` tool, a standalone rust binary which can be built and used according to the [docs](https://nymtech.net/docs/tools/nym-cli.html) can be used in much the same way as `nyxd`. It is a bit simpler to use than the `nyxd` binary, but is not recommended for complex queries, and not all commands are currently implemented. A list of Nym CLI commands and example usage can be found [here](https://nymtech.net/docs/tools/nym-cli.html) 

### How do I generate an address/mnemonic for users to interact with?
**Nyxd**

Use the following command, replacing `your_id` with the ID you want to use for your keypair:
```
./nyxd keys add your_id --chain-id=nyx --gas=auto --gas-adjustment=1.4 --fees=7000unym
```

**Nym-CLI**
```
./nym-cli account create
```

Both methods will generate a keypair and log the mnemonic in the console.

**CosmJS** 

You can find example code for keypair generation [here](https://tutorials.cosmos.network/tutorials/7-cosmjs/2-first-steps.html#testnet-preparation). 

### How to get block information like block height, block hash, block time as so on? 
**Nyxd**

You would use one of the subcommands returned by this command:
```
./nyxd query tx --chain-id=nyx --gas=auto --gas-adjustment=1.4 --fees=7000unym
```

**Nym-CLI**
```
./nym-cli block current-height
```

**CosmJS** 

`CosmJS` documentation can be found [here](https://cosmos.github.io/cosmjs/). We will be working on example code blocks soon. 

### How to get account/address balance to check there is enough coins to withdraw?
**Nyxd**
```
./nyxd query bank balances <address> --chain-id=nyx --gas=auto --gas-adjustment=1.4 --fees=7000unym
```

**Nym-CLI**
```
./nym-cli account balance
```

**CosmJS** 

`CosmJS` documentation can be found [here](https://cosmos.github.io/cosmjs/). We will be working on example code blocks soon. 

### How do I transfer tokens to another address? 
**Nyxd**
```
./nyxd tx bank send [from_key_or_address] [to_address] [amount] --chain-id=nyx --gas=auto --gas-adjustment=1.4 --fees=7000unym
```

**Nym-CLI**
```
./nym-cli account send TARGET_ADDRESS AMOUNT
```
**CosmJS** 

`CosmJS` documentation can be found [here](https://cosmos.github.io/cosmjs/). We will be working on example code blocks soon. 

### Does the address support the inclusion of a `memo` or `destinationTag` when doing the transfer?
Yes, it is supported.

### Can I use my Ledger hardware wallet to interact with the Nyx blockchain? 
Yes. Follow the instructions in the [Ledger support for Nyx documentation](https://nymtech.net/docs/nyx/ledger-live.html). 

### Where can I find network details such as deployed smart contract addresses? 
In the [`network defaults`](https://github.com/nymtech/nym/blob/master/common/network-defaults/src/mainnet.rs) file. 

## `NYM` Token 
The token used to reward mixnet infrastructure operators - `NYM` - is one of the native tokens of the Nyx blockchain. The other token is `NYX`. 

`NYM` is used to incentivise the mixnet, whereas `NYX` is used to secure the Nyx blockchain via Validator staking. 

> Integration with Nym's technology stack will most likely involve using `NYM` if you do need to interact with the Nyx blockchain and transfer tokens.

### I've seen an ERC20 representation of `NYM` on Ethereum - what's this and how do I use it? 

We use the [Gravity Bridge](https://github.com/Gravity-Bridge) blockchain to bridge an ERC20 representation of `NYM` between the Cosmos ecosystem of IBC-enabled chains and Ethereum mainnet. Gravity Bridge is its own IBC-enabled CosmosSDK chain, which interacts with a smart contract deployed on Ethereum mainnet. 

> The ERC20 representation of `NYM` **cannot** be used with the mixnet; only the native Cosmos representation is usable for staking or bonding nodes.

If you need to transfer tokens across the bridge, we recommend users use Cosmostation's [spacestation.zone](https://spacestation.zone/) dApp with Metamask and Keplr. 

### What is Circulating Supply and how to find out the distribution amount?

Circulating supply is the total number of available `NYM`. `NYM` is currently present on the IBC-enabled Nyx blockchain, as well as in ERC20 form on Ethereum Mainnet.

The Validator API endpoints can be found via the [Swagger Documentation](https://validator.nymtech.net/api/swagger/index.html). The following endpoints can be called to retrieve the correct distribution amount and circulating supply within Nym. 

Using this API endpoint returns information about the circulating supply of Nym tokens:

```
/circulating-supply
```
Query Response: 

	{
	    "total_supply": {
	        "denom": "unym",
	        "amount": "1000000000000000"
	    },
	    "mixmining_reserve": {
	        "denom": "unym",
	        "amount": "241105338883248"
	    },
	    "vesting_tokens": {
	        "denom": "unym",
	        "amount": "390255200928865"
	    },
	    "circulating_supply": {
	        "denom": "unym",
	        "amount": "368639460187887"
	    }
	}

- `total_supply`- The total number of NYM tokens that have been created and can exist, including those that are currently in circulation and those that are reserved for various purposes.

- `mixmining_reserved`- The number of NYM tokens that are reserved for the mixnet miners who help to power the Nym network.

- `vesting_tokens`- The number of NYM tokens that are subject to vesting, meaning they are gradually released over time to certain stakeholders such as the team, advisors, and early investors.

- `circulating_supply`- The number of NYM tokens that are currently in circulation and available to be traded on the open market, which is calculated by subtracting the `mixmining_reserved` and `vesting_tokens` from the `total_supply`.

Using this API endpoint returns the current value of the total supply of NYM tokens:

```
/circulating-supply/total-supply-value
```
Query Response:

	1000000000.0 

> The maximum number of `NYM` tokens that can ever be created is 1 billion.

Using this API endpoint returns the current value of the circulating supply of NYM tokens:

```
/circulating-supply/circulating-supply-value
```
Query Response:

	368639460.187887

> This refers to the present quantity of `NYM` tokens that are actively in circulation.


## Sending traffic through the Nym mixnet  
### Is the mixnet free to use? 
For the moment then yes, the mixnet is free to use. There are no limits on the amount of traffic that an app can send through the mixnet. 

### Do I need to run my own gateway to send application traffic through the mixnet? 
No, although we do recommend that apps that wish to integrate look into running some of their own infrastructure such as gateways in order to assure uptime. 

### How can I find out if an application is already supported by network requester services? 
You can check the [default allowed list](https://nymtech.net/.wellknown/network-requester/standard-allowed-list.txt) file to see which application traffic is whitelisted by default. If the domain is present on that list, it means that existing [network requesters](https://nymtech.net/docs/nodes/network-requester.html) can be used to privacy-protect your application traffic. 

Currently we are undergoing changes on this policy under the name [Project Smoosh](https://nymtech.net/operators/faq/smoosh-faq.html) where a new type of node [Exit Gateway](https://nymtech.net/operators/legal/exit-gateway.html) will allow users to connect to much wider range of domains, restricted by our new [exit policy](https://nymtech.net/.wellknown/network-requester/exit-policy.txt). Follow the changes [here](https://nymtech.net/operators/faq/smoosh-faq.html#what-are-the-changes).

