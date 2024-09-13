# Sandbox Testnet

Nym node operators can run their nodes in Nym Sandbox testnet environment. Whether it's testing new configuration, hot features from Nym developers or just trying to setup a node for the first time, this environment is for you.

Below are steps to [setup your environment](#sandbox-environment-setup) and an introduction to [Sandbox token faucet](#sandbox-token-faucet).

```admonish warning title=""
This page is for Nym node operators. If you want to run NymVPN CLI over Sandbox testnet, visit our [developers portal](https://nymtech.net/developers/nymvpn/cli.html#testnet-environment).
```

## Sandbox Environment Setup

> Any syntax in `<>` brackets is a user's unique variable. Exchange with a corresponding name without the `<>` brackets.

To run Nym binaries in Sandbox testnet, you need to get `sandbox.env` configuration file and point your binary to it. Follow the steps below:

1. Create Sandbox environment config file by saving [this](https://raw.githubusercontent.com/nymtech/nym/develop/envs/sandbox.env) as `sandbox.env` in the same directory as your binaries:
```sh
curl -o sandbox.env -L https://raw.githubusercontent.com/nymtech/nym/develop/envs/sandbox.env

# In case you want to save the file elswhere, change the path in '-o' flag
```

2. Run your `nym-node` with an additional flag `-c` or `--config-env-file` with a path to `sandbox.env` file followed by all needed commands and options. For example:
```sh
# this example is for nym-node in mixnode mode
./nym-node --config-env-file <PATH/TO/>sandbox.env run --mode mixnode

# this example is for nym-node in exit-gateway mode
./nym-node --config-file-env <PATH/TO/>sandbox.env run --mode exit-gateway --id <ID> --public-ips "$(curl -4 https://ifconfig.me)" --hostname "<YOUR_DOMAIN>" --http-bind-address 0.0.0.0:8080 --mixnet-bind-address 0.0.0.0:1789 true --location <COUNTRY_FULL_NAME>

# In case you downloaded sandbox.env to the same directory, <PATH> is not needed
```

3. Bond your node to Nym Sandbox environment:
	- Open [Nym Wallet](https://nymtech.net/download/wallet) and switch to testnet
	- Go to [faucet.nymtech.net](https://faucet.nymtech.net) and aquire 101 testnet NYM tokens
	- Follow the steps on the [bonding page](nodes/bonding.md)

![](images/sandbox.png)

~~~admonish tip
1. If you [built Nym from source](../binaries/building-nym.md), you already have `sandbox.env` as a part of the monorepo (`nym/envs/sandbox.env`). Giving that you are likely to run `nym-node` from `nym/target/release`, the flag will look like this `--config-env-file ../../envs/sandbox.env`

2. You can export the path to `sandbox.env` to your enviromental variables:
```sh
export NYMNODE_CONFIG_ENV_FILE_ARG=<PATH/TO/sandbox.env>
```
~~~

## Sandbox Token Faucet

To run your nodes in Sandbox environment, you need testnet version of NYM token, that can be aquired from [faucet.nymtech.net](https://faucet.nymtech.net).

To prevent abuse, the faucet is rate-limited - your request will fail if the requesting wallet already has 101 NYM tokens.
