# Nym CLI

This is a CLI tool for interacting with:

- the Nyx blockchain
- the smart contracts for the Mixnet

It provides a convenient wrapper around the [`nyxd client`](../../common/client-libs) with similar functionality to the`nyxd` binary for querying the chain or executing smart contract methods.

And in the future it will provide an easy way to interact with Coconut, to issue and verify Coconut credenitals.

### It DOES NOT do these things:

The infrastructure components that run a [`gateway`](../../gateway), [`mixnode`](../../mixnode) or Service Provider have their own binaries.

The [`socks5`](../../common/socks5) client also has its own binary, or use [NymConnect](../../nym-connect).

# Installing

Download the CLI binary for your platform from https://nymtech.net/downloads or get a specific version from [GitHub releases](https://github.com/nymtech/nym/releases?q=nym-cli&expanded=true).

# Configuration

The Nym CLI runs against mainnet by default.

If you want to use another environment, you can do this by:
- providing a `.env` file
- setting environment variables ([see here for options](../../common/network-defaults/envs/mainnet.env))
- passing named arguments

### `.env` File

There are two ways to provide this:

1. A file called `.env` in the same directory as the binary
2. Pass the `--config-env-file` along with a command

### Passing named arguments

You will need to pass the following with most commands as arguments:

```
--mnemonic <MNEMONIC>                    
--nyxd-url <nyxd_URL>                    
--mixnet-contract <MIXNET_CONTRACT_ADDRESS>      
--vesting-contract <VESTING_CONTRACT_ADDRESS>
```

# How do I use it?

The simplest way to find out how to use the CLI is to explore the built-in help:

```
nym-cli --help
```

# Features

### 🏦 Account

- create a new account with a random mnemonic
- query the account balance
- query the account public key (needed to verify signatures)
- query for transactions originating from the account
- send tokens to another account

### ⛓ Block

- query for the current block height
- query for a block at a height
- query for a block at a timestamp

### 🪐 cosmwasm

- upload a smart contract
- instantiate a smart contract
- upgrade a smart contract
- execute a smart contract method

### 𐄳 Mixnet

#### 📒 Directory

- query for mixnodes
- query for gateways

#### 🧑‍🔧 Operators

- bond/unbond a mixnode or gateway
- query for waiting rewards
- withdraw rewards
- manage mixnode settings
- create payload for family creation signature 
- create family

#### 🥩 Delegators

- delegate/undelegate to a mixnode
- query for waiting rewards
- withdraw rewards

### ✍ Sign

- create a signature for string data (UTF-8)
- verify a signature for an account

### 🕓 Vesting
- create a vesting schedule
- query for a vesting schedule

### 🥥 Coconut

Coming soon, including:

- issue credential
- verify credential

# Building

Build the tool locally by running the following in this directory:

```
cargo build --release
```

# Generating user docs

There is a [Makefile](./Makefile) with a target to build the user docs:

```
make generate-user-docs
```

Build the tool and run the `generate` command:

```
cargo build --release
../../target/release/nym-cli generate-fig > user-docs/fig-spec.ts
```

See https://github.com/withfig/autocomplete-tools/tree/main/types.