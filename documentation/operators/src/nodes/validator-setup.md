# Validators

> Nym has two main codebases:  
> - the [Nym platform](https://github.com/nymtech/nym), written in Rust. This contains all of our code except for the validators.
> - the [Nym validators](https://github.com/nymtech/nyxd), written in Go.

The validator is built using [Cosmos SDK](https://cosmos.network) and [Tendermint](https://tendermint.com), with a [CosmWasm](https://cosmwasm.com) smart contract controlling the directory service, node bonding, and delegated mixnet staking.

> We are currently working towards building up a closed set of reputable validators. You can ask us for coins to get in, but please don't be offended if we say no - validators are part of our system's core security and we are starting out with people we already know or who have a solid reputation.

## Building your validator

> Any syntax in `<>` brackets is a user's unique variable. Exchange it with a corresponding name without the `<>` brackets.

### Prerequisites
#### `git`, `gcc`, `jq`

* Debian-based systems:
```
apt install git build-essential jq

# optional additional manual pages can be installed with:
apt-get install manpages-dev
```

* Arch-based systems:
Install `git`, `gcc` and `jq` with the following:
```
pacman -S git gcc jq
```

#### `Go`
`Go` can be installed via the following commands (taken from the [Go Download and install page](https://go.dev/doc/install)):

```
# First remove any existing old Go installation and extract the archive you just downloaded into /usr/local: 
# You may need to run the command as root or through sudo
rm -rf /usr/local/go && tar -C /usr/local -xzf go1.20.6.linux-amd64.tar.gz

# Add /usr/local/go/bin to the PATH environment variable
export PATH=$PATH:/usr/local/go/bin
source $HOME/.profile
```

Verify `Go` is installed with:

```
go version
# Should return something like:
go version go1.20.4 linux/amd64
```

### Download a precompiled validator binary
You can find pre-compiled binaries for Ubuntu `22.04` and `20.04` [here](https://github.com/nymtech/nyxd/releases).

```admonish caution title=""
There are seperate releases for Mainnet and the Sandbox testnet - make sure to download the correct binary to avoid `bech32Prefix` mismatches.
```

### Manually compiling your validator binary
The codebase for the Nyx validators can be found [here](https://github.com/nymtech/nyxd).

The validator binary can be compiled by running the following commands:
```
git clone https://github.com/nymtech/nyxd.git
cd nyxd
git checkout release/<NYXD_VERSION>

# Mainnet
make build

# Sandbox testnet
BECH32_PREFIX=nymt make build
```

At this point, you will have a copy of the `nyxd` binary in your `build/` directory. Test that it's compiled properly by running:

```
./build/nyxd
```

You should see a similar help menu printed to you:

~~~admonish example collapsible=true title="Console output"
```
Wasm Daemon (server)

Usage:
  nyxd [command]

Available Commands:
  add-genesis-account      Add a genesis account to genesis.json
  add-wasm-genesis-message Wasm genesis subcommands
  collect-gentxs           Collect genesis txs and output a genesis.json file
  config                   Create or query an application CLI configuration file
  debug                    Tool for helping with debugging your application
  export                   Export state to JSON
  gentx                    Generate a genesis tx carrying a self delegation
  help                     Help about any command
  init                     Initialize private validator, p2p, genesis, and application configuration files
  keys                     Manage your application's keys
  query                    Querying subcommands
  rollback                 rollback cosmos-sdk and tendermint state by one height
  start                    Run the full node
  status                   Query remote node for status
  tendermint               Tendermint subcommands
  tx                       Transactions subcommands
  validate-genesis         validates the genesis file at the default location or at the location passed as an arg
  version                  Print the application binary version information

Flags:
  -h, --help                help for nyxd
      --home string         directory for config and data (default "/home/willow/.nyxd")
      --log_format string   The logging format (json|plain) (default "plain")
      --log_level string    The logging level (trace|debug|info|warn|error|fatal|panic) (default "info")
      --trace               print out full stack trace on errors

Use "nyxd [command] --help" for more information about a command.

```
~~~

### Linking `nyxd` to `libwasmvm.so`

`libwasmvm.so` is the wasm virtual machine which is needed to execute smart contracts in `v0.26.1`. This file is renamed in `libwasmvm.x86_64.so` in `v0.31.1`.

If you downloaded your `nyxd` binary from Github, you will have seen this file when un-`tar`-ing the `.tar.gz` file from the releases page.

If you are seeing an error concerning this file when trying to run `nyxd`, then you need to move the `libwasmvm.so` file to correct location.

Simply `cp` or `mv` that file to `/lib/x86_64-linux-gnu/` and re-run `nyxd`.

### Adding `nyxd` to your `$PATH`
You'll need to set `LD_LIBRARY_PATH` in your user's `~/.bashrc` or `~/.zshrc` file (depends on the terminal you use), and add that to our path. Replace `/home/<USER>/<PATH-TO-NYM>/binaries` in the command below to the locations of `nyxd` and `libwasmvm.so` and run it. If you have compiled these on the server, they will be in the `build/` folder:

```
NYX_BINARIES=/home/<USER>/<PATH-TO-VALIDATOR>/<BINARY>

# if you are using another shell like zsh replace '.bashrc' with the relevant config file
echo 'export LD_LIBRARY_PATH=${LD_LIBRARY_PATH}:'NYX_BINARIES >> ~/.bashrc
echo 'export PATH=$PATH:'${NYX_BINARIES} >> ~/.bashrc
source ~/.bashrc
```

Test everything worked:

```
nyxd
```

This should return the regular help menu:

~~~admonish example collapsible=true title="Console output"
```
Wasm Daemon (server)

Usage:
  nyxd [command]

Available Commands:
  add-genesis-account      Add a genesis account to genesis.json
  add-wasm-genesis-message Wasm genesis subcommands
  collect-gentxs           Collect genesis txs and output a genesis.json file
  config                   Create or query an application CLI configuration file
  debug                    Tool for helping with debugging your application
  export                   Export state to JSON
  gentx                    Generate a genesis tx carrying a self delegation
  help                     Help about any command
  init                     Initialize private validator, p2p, genesis, and application configuration files
  keys                     Manage your application's keys
  query                    Querying subcommands
  rollback                 rollback cosmos-sdk and tendermint state by one height
  start                    Run the full node
  status                   Query remote node for status
  tendermint               Tendermint subcommands
  tx                       Transactions subcommands
  validate-genesis         validates the genesis file at the default location or at the location passed as an arg
  version                  Print the application binary version information

Flags:
  -h, --help                help for nyxd
      --home string         directory for config and data (default "/home/willow/.nyxd")
      --log_format string   The logging format (json|plain) (default "plain")
      --log_level string    The logging level (trace|debug|info|warn|error|fatal|panic) (default "info")
      --trace               print out full stack trace on errors

Use "nyxd [command] --help" for more information about a command.

```
~~~

## Initialising your validator
### Prerequisites:

- FQDN Domain name
- IPv4 and IPv6 connectivity

Choose a name for your validator and use it in place of `<ID>` in the following command:

```
# Mainnet
nyxd init <ID> --chain-id=nyx

# Sandbox testnet
nyxd init <ID> --chain-id=sandbox
```

```admonish caution title=""
`init` generates `priv_validator_key.json` and `node_key.json`.

If you have already set up a validator on a network, **make sure to back up the key located at**
`~/.nyxd/config/priv_validator_key.json`.

If you don't save the validator key, then it can't sign blocks and will be jailed all the time, and
there is no way to deterministically (re)generate this key.
```

At this point, you have a new validator, with its own genesis file located at `$HOME/.nyxd/config/genesis.json`. You will need to replace the contents of that file that with either the Nyx Mainnet or Sandbox Testnet genesis file.

You can use the following command to download them for the correct network:

```
# Mainnet
wget  -O $HOME/.nyxd/config/genesis.json https://nymtech.net/genesis/genesis.json

# Sandbox testnet
wget -O $HOME/.nyxd/config/genesis.json https://sandbox-validator1.nymtech.net/snapshots/genesis.json
```

### `config.toml` configuration

Edit the following config options in `$HOME/.nyxd/config/config.toml` to match the information below for your network:

```
# Mainnet
persistent_peers = "ee03a6777fb76a2efd0106c3769daaa064a3fcb5@51.79.21.187:26656"
create_empty_blocks = false
laddr = "tcp://0.0.0.0:26656"
```

```
# Sandbox testnet
cors_allowed_origins = ["*"]
persistent_peers = "8421c0a3d90d490e27e8061f2abcb1276c8358b6@sandbox-validator1.nymtech.net:26666"
create_empty_blocks = false
laddr = "tcp://0.0.0.0:26656"
```

These affect the following:  
* `persistent_peers = "<PEER_ADDRESS>@<DOMAIN>.nymtech.net:26666"` allows your validator to start pulling blocks from other validators. **The main sandbox validator listens on `26666` instead of the default `26656` for debugging**. It is recommended you do not change your port from `26656`.
* `create_empty_blocks = false` will save space
* `laddr = "tcp://0.0.0.0:26656"` is in your p2p configuration options

Optionally, if you want to enable [Prometheus](https://prometheus.io/) metrics then the following must also match in the `config.toml`:

- `prometheus = true`
- `prometheus_listen_addr = ":26660"`

> Remember to enable metrics in the 'Configuring Prometheus metrics' section below as well.

And if you wish to add a human-readable moniker to your node:

- `moniker = "<YOUR_VALIDATOR_NAME>"`

Finally, if you plan on using [Cockpit](https://cockpit-project.org/documentation.html) on your server, change the `grpc` port from `9090` as this is the port used by Cockpit.

### `app.toml` configuration
In the file `$HOME/nyxd/config/app.toml`, set the following values:

```
# Mainnet
minimum-gas-prices = "0.025unym,0.025unyx"
enable = true in the `[api]` section to get the API server running
```
```
# Sandbox Testnet
minimum-gas-prices = "0.025unymt,0.025unyxt"
enable = true` in the `[api]` section to get the API server running
```

### Setting up your validator's admin user
You'll need an admin account to be in charge of your validator. Set that up with:

```
nyxd keys add nyxd-admin
```

This will add keys for your administrator account to your system's keychain and log your name, address, public key, and mnemonic. As the instructions say, remember to **write down your mnemonic**.

You can get the admin account's address with:

```
nyxd keys show nyxd-admin -a
```

Type in your keychain **password**, not the mnemonic, when asked.

## Starting your validator

```admonish caution title=""
If you are running a Sandbox testnet validator, please skip the `validate-genesis` command: it will fail due to the size of the genesis file as this is a fork of an existing chain state.
```

Everything should now be ready to go. You've got the validator set up, all changes made in `config.toml` and `app.toml`, the Nym genesis file copied into place (replacing the initial auto-generated one). Now let's validate the whole setup:

```
nyxd validate-genesis
```

If this check passes, you should receive the following output:

```
File at /path/to/genesis.json is a valid genesis file
```

> If this test did not pass, check that you have replaced the contents of `/<PATH-TO>/.nymd/config/genesis.json` with that of the correct genesis file.

### Open firewall ports

Before starting the validator, we will need to open the firewall ports:

```
# if ufw is not already installed:
sudo apt install ufw
sudo ufw enable
sudo ufw allow 1317,26656,26660,22,80,443/tcp
# to check everything worked
sudo ufw status
```

Ports `22`, `80`, and `443` are for ssh, http, and https connections respectively. The rest of the ports are documented [here](https://docs.cosmos.network/main/core/grpc_rest).

For more information about your validator's port configuration, check the [validator port reference table](./maintenance.md#ports) below.

> If you are planning to use [Cockpit](https://cockpit-project.org/) on your validator server then you will have defined a different `grpc` port in your `config.toml` above: remember to open this port as well.

Start the validator:

```
nyxd start
```

Once your validator starts, it will start requesting blocks from other validators. This may take several hours. Once it's up to date, you can issue a request to join the validator set with the command below.

### Syncing from a snapshot
If you wish to sync from a snapshot on **mainnet** use Polkachu's [mainnet](https://polkachu.com/networks/nym) resources.

If you wish to sync from a snapshot on **Sandbox testnet** use the below commands, which are a modified version of Polkachu's excellent resources. These commands assume you are running an OS with `apt` as the package manager:

```
# install lz4 if necessary
sudo apt install snapd -y
sudo snap install lz4

# download the snapshot
wget -O nyxd-sandbox-snapshot-data.tar.lz4 https://sandbox-validator1.nymtech.net/snapshots/nyxd-sandbox-snapshot-data.tar.lz4

# reset your validator state
nyxd tendermint unsafe-reset-all

# unpack the snapshot
lz4 -c -d nyxd-sandbox-snapshot-data.tar.lz4 | tar -x -C $HOME/.nyxd
```

You can then restart `nyxd` - it should start syncing from a block > 2000000.

### Joining Consensus
```admonish caution title=""
When joining consensus, make sure that you do not disrupt (or worse - halt) the network by coming in with a disproportionately large amount of staked tokens.

Please initially stake a small amount of tokens compared to existing validators, then delegate to yourself in tranches over time.
```

Once your validator has synced and you have received tokens, you can join consensus and produce blocks.

```
# Mainnet
nyxd tx staking create-validator
  --amount=10000000unyx
  --fees=0unyx
  --pubkey=$(/home/<USER>/<PATH-TO>/nyxd/binaries/nyxd tendermint show-validator)
  --moniker="<YOUR_VALIDATOR_NAME>"
  --chain-id=nyx
  --commission-rate="0.10"
  --commission-max-rate="0.20"
  --commission-max-change-rate="0.01"
  --min-self-delegation="1"
  --gas="auto"
  --gas-adjustment=1.15
  --from="KEYRING_NAME"
  --node https://rpc-1.nyx.nodes.guru:443
```
```
# Sandbox Testnet
nyxd tx staking create-validator
  --amount=10000000unyxt
  --fees=5000unyxt
  --pubkey=$(/home/<USER>/<PATH-TO>/nym/binaries/nyxd tendermint show-validator)
  --moniker="<YOUR_VALIDATOR_NAME>"
  --chain-id=sandbox
  --commission-rate="0.10"
  --commission-max-rate="0.20"
  --commission-max-change-rate="0.01"
  --min-self-delegation="1"
  --gas="auto"
  --gas-adjustment=1.15
  --from="KEYRING_NAME"
  --node https://sandbox-validator1.nymtech.net:443
```

You'll need either `unyxt` tokens on Sandbox, or `unyx` tokens on mainnet to perform this command.

> We are currently working towards building up a closed set of reputable validators. You can ask us for coins to get in, but please don't be offended if we say no - validators are part of our system's core security and we are starting out with people we already know or who have a solid reputation.

If you want to edit some details for your node you will use a command like this:

```
# Mainnet
nyxd tx staking edit-validator
  --chain-id=nyx
  --moniker="<YOUR_VALIDATOR_NAME>"
  --details="Nyx validator"
  --security-contact="<YOUR_EMAIL>"
  --identity="<YOUR_IDENTITY>"
  --gas="auto"
  --gas-adjustment=1.15
  --from="KEYRING_NAME"
  --fees 2000unyx
```
```
# Sandbox testnet
nyxd tx staking edit-validator
  --chain-id=sandbox
  --moniker="<YOUR_VALIDATOR_NAME>"
  --details="Sandbox testnet validator"
  --security-contact="your email"
  --identity="<YOUR_IDENTITY>"
  --gas="auto"
  --gas-adjustment=1.15
  --from="KEYRING_NAME"
  --fees 2000unyxt
```

With above command you can specify the `gpg` key last numbers (as used in `keybase`) as well as validator details and your email for security contact.

### Automating your validator with systemd
You will most likely want to automate your validator restarting if your server reboots. Checkout the [maintenance page](./maintenance.md#systemd) with a quick tutorial.

### Installing and configuring nginx for HTTPS

If you want to set up a reverse proxying on the validator server to improve security and performance, using [nginx](https://www.nginx.com/resources/glossary/nginx/#:~:text=NGINX%20is%20open%20source%20software,%2C%20media%20streaming%2C%20and%20more.&text=In%20addition%20to%20its%20HTTP,%2C%20TCP%2C%20and%20UDP%20servers.), follow the manual on the [maintenance page](./maintenance.md#setup).

### Setting the ulimit

Linux machines limit how many open files a user is allowed to have. This is called a `ulimit`. We need to set it to a higher value than the default 1024. Follow the instructions in the [maintenance page](./maintenance.md#Setting-the-ulimit) to change the `ulimit` value for validators.

## Using your validator
### Unjailing your validator
If your validator gets jailed, you can fix it with the following command:

```
# Mainnet
nyxd tx slashing unjail
  --broadcast-mode=block
  --from="KEYRING_NAME"
  --chain-id=nyx
  --gas=auto
  --gas-adjustment=1.4
  --fees=7000unyx
```
```
# Sandbox Testnet
nyxd tx slashing unjail
  --broadcast-mode=block
  --from="KEYRING_NAME"
  --chain-id=sandbox
  --gas=auto
  --gas-adjustment=1.4
  --fees=7000unyxt
```

### Upgrading your validator

To upgrade your validator, follow the steps on the [maintenance page](./maintenance.md#setting-the-ulimit).

#### Common reasons for your validator being jailed

The most common reason for your validator being jailed is that your validator is out of memory because of bloated syslogs.

Running the command `df -H` will return the size of the various partitions of your VPS.

If the `/dev/sda` partition is almost full, try pruning some of the `.gz` syslog archives and restart your validator process.

### Day 2 operations with your validator

You can check your current balances with:

```
nymd query bank balances ${ADDRESS}
```

For example, on the Sanbox testnet this would return:

```yaml
balances:
- amount: "919376"
denom: unymt
pagination:
next_key: null
total: "0"
```

You can, of course, stake back the available balance to your validator with the following command.

> Remember to save some tokens for gas costs!

```
# Mainnet
nyxd tx staking delegate VALOPERADDRESS AMOUNTunym
  --from="KEYRING_NAME"
  --keyring-backend=os
  --chain-id=nyx
  --gas="auto"
  --gas-adjustment=1.15
  --fees 5000unyx
```
```
# Sandbox Testnet
nyxd tx staking delegate VALOPERADDRESS AMOUNTunymt
  --from="KEYRING_NAME"
  --keyring-backend=os
  --chain-id=sandbox
  --gas="auto"
  --gas-adjustment=1.15
  --fees 5000unyxt
```

