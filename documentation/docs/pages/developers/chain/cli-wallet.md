# CLI Wallet

If you have already read our [validator setup and maintenance documentation](../../operators/nodes/validator-setup) you will have seen that we compile and use the `nyxd` binary primarily for our validators. This binary can however be used for many other tasks, such as creating and using keypairs for wallets, or automated setups that require the signing and broadcasting of transactions.

### Using `nyxd` binary as a CLI wallet
You can use the `nyxd` as a minimal CLI wallet if you want to set up an account (or multiple accounts). Just compile the binary as per the documentation, **stopping after** the [building your validator](../../operators/nodes/validator-setup#building-your-validator) step is complete. You can then run `nyxd keys --help` to see how you can set up and store different keypairs with which to interact with the Nyx blockchain.
