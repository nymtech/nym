# Gateways

> The Nym gateway was built in the [building nym](../binaries/building-nym.md) section. If you haven't yet built Nym and want to run the code, go there first.

## Current version
```
<!-- cmdrun ../../../../target/release/nym-gateway --version | grep "Build Version" | cut -b 21-26  -->
```


## Preliminary steps
There are a couple of steps that need completing before starting to set up your gateway:

- preparing your wallet
- requisitioning a VPS (Virtual Private Server)

### Wallet preparation
#### Mainnet
Before you initialise and run your gateway, head to our [website](https://nymtech.net/download/) and download the Nym wallet for your operating system. If pre-compiled binaries for your operating system aren't availiable, you can build the wallet yourself with instructions [here](../wallet/desktop-wallet.md).

If you don't already have one, please create a Nym address using the wallet, and fund it with tokens. The minimum amount required to bond a gateway is 100 `NYM`, but make sure you have a bit more to account for gas costs.

`NYM` can be purchased via Bity from the wallet itself with BTC or fiat, and is currently present on several exchanges.

> Remember that you can **only** use native Cosmos `NYM` tokens to bond your gateway. You **cannot** use ERC20 representations of `NYM` to run a node.


#### Sandbox testnet
Make sure to download a wallet and create an account as outlined above. Then head to our [token faucet](https://faucet.nymtech.net/) and get some tokens to use to bond it.

### VPS Hardware Specs
You will need to rent a VPS to run your mix node on. One key reason for this is that your node **must be able to send TCP data using both IPv4 and IPv6** (as other nodes you talk to may use either protocol.

We currently have these _rough_ specs for VPS hardware:

- Processors: 2 cores are fine. Get the fastest CPUs you can afford.
- RAM: Memory requirements depend on the amount of users your Gateway will be serving at any one time. If you're just going to be using it yourself, then minimal RAM is fine. **If you're running your Gateway as part of a Service Grant, get something with at least 4GB RAM.**
- Disks: much like the amount of RAM your Gateway could use, the amount of disk space required will vary with the amount of users your Gateway is serving. **If you're running your Gateway as part of a Service Grant, get something with at least 40GB storage.**

## Gateway setup
Now that you have built the codebase, set up your wallet, and have a VPS with the `nym-gateway` binary, you can set up your gateway with the instructions below.

### Viewing command help
You can check that your binaries are properly compiled with:

```
./nym-gateway --help
```

~~~admonish example collapsible=true title="Console output"
```
<!-- cmdrun ../../../../target/release/nym-gateway --help -->
```
~~~

You can also check the various arguments required for individual commands with:

```
./nym-gateway <command> --help
```

### Initialising your gateway
To check available configuration options use:

```
 ./nym-gateway init --help
```

~~~admonish example collapsible=true title="Console output"
```
<!-- cmdrun ../../../../target/release/nym-gateway init --help -->
```
~~~

The following command returns a gateway on your current IP with the `id` of `supergateway`:

```
./nym-gateway init --id supergateway --host $(curl ifconfig.me) --wallet-address n1eufxdlgt0puwrwptgjfqne8pj4nhy2u5ft62uq
```

~~~admonish example collapsible=true title="Console output"
```
<!-- cmdrun ../../../../target/release/nym-gateway init --id supergateway --host $(curl ifconfig.me) --wallet-address n1eufxdlgt0puwrwptgjfqne8pj4nhy2u5ft62uq -->
```
~~~

The `$(curl ifconfig.me)` command above returns your IP automatically using an external service. Alternatively, you can enter your IP manually wish. If you do this, remember to enter your IP **without** any port information.

### Bonding your gateway
#### Via the Desktop wallet
You can bond your gateway via the Desktop wallet.

* Open your wallet, and head to the `Bond` page, then select the node type and input your node details. Press `continue`

* You will be asked to run a the `sign` command with your `gateway` - copy and paste the long signature as the value of `--contract-msg` and run it. It will look something like this:

~~~admonish example collapsible=true title="Console output"
```
./nym-gateway sign --id upgrade_test --contract-msg 2Mf8xYytgEeyJke9LA7TjhHoGQWNBEfgHZtTyy2krFJfGHSiqy7FLgTnauSkQepCZTqKN5Yfi34JQCuog9k6FGA2EjsdpNGAWHZiuUGDipyJ6UksNKRxnFKhYW7ri4MRduyZwbR98y5fQMLAwHne1Tjm9cXYCn8McfigNt77WAYwBk5bRRKmC34BJMmWcAxphcLES2v9RdSR68tkHSpy2C8STfdmAQs3tZg8bJS5Qa8pQdqx14TnfQAPLk3QYCynfUJvgcQTrg29aqCasceGRpKdQ3Tbn81MLXAGAs7JLBbiMEAhCezAr2kEN8kET1q54zXtKz6znTPgeTZoSbP8rzf4k2JKHZYWrHYF9JriXepuZTnyxAKAxvGFPBk8Z6KAQi33NRQkwd7MPyttatHna6kG9x7knffV6ebGzgRBf7NV27LurH8x4L1uUXwm1v1UYCA1WSBQ9Pp2JW69k5v5v7G9gBy8RUcZnMbeL26Qqb8WkuGcmuHhaFfoqSfV7PRHPpPT4M8uRqUyR4bjUtSJJM1yh6QSeZk9BEazzoJqPeYeGoiFDZ3LMj2jesbJweQR4caaYuRczK92UGSSqu9zBKmE45a


      _ __  _   _ _ __ ___
     | '_ \| | | | '_ \ _ \
     | | | | |_| | | | | | |
     |_| |_|\__, |_| |_| |_|
            |___/

             (nym-gateway - version {{platform_release_version}})


>>> attempting to sign 2Mf8xYytgEeyJke9LA7TjhHoGQWNBEfgHZtTyy2krFJfGHSiqy7FLgTnauSkQepCZTqKN5Yfi34JQCuog9k6FGA2EjsdpNGAWHZiuUGDipyJ6UksNKRxnFKhYW7ri4MRduyZwbR98y5fQMLAwHne1Tjm9cXYCn8McfigNt77WAYwBk5bRRKmC34BJMmWcAxphcLES2v9RdSR68tkHSpy2C8STfdmAQs3tZg8bJS5Qa8pQdqx14TnfQAPLk3QYCynfUJvgcQTrg29aqCasceGRpKdQ3Tbn81MLXAGAs7JLBbiMEAhCezAr2kEN8kET1q54zXtKz6znTPgeTZoSbP8rzf4k2JKHZYWrHYF9JriXepuZTnyxAKAxvGFPBk8Z6KAQi33NRQkwd7MPyttatHna6kG9x7knffV6ebGzgRBf7NV27LurH8x4L1uUXwm1v1UYCA1WSBQ9Pp2JW69k5v5v7G9gBy8RUcZnMbeL26Qqb8WkuGcmuHhaFfoqSfV7PRHPpPT4M8uRqUyR4bjUtSJJM1yh6QSeZk9BEazzoJqPeYeGoiFDZ3LMj2jesbJweQR4caaYuRczK92UGSSqu9zBKmE45a
>>> decoding the message...
>>> message to sign: {"nonce":0,"algorithm":"ed25519","message_type":"gateway-bonding","content":{"sender":"n1ewmme88q22l8syvgshqma02jv0vqrug9zq9dy8","proxy":null,"funds":[{"denom":"unym","amount":"100000000"}],"data":{"gateway":{"host":"62.240.134.189","mix_port":1789,"clients_port":9000,"location":"62.240.134.189","sphinx_key":"FKbuN7mPdoCG9jA3CkAfXxC5X4rHhqeMVtmfRtJ3cFZd","identity_key":"3RoAhR8gEdfBETMjm2vbMFzKddxXDdE9ygBAnJHWqSzD","version":"1.1.13"}}}}
```
~~~

* Copy the resulting signature:

```
>>> The base58-encoded signature is:
2SPDjLjX4b6XEtkgG7yD8Znsb1xycL1edFvRK4JcVnPsM9k6HXEUUeVS6rswRiYxoj1bMgiRKyPDwiksiuyxu8Xi
```

* And paste it into the wallet nodal, then confirm the transaction.

![Paste Signature](../images/wallet-sign.png)

* Your gateway is now bonded.

> You are asked to `sign` a transaction on bonding so that the mixnet smart contract is able to map your nym address to your node. This allows us to create a nonce for each account and defend against replay attacks.

#### Via the CLI (power users)
If you want to bond your mix node via the CLI, then check out the [relevant section in the Nym CLI](../tools/nym-cli.md#bond-a-gateway) docs.

### Running your gateway
The `run` command starts the gateway:

```
./nym-gateway run --id supergateway
```

### Upgrading your gateway
Upgrading your node is a two-step process:
* Updating the binary and `config.toml` on your VPS
* Updating the node information in the [mixnet smart contract](../nyx/mixnet-contract.md). **This is the information that is present on the [mixnet explorer](https://explorer.nymtech.net)**.

>These instructions are specifically regarding upgrading your gateway binary from one version to another. If you want to change node information such as the listening port, you can do this by clicking the `node settings` tab in the `bond` page of the wallet.

#### Step 1: upgrading your binary
Follow these steps to upgrade your binary and update its config file:
* pause your gateway process.
* replace the existing binary with the newest binary (which you can either compile yourself or grab from our [releases page](https://github.com/nymtech/nym/releases)).
* re-run `init` with the same values as you used initially. **This will just update the config file, it will not overwrite existing keys**.
* restart your gateway process with the new binary.

#### Step 2: updating your node information in the smart contract
Follow these steps to update the information about your node which is publically avaliable from the [Nym API](https://validator.nymtech.net/api/swagger/index.html) and information displayed on the [mixnet explorer](https://explorer.nymtech.net).

You can either do this graphically via the Desktop Wallet, or the CLI.

#### Updating node information via the Desktop Wallet
* Navigate to the `Bonding` page and click the `Node Settings` link in the top right corner:
![Bonding page](../images/wallet-screenshots/bonding.png)

* Update the fields in the `Node Settings` page and click `Submit changes to the blockchain`.
![Node Settings Page](../images/wallet-screenshots/node_settings.png)

#### Updating node information via the CLI
If you want to bond your mix node via the CLI, then check out the [relevant section in the Nym CLI](../tools/nym-cli.md#upgrade-a-mix-node) docs.

## VPS Setup and Automation
### Configure your firewall
Although your gateway is now ready to receive traffic, your server may not be - the following commands will allow you to set up a properly configured firewall using `ufw`:

```
# check if you have ufw installed
ufw version
# if it is not installed, install with
sudo apt install ufw -y
# enable ufw
sudo ufw enable
# check the status of the firewall
sudo ufw status
```

Finally open your gateway's p2p port, as well as ports for ssh and incoming traffic connections:

```
sudo ufw allow 1789,22,9000/tcp
# check the status of the firewall
sudo ufw status
```

For more information about your gateway's port configuration, check the [gateway port reference table](#gateway-port-reference) below.

### Automating your gateway with systemd
Although it's not totally necessary, it's useful to have the gateway automatically start at system boot time. Here's a systemd service file to do that:

```ini
[Unit]
Description=Nym Gateway ({{platform_release_version}})
StartLimitInterval=350
StartLimitBurst=10

[Service]
User=nym
LimitNOFILE=65536
ExecStart=/home/nym/nym-gateway run --id supergateway
KillSignal=SIGINT
Restart=on-failure
RestartSec=30

[Install]
WantedBy=multi-user.target
```

Put the above file onto your system at `/etc/systemd/system/nym-gateway.service`.

Change the path in `ExecStart` to point at your gateway binary (`nym-gateway`), and the `User` so it is the user you are running as.

If you have built nym on your server, and your username is `jetpanther`, then the start command might look like this:

`ExecStart=/home/jetpanther/nym/target/release/nym-gateway run --id your-id`. Basically, you want the full `/path/to/nym-gateway run --id whatever-your-node-id-is`


Then run:

```
systemctl enable nym-gateway.service
```

Start your node:

```
service nym-gateway start
```

This will cause your node to start at system boot time. If you restart your machine, the node will come back up automatically.

You can also do `service nym-gateway stop` or `service nym-gateway restart`.

Note: if you make any changes to your systemd script after you've enabled it, you will need to run:

```
systemctl daemon-reload
```

This lets your operating system know it's ok to reload the service configuration.

## Gateway related Validator API endpoints
Numerous gateway related API endpoints are documented on the Validator API's [Swagger Documentation](https://validator.nymtech.net/api/swagger/index.html). There you can also try out various requests from your broswer, and download the response from the API. Swagger will also show you what commands it is running, so that you can run these from an app or from your CLI if you prefer.

## Ports
All gateway specific port configuration can be found in `$HOME/.nym/gateways/<your-id>/config/config.toml`. If you do edit any port configs, remember to restart your gateway.

### Gateway port reference
| Default port | Use                       |
|--------------|---------------------------|
| 1789         | Listen for Mixnet traffic |
| 9000         | Listen for Client traffic |
