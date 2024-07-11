# Testnet manager

This is extremely experimental tool. Only to be used internally. Expect a lot of breaking changes.

Currently (as of 11.07.24), it exposes the following commands:

## `build-info`

Show build information of this binary. Does it need any more than that?

## `initialise-new-network`

pre-requisites:

1. you must have built all nym-contracts and put them in the same directory (just run `make contracts` from the root
   directory)

Initialises new testnet network:

1. attempts to retrieve paths to all .wasm files of the nym-contracts based on provided arguments
2. uploads all the contracts to the specified nyxd
3. creates mnemonics for all contract admins
4. transfers some tokens to each created account
5. instantiates all the contracts
6. performs post-instantiation migration (like sets vesting contract address inside the mixnet contract)
7. queries each contract and retrieves its build information to display any warnings if they were built using some
   ancient commits
8. persists all the network info (addresses, mnemonics, etc.) in the database for future use

**note: if you intend to `bond-local-mixnet` afterward, you want to set `--custom-epoch-duration-secs` to a rather low
value (like 60s)**

## `load-network-details`

Attempt to load testnet network details using either the provided name, or if nothing was specified, the latest one
created.

It outputs contents of an `.env` file you'd use with that network.

## `bypass-dkg`

pre-requisites:

1. you must have built the `dkg-bypass contract` (just run `make build-bypass-contract` from **this** directory)

Attempts to bypass the DKG by overwriting the contract state with pre-generated keys:

1. generates data for each specified ecash signer:
    - ecash keys via a ttp
    - ed25519 identity keys
    - cosmos mnemonic
2. validates the existing DKG contract to make sure the DKG hasn't actually already been run and checks the group
   contract to make sure its empty
3. persists the signer data generated at the beginning
4. uploads the bypass contract
5. overwrites the contract state (endpoints, keys, etc.) using the uploaded contract
6. restores the original DKG contract code
7. adds the ecash signers to the CW4 group
8. transfers some tokens to each ecash signer so they could actually execute txs

## `initialise-post-dkg-network`

pre-requisites:

1. you must have built all nym-contracts and put them in the same directory (just run `make contracts` from the root
   directory)
2. you must have built the `dkg-bypass contract` (just run `make build-bypass-contract` from **this** directory)

Initialises new network and bypasses the DKG. It's just the equivalent of running `initialise-new-network`
and `bypass-dkg` separately:

1. runs equivalent of `initialise-new-network`
2. runs equivalent of `bypass-dkg`

## `create-local-ecash-apis`(local_ecash_apis::Args),

pre-requisites:

1. you must have built all nym-contracts and put them in the same directory (just run `make contracts` from the root
   directory)
2. you must have built the `dkg-bypass contract` (just run `make build-bypass-contract` from **this** directory)
3. you must have built `nym-api` binary

Attempt to create brand new network, in post DKG-state, using locally running nym-apis.

1. runs equivalent of `initialise-post-dkg-network`, with one difference: rather than requiring you to provide api
   endpoints to all signers, it defaults to `http:://127.0.0.1:X`, where `X = 10000 + i`, based on the number of apis
   specified in the args
2. runs `nym-api init` for all required api
3. copies over keys generated during `bypass-dkg` into the correct path for each API,
4. generates an `.env` file to use in all subsequent `run` commands
5. generates and outputs (either as raw string or `json` if used with `--output=json`) run commands for each nym-api
   using full canonical and absolute paths (so you could paste them regardless of local directory)

## `bond-local-mixnet`

pre-requisites:

1. you must have a running network **including nym-api** (just run `create-local-ecash-apis` and start the binaries)
2. the mixnet epoch must be waiting for transition (thus `--custom-epoch-duration-secs` recommendation)
3. you must have built `nym-node` binary

Attempt to bond minimal local mixnet (3 mixnodes + 1 gateways) and output the run commands.

1. runs `nym-node init` 4 times, including once in `mode==entry` (with credentials)
2. generates mnemonics for each node
3. generates bonding signatures for each node
4. transfers some tokens to each bond owner
5. performs bonding of mixnode/gateway
6. assigns all nodes to the active set by:
    - starting epoch transition
    - reconciling epoch events
    - advancing current epoch and assigning the nodes to the set
7. generates and outputs (either as raw string or `json` if used with `--output=json`) run commands for each nym-node
   using full canonical and absolute paths (so you could paste them regardless of local directory)

## `create-local-client`

pre-requisites:

1. you must have a running MIXNET **including nym-api AND nym-nodes** (just run `create-local-ecash-apis` followed
   by `bond-local-mixnet` and start the binaries)
2. you must have built `nym-client` binary

Initialise a locally run nym-client, adjust its config and output the run command:

1. runs `nym-client init` in credentials mode
2. updates its config to add `minimum_mixnode_performance = 0` and `minimum_gateway_performance = 0` thus ignoring the
   lack of a network monitor
3. generates and outputs run command for the client using full canonical and absolute paths (so you could paste it
   regardless of local directory)

### Extra

For reference, my workflow was as follows:

note: for the very first run you'll have to explicitly provide mnemonics and nyxd

1. rebuild whichever binary/contract was needed
2. `cargo run -- create-local-ecash-apis --bypass-dkg-contract ../../../target/wasm32-unknown-unknown/release/dkg_bypass_contract.wasm --number-of-apis=2 --nym-api-bin ../../../target/release/nym-api --built-contracts ../../../contracts/target/wasm32-unknown-unknown/release --custom-epoch-duration-secs=60`
3. run the apis in separate terminal window
4. `cargo run -- bond-local-mixnet --nym-node-bin ../../../target/release/nym-node`
5. start all the nym-nodes
6. `cargo run -- create-local-client --nym-client-bin ../../../target/debug/nym-client`
7. usually at this point I was using `nym-cli` to get some ticketbooks into my client before running it with the command
   that was output in the previous step




