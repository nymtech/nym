# Nym-CLI

## What is this tool for?
This is a CLI tool for interacting with:

* the Nyx blockchain (account management, querying the chain state, etc)
* the smart contracts deployed on Nyx (bonding and un-bonding mixnodes, collecting rewards, etc)

It provides a convenient wrapper around the `nymd` client, and has similar functionality to the `nyxd` binary for querying the chain or executing smart contract methods.

## Building
The `nym-cli` binary can be built by running `cargo build --release` in the `nym/tools/nym-cli` directory.

## Usage
See the [commands](./nym-cli/commands.mdx) page for an overview of all command options.

### Staking on someone's behalf (for custodians)

There is a limitation the staking address can only perform the following actions (and are visible via the Nym Wallet:

- Bond on the gateway's or mix node's behalf.
- Delegate or Un-delegate (to a mix node in order to begin receiving rewards)
- Claiming the rewards on the account

```admonish note title=""
The staking address has no ability to withdraw any coins from the parent's account.
```

The staking address must maintain the same level of security as the parent mnemonic; while the parent mnemonic's delegations and bonding events will be visible to the parent owner, the staking address will be the only account capable of undoing the bonding and delegating from the mix nodes or gateway.

Query for staking on behalf of someone else
```
./nym-cli --mnemonic <staking address mnemonic>  mixnet delegators delegate --mix-id <input> --identity-key <input> --amount <input>
```
