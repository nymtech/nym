# Sandbox Testnet

Nym node operators can run their nodes on Nym Sandbox testnet environment. Whether it's testing new configuration, hot features from Nym developers or just trying to setup a node for the first time, this environment is for you. Below are steps to [setup your environment](#sandbox-environment-setup) and an introduction to [Sandbox token faucet](#sandbox-token-faucet).

```admonish warning title=""
This page is for Nym node operators. If you want to run NymVPN CLI over Sandbox testnet, visit our [developers portal](https://nymtech.net/developers/nymvpn/cli.html#testnet-environment).
```

## Sanbox Environment Setup

> Any syntax in `<>` brackets is a user's unique variable. Exchange with a corresponding name without the `<>` brackets.

To run Nym binaries in Sandbox Testnet you need to get the `.env` configuration file and point your binary to it. Follow the steps below:


1. Create Sandbox environment config file by saving [this](https://raw.githubusercontent.com/nymtech/nym/develop/envs/sandbox.env) as `sandbox.env` in the same directory as your binaries:
```sh
curl -o sandbox.env -L https://raw.githubusercontent.com/nymtech/nym/develop/envs/sandbox.env
```

2. Run your `nym-node` with all the commands as always with an additional flag `--config-file` with a path to `sanbox.env` file. For example:
```sh
# this example is for mixnode mode
./nym-node run --mode mixnode --config-file <PATH/TO/sandbox.env>

# this example is for exit-gateway mode
./nym-node run --id <ID> --config-file <PATH/TO/sandbox.env> --mode exit-gateway --public-ips "$(curl -4 https://ifconfig.me)" --hostname "<YOUR_DOMAIN>" --http-bind-address 0.0.0.0:8080 --mixnet-bind-address 0.0.0.0:1789 true --location <COUNTRY_FULL_NAME>
```

3. Bond your node to Nym Sandbox environment:
	- Open [Nym Wallet](https://nymtech.net/download/wallet) and switch to testnet
	- Go to [faucet.nymtech.net](https://faucet.nymtech.net) and aquire 101 testnet NYM tokens
	- Follow the steps on the [bonding page](nodes/bonding.md)

![](images/sandbox.png)

~~~admonish tip
1. If you [built Nym from source](building-nym.md), you already have `sanbox.env` as a part of the monorepo (`nym/envs/sandbox.env`). Giving that you likely to run `nym-node` from `nym/target/release`, the flag will look like this `--config-env ../../envs/sandbox.env`

2. You can export the path to `sanbox.env` to your enviromental variables:
```sh
export NYMNODE_CONFIG=<PATH/TO/sandbox.env>
```
~~~

## Sandbox Token Faucet

To run your nodes in the sandbox environment, you need testnet version of NYM token, that can be aquired at [faucet.nymtech.net](https://faucet.nymtech.net).

To prevent abuse, the faucet is rate-limited - your request will fail if the requesting wallet already has 101 NYM tokens.
