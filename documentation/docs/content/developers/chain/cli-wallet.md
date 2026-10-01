# CLI Wallet

If you have read our [validator setup and maintenance documentation](../../operators/nodes/validator-setup), you will have seen that we compile and use the `nyxd` binary primarily for our validators. This binary can also be used for other tasks, such as creating and using keypairs for wallets, or automated setups that require the signing and broadcasting of transactions.

### Using `nyxd` binary as a CLI wallet
You can use `nyxd` as a minimal CLI wallet if you want to set up one or more accounts. Compile the binary as per the documentation, stopping after the [building your validator](../../operators/nodes/validator-setup#building-your-validator) step is complete. Then run `nyxd keys --help` to see how to set up and store keypairs for interacting with the Nyx blockchain.
