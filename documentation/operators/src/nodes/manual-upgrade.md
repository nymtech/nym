# Manual Node Upgrade

> The process here is similar for the Mix Node, Gateway and Network Requester binaries. In the following steps we use a placeholder `<NODE>` in the commands, please change it for the binary name you want to upgrade (e.g.`nym-mixnode`). Any particularities for the given type of node are included.

Upgrading your node is a two-step process:

1. Updating the binary and `~/.nym/<NODE>/<YOUR_ID>/config/config.toml` on your VPS
2. Updating the node information in the [mixnet smart contract](https://nymtech.net/docs/nyx/mixnet-contract.html). **This is the information that is present on the [mixnet explorer](https://explorer.nymtech.net)**.

## Step 1: Upgrading your binary
Follow these steps to upgrade your Node binary and update its config file:
* Pause your node process.
    - if you see the terminal window with your node, press `ctrl + c`
    - if you run it as `systemd` service, run: `systemctl stop <NODE>.service`
* Replace the existing `<NODE>` binary with the newest binary (which you can either [compile yourself](https://nymtech.net/docs/binaries/building-nym.html) or grab from our [releases page](https://github.com/nymtech/nym/releases)).
* Re-run `init` with the same values as you used initially for your `<NODE>` ([Mix Node](./mix-node-setup.md#initialising-your-mix-node), [Gateway](./gateway-setup.md#initialising-your-gateway)) . **This will just update the config file, it will not overwrite existing keys**.
* Restart your node process with the new binary:
    - if your node is *not automated*, just `run` your `<NODE>` with `./<NODE> run --id <ID>`. Here are exact guidelines for [Mix Node](./mix-node-setup.md#running-your-mix-node) and [Gateway](./gateway-setup.md#running-your-gateway).
    - if you *automated* your node with systemd (recommended) run:
```sh
systemctl daemon-reload # to pickup the new unit file
systemctl start <NODE>.service
journalctl -f -u <NODE>.service # to monitor log of you node
```

If these steps are too difficult and you prefer to just run a script, you can use [ExploreNYM script](https://github.com/ExploreNYM/bash-tool) or one done by [Nym developers](https://gist.github.com/tommyv1987/4dca7cc175b70742c9ecb3d072eb8539).

> In case of a Network Requester this is all, the following step is only for Mix Nodes and Gateways.

## Step 2: Updating your node information in the smart contract
Follow these steps to update the information about your `<NODE>` which is publicly available from the [`nym-api`](https://validator.nymtech.net/api/swagger/index.html) and information displayed on the [Mixnet explorer](https://explorer.nymtech.net).

You can either do this graphically via the Desktop Wallet, or the CLI.

### Updating node information via the Desktop Wallet (recommended)
* Navigate to the `Bonding` page and click the `Node Settings` link in the top right corner:

![Bonding page](../images/wallet-screenshots/bonding.png)

* Update the fields in the `Node Settings` page (usually the field `Version` is the only one to change) and click `Submit changes to the blockchain`.

![Node Settings Page](../images/wallet-screenshots/node_settings.png)

### Updating node information via the CLI
If you want to bond your `<NODE>` via the CLI, then check out the [relevant section in the Nym CLI](https://nymtech.net/docs/tools/nym-cli.html#upgrade-a-mix-node) docs.


## Upgrading Network Requester to >= v1.1.10 from <v1.1.9

In the previous version of the network-requester, users were required to run a nym-client along side it to function. As of `v1.1.10`, the network-requester now has a nym client embedded into the binary, so it can run standalone.

If you are running an existing Network Requester registered with nym-connect, upgrading requires you move your old keys over to the new Network Requester configuration. We suggest following these instructions carefully to ensure a smooth transition.

Initiate the new Network Requester:

```sh
nym-network-requester init --id <YOUR_ID>
```

Copy the old keys from your client to the network-requester configuration that was created above:

```sh
cp -vr ~/.nym/clients/myoldclient/data/* ~/.nym/service-providers/network-requester/<YOUR_ID>/data
```

Edit the configuration to match what you used on your client. Specifically, edit the configuration file at:

```sh
~/.nym/service-providers/network-requester/<YOUR_ID>/config/config.toml
```

Ensure that the fields `gateway_id`, `gateway_owner`, `gateway_listener` in the new config match those in the old client config at:

```sh
~/.nym/clients/myoldclient/config/config.toml
```

## Upgrading your validator

Upgrading from `v0.31.1` -> `v0.32.0` process is fairly simple. Grab the `v0.32.0` release tarball from the [`nyxd` releases page](https://github.com/nymtech/nyxd/releases), and untar it. Inside are two files:

- the new validator (`nyxd`) v0.32.0
- the new wasmvm (it depends on your platform, but most common filename is `libwasmvm.x86_64.so`)

Wait for the upgrade height to be reached and the chain to halt awaiting upgrade, then:

* copy `libwasmvm.x86_64.so` to the default LD_LIBRARY_PATH on your system (on Ubuntu 20.04 this is `/lib/x86_64-linux-gnu/`) replacing your existing file with the same name.
* swap in your new `nyxd` binary and restart.

You can also use something like [Cosmovisor](https://github.com/cosmos/cosmos-sdk/tree/main/tools/cosmovisor) - grab the relevant information from the current upgrade proposal [here](https://nym.explorers.guru/proposal/9).

Note: Cosmovisor will swap the `nyxd` binary, but you'll need to already have the `libwasmvm.x86_64.so` in place.

### Common reasons for your validator being jailed

The most common reason for your validator being jailed is that your validator is out of memory because of bloated syslogs.

Running the command `df -H` will return the size of the various partitions of your VPS.

If the `/dev/sda` partition is almost full, try pruning some of the `.gz` syslog archives and restart your validator process.

