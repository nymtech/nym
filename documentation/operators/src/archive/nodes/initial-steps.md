# Preliminary Steps

```admonish warning
**This is an archived page for backwards compatibility. The content of this page is not updated since April 19th 2024. Eventually this page will be terminated!**
```

> The Nym `mixnode`, `gateway` and `network-requester` binaries were built in the [building nym](../../binaries/building-nym.md) section. If you haven't yet built Nym and want to run the code, go there first.

There are a couple of steps that need completing before starting to set up your mix node, gateway or a network requester:

- preparing your [desktop wallet](https://nymtech.net/docs/wallet/desktop-wallet.html) or [CLI wallet](https://nymtech.net/docs/wallet/cli-wallet.html).
- requisitioning a VPS (Virtual Private Server)

### Wallet preparation
#### Mainnet
Before you initialise and run your mix node, head to our [website](https://nymtech.net/download/) and download the Nym wallet for your operating system. If pre-compiled binaries for your operating system aren't available, you can build the wallet yourself with instructions [here](https://nymtech.net/docs/wallet/desktop-wallet.html).

If you don't already have one, please create a Nym address using the wallet, and fund it with tokens. The minimum amount required to bond a mix node is 100 `NYM`, but make sure you have a bit more to account for gas costs.

`NYM` can be purchased via Bity from the wallet itself with BTC or fiat, and is currently present on several [exchanges](https://www.coingecko.com/en/coins/nym#markets).

> Remember that you can **only** use Cosmos `NYM` tokens to bond your mix node. You **cannot** use ERC20 representations of `NYM` to run a node.


#### Sandbox testnet
Make sure to download a wallet and create an account as outlined above. Then head to our [token faucet](https://faucet.nymtech.net/) and get some tokens to use to bond it.

### VPS Hardware Specs
You will need to rent a VPS to run your node on. One key reason for this is that your node **must be able to send TCP data using both IPv4 and IPv6** (as other nodes you talk to may use either protocol).

For the moment, we haven't put a great amount of effort into optimizing concurrency to increase throughput, so don't bother provisioning a beastly server with multiple cores. This will change when we get a chance to start doing performance optimizations in a more serious way. Sphinx packet decryption is CPU-bound, so once we optimize, more fast cores will be better.

For now, see the below rough specs:

- Processors: 2 cores are fine. Get the fastest CPUs you can afford.

#### For mix node

- RAM: Memory requirements are very low - typically a mix node may use only a few hundred MB of RAM.
- Disks: The mixnodes require no disk space beyond a few bytes for the configuration files.

#### For Gateway

- RAM: Memory requirements depend on the amount of users your Gateway will be serving at any one time. If you're just going to be using it yourself, then minimal RAM is fine. **If you're running your Gateway as part of a Service Grant, get something with at least 4GB RAM.**
- Disks: much like the amount of RAM your Gateway could use, the amount of disk space required will vary with the amount of users your Gateway is serving. **If you're running your Gateway as part of a Service Grant, get something with at least 40GB storage.**
