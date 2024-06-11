# Querying the Chain
Now that all the code has been written, time to query the blockchain.

To test against the account operating one of the mix nodes on the testnet, use `n1lcutqz94k739s39u26rvexql40ehf42zd27fwe` in the following instructions:

```sh
# build the binaries
cargo build --release

# open two console windows. In one run the following
./target/release/service

# copy the printed Nym address from the console - this is used by the client when running the chain query

# in the other run
./target/release/client query-balance n1lcutqz94k739s39u26rvexql40ehf42zd27fwe <SERVICE_ADDRESS_FROM_CLIPBOARD>
```

The following happens:
* `client` and `service` both start their Nym clients and log the address. `service` is listening for incoming messages from the mixnet.
* `client` sends a request to `service` using the supplied `n1...` address as the Nyx account to query the balance of, and the supplied Nym address to communicate with this instance of `service`.
* `service` queries the Sandbox testnet blockchain using the `broadcaster` http client. It then serialises the response and returns it using SURBs to the `client`.

All in all, quite simple. By using the service as a proxy, the client never interacts with the blockchain, thus is not revealing metadata to the operator of the Validator nor any entities monitoring its incoming and outgoing traffic. Furthermore, the client doesn't even need to share its Nym address with the service, as the service is able to reply via SURBs.

## Creating Sandbox Account
If you wish to create an account on Sandbox to use instead of the supplied account above, the easiest way is by building the `nym-cli` tool and using that to create one:

```sh
# start from the root of the nym monorepo
cd tools/nym-cli
# build
cargo build --release
# create account using Sandbox testnet environment
../../target/release/nym-cli --config-env-file ../../envs/sandbox.env account create
```

However, since this account is fresh, it won't have any tokens. Querying the balance will still work obviously, it will just return `0`.

## Get Tokens
We're working on getting the faucet up and running again in preparation for part 2 of this tutorial: using the tokens you have privately checked the balance of to generate a [bandwidth credential](https://nymtech.net/docs/bandwidth-credentials.html).

If you wish to get testnet tokens already then feel free to ask in the [Dev channel](https://matrix.to/#/#dev:nymtech.chat) on Matrix.
