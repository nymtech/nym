<!--
Copyright 2023 - Nym Technologies SA <contact@nymtech.net>
SPDX-License-Identifier: Apache-2.0
-->

## Credential binary

The credential binary is used to acquire coconut bandwidth credentials in exchange for nym tokens. Those credentials are stored in the client's `data` directory, so that they can be used as the client sees fit.

### Warning

The credential binary is still experimental software. The infrastructure for using it is not yet deployed to mainnet and it's still in the process of being deployed to sandbox.

### Building

From the project's root directory, run:
```
cargo build -p nym-credential-client
```
which generates the `nym-credential-client` binary in `target/debug/nym-credential-client`.


### Running

For example, you can get a credential worth 3 nym (3000000 unym) in a socks5 client that was already initialized like so:

```
./target/debug/nym-credential-client --config-env-file envs/sandbox.env --client-home-directory  ~/.nym/socks5-clients/cred_client  --nyxd-url  https://sandbox-validator1.nymtech.net --mnemonic $MNEMONIC  --recovery-dir /tmp/recovery --amount 3000000
```

More information regarding how to run the binary can be found by running it with the `--help` argument.

