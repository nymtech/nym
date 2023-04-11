# Mix Nodes

> The Nym mix node binary was built in the [building nym](../binaries/building-nym.md) section. If you haven't yet built Nym and want to run the code, go there first.

```admonish info
The `nym-mixnode` binary is currently one point version ahead of the rest of the platform binaries due to a patch applied between releases. 
```

## Preliminary steps

There are a couple of steps that need completing before starting to set up your mix node:

- preparing your wallet
- requisitioning a VPS (Virtual Private Server)

### Wallet preparation

#### Mainnet

Before you initialise and run your mixnode, head to our [website](https://nymtech.net/download/) and download the Nym wallet for your operating system. If pre-compiled binaries for your operating system aren't availiable, you can build the wallet yourself with instructions [here](../wallet/desktop-wallet.md).

If you don't already have one, please create a Nym address using the wallet, and fund it with tokens. The minimum amount required to bond a mixnode is 100 `NYM`, but make sure you have a bit more to account for gas costs.

`NYM` can be purchased via Bity from the wallet itself, and is currently present on several exchanges. Head to our [telegram channels](https://t.me/nymchan) to find out where to get `NYM` tokens.

> Remember that you can **only** use Cosmos `NYM` tokens to bond your mixnode. You **cannot** use ERC20 representations of `NYM` to run a node.

#### Sandbox testnet

Make sure to download a wallet and create an account as outlined above. Then head to our [token faucet](https://faucet.nymtech.net/) and get some tokens to use to bond it.

### VPS Hardware Specs

You will need to rent a VPS to run your mix node on. One key reason for this is that your node **must be able to send TCP data using both IPv4 and IPv6** (as other nodes you talk to may use either protocol).

For the moment, we haven't put a great amount of effort into optimizing concurrency to increase throughput, so don't bother provisioning a beastly server with multiple cores. This will change when we get a chance to start doing performance optimizations in a more serious way. Sphinx packet decryption is CPU-bound, so once we optimise, more fast cores will be better.

For now, see the below rough specs:

- Processors: 2 cores are fine. Get the fastest CPUs you can afford.
- RAM: Memory requirements are very low - typically a mix node may use only a few hundred MB of RAM.
- Disks: The mixnodes require no disk space beyond a few bytes for the configuration files.

## Mix node setup

Now that you have built the codebase, set up your wallet, and have a VPS with the `nym-mixnode` binary, you can set up your mix node with the instructions below.

### Viewing command help

You can check that your binaries are properly compiled with:

```
./nym-mixnode --help
```

Which should return a list of all avaliable commands.

~~~admonish example collapsible=true title="Console output"
```
  _ __  _   _ _ __ ___
 | '_ \| | | | '_ \ _ \
 | | | | |_| | | | | | |
 |_| |_|\__, |_| |_| |_|
        |___/

         (mixnode - version {{mix_node_release_version}})


nym-mixnode {{mix_node_release_version}}
Nymtech
Implementation of a Loopix-based Mixnode

USAGE:
    nym-mixnode [OPTIONS] <SUBCOMMAND>

OPTIONS:
        --config-env-file <CONFIG_ENV_FILE>
            Path pointing to an env file that configures the mixnode

    -h, --help
            Print help information

    -V, --version
            Print version information

SUBCOMMANDS:
    completions          Generate shell completions
    describe             Describe your mixnode and tell people why they should delegate state to you
    generate-fig-spec    Generate Fig specification
    help                 Print this message or the help of the given subcommand(s)
    init                 Initialise the mixnode
    node-details         Show details of this mixnode
    run                  Starts the mixnode
    sign                 Sign text to prove ownership of this mixnode
    upgrade              Try to upgrade the mixnode

    nym-mixnode {{mix_node_release_version}}
    Nymtech
```
~~~

You can also check the various arguments required for individual commands with:

```
./nym-mixnode <command> --help
```

### Initialising your mix node

To check available configuration options for initializing your node use:

```
./nym-mixnode init --help
```

~~~admonish example collapsible=true title="Console output"
```
  _ __  _   _ _ __ ___
 | '_ \| | | | '_ \ _ \
 | | | | |_| | | | | | |
 |_| |_|\__, |_| |_| |_|
        |___/

         (mixnode - version {{mix_node_release_version}})


nym-mixnode-init
Initialise the mixnode

USAGE:
    nym-mixnode init [OPTIONS] --id <ID> --host <HOST> --wallet-address <WALLET_ADDRESS>

OPTIONS:
        --announce-host <ANNOUNCE_HOST>
            The custom host that will be reported to the directory server

    -h, --help
            Print help information

        --host <HOST>
            The host on which the mixnode will be running

        --http-api-port <HTTP_API_PORT>
            The port on which the mixnode will be listening for http requests

        --id <ID>
            Id of the mixnode we want to create config for

        --mix-port <MIX_PORT>
            The port on which the mixnode will be listening for mix packets

        --validators <VALIDATORS>
            Comma separated list of rest endpoints of the validators

        --verloc-port <VERLOC_PORT>
            The port on which the mixnode will be listening for verloc packets

        --wallet-address <WALLET_ADDRESS>
            The wallet address you will use to bond this mixnode, e.g.
            nymt1z9egw0knv47nmur0p8vk4rcx59h9gg4zuxrrr9
```
~~~

Initalise your mixnode with the following command, replacing the value of `--id` with the moniker you wish to give your mixnode, and the `--wallet-address` with the Nym address you created earlier. Your `--host` must be publicly routable on the internet in order to mix packets, and can be either an Ipv4 or IPv6 address. The `$(curl ifconfig.me)` command returns your IP automatically using an external service. If you enter your IP address manually, enter it **without** any port information.

```
./nym-mixnode init --id winston-smithnode --host $(curl ifconfig.me) --wallet-address <wallet-address>
```

> The `init` command will refuse to destroy existing mix node keys.

During the `init` process you will have the option to change the `http_api`, `verloc` and `mixnode` ports from their default settings. If you wish to change these in the future you can edit their values in the `config.toml` file created by the initialization process, which is located at `~/.nym/mixnodes/<your-id>/`.

### Bonding your mix node

```admonish caution
From `v1.1.3`, if you unbond your mixnode that means you are leaving the mixnet and you will lose all your delegations (permanently). You can join again with the same identity key, however, you will start with **no delegations**.
```

#### Bond via the Desktop wallet (recommended)

You can bond your mix node via the Desktop wallet.

* Open your wallet, and head to the `Bond` page, then select the node type and input your node details. Press `continue` 

* You will be asked to run a the `sign` command with your `gateway` - copy and paste the long signature as the value of `--contract-msg` and run it. It will look something like this: 

~~~admonish example collapsible=true title="Console output"
```
./nym-mixnode sign --id upgrade_test --contract-msg 5XrvVEMzRJk2AcT2h1o6ErZNb8z1ZzD3h7teipBW3NUtrtYq7vu4DRMgzZRTPVPnyr2YWCxpmKCMFaEXvksnJ
4jt7np3NMLxsLMrFjEBhh67Crtjy4868vCzAivUqzdc365RiqxQQKtv4r9eTk9mTbE9JY8U3TxzKJCSGcBqbrb9JX3HrZVWm6tqbUYbsnku9pqnfeyeUiaYKY44Lm72TYrkZfRrMAZLMATiXT1ntmiKqT37HzRxNZjiH8qHeQEoRHkgDsmXDXRbfppGTpPrN7R4sjynJzehzUBZ8Ug7ovT9FoAHb8kuVQhUiMs1js6tdwtthzQMbPi9vwxUtVvjYknN2fnJgMnckEhzJJpJDCNdH7YhpPaWQnGVVS334mskiuqkbRVrFPJN2nnwArHr3L2cLxSMk9toKfw7ViKJ2p5E5JxiSmKY1cFGZ7uRLsuQ833PJN9JE8crPtkBNefqkbFNz68S5jPmzUShSvAc4TqXKeovDASFmmhKaPqLUrfsSWm7nzuKnzJSMADF6xSuwr9cknMoirqkRkLe7ybJ2ERwSdf5cUxMjF7yjS8tW9hZudnTUb1uPNDuSmPPVrCR12XZyFzBvVgxH51ZNJTym46nqnfA881LQcmFMnCwJf39rVJ4ASLnzEzmuwXj75QoB9ce9kiLmoBNLYe4QKSB6gDd858VnBtBNQELVuCCZbrTYuSCeNdUFhvMwD4kryc1pBYUa8Ro81F3QVfiKN

      _ __  _   _ _ __ ___
     | '_ \| | | | '_ \ _ \
     | | | | |_| | | | | | |
     |_| |_|\__, |_| |_| |_|
            |___/
        
             (nym-mixnode - version 1.1.14)

    
>>> attempting to sign 5XrvVEMzRJk2AcT2h1o6ErZNb8z1ZzD3h7teipBW3NUtrtYq7vu4DRMgzZRTPVPnyr2YWCxpmKCMFaEXvksnJ4jt7np3NMLxsLMrFjEBhh67Crtjy4868vCzAivUqzdc365RiqxQQKtv4r9eTk9mTbE9JY8U3TxzKJCSGcBqbrb9JX3HrZVWm6tqbUYbsnku9pqnfeyeUiaYKY44Lm72TYrkZfRrMAZLMATiXT1ntmiKqT37HzRxNZjiH8qHeQEoRHkgDsmXDXRbfppGTpPrN7R4sjynJzehzUBZ8Ug7ovT9FoAHb8kuVQhUiMs1js6tdwtthzQMbPi9vwxUtVvjYknN2fnJgMnckEhzJJpJDCNdH7YhpPaWQnGVVS334mskiuqkbRVrFPJN2nnwArHr3L2cLxSMk9toKfw7ViKJ2p5E5JxiSmKY1cFGZ7uRLsuQ833PJN9JE8crPtkBNefqkbFNz68S5jPmzUShSvAc4TqXKeovDASFmmhKaPqLUrfsSWm7nzuKnzJSMADF6xSuwr9cknMoirqkRkLe7ybJ2ERwSdf5cUxMjF7yjS8tW9hZudnTUb1uPNDuSmPPVrCR12XZyFzBvVgxH51ZNJTym46nqnfA881LQcmFMnCwJf39rVJ4ASLnzEzmuwXj75QoB9ce9kiLmoBNLYe4QKSB6gDd858VnBtBNQELVuCCZbrTYuSCeNdUFhvMwD4kryc1pBYUa8Ro81F3QVfiKN
>>> decoding the message...
>>> message to sign: {"nonce":0,"algorithm":"ed25519","message_type":"mixnode-bonding","content":{"sender":"n1eufxdlgt0puwrwptgjfqne8pj4nhy2u5ft62uq","proxy":null,"funds":[{"denom":"unym","amount":"100000000"}],"data":{"mix_node":{"host":"62.240.134.189","mix_port":1789,"verloc_port":1790,"http_api_port":8000,"sphinx_key":"CfZSy1jRfrfiVi9JYexjFWPqWkKoY72t7NdpWaq37K8Z","identity_key":"DhmUYedPZvhP9MMwXdNpPaqCxxTQgjAg78s2nqtTTiNF","version":"1.1.14"},"cost_params":{"profit_margin_percent":"0.1","interval_operating_cost":{"denom":"unym","amount":"40000000"}}}}}
```
~~~

* Copy the resulting signature:

```
>>> The base58-encoded signature is:
2GbKcZVKFdpi3sR9xoJWzwPuGdj3bvd7yDtDYVoKfbTWdpjqAeU8KS5bSftD5giVLJC3gZiCg2kmEjNG5jkdjKUt
```

* And paste it into the wallet nodal, then confirm the transaction.

![Paste Signature](../images/wallet-sign.png)

* Your node will now be bonded and ready to mix at the beginning of the next epoch (at most 1 hour). 

> You are asked to `sign` a transaction on bonding so that the mixnet smart contract is able to map your nym address to your node. This allows us to create a nonce for each account and defend against replay attacks.

#### Bond via the CLI (power users)
If you want to bond your mix node via the CLI, then check out the [relevant section in the Nym CLI](../tools/nym-cli.md#bond-a-mix-node) docs.

### Running your mix node

Now you've bonded your mix node, run it with:

```
./nym-mixnode run --id winston-smithnode
```

~~~admonish example collapsible=true title="Console output"
```

Starting mixnode winston-smithnode...

To bond your mixnode you will need to install the Nym wallet, go to https://nymtech.net/get-involved and select the Download button.
Select the correct version and install it to your machine. You will need to provide the following:

Identity Key: GWrymUuLaxVHSs8iE7YW48MB81npnKjrVuJzJsGkeji6
Sphinx Key: FU89ULkS4YYDXcm5jShhJvoit7H4jG4EXHxRKbS9cXSJ
Host: 62.240.134.46 (bind address: 62.240.134.46)
Version: {{mix_node_release_version}}
Mix Port: 1789, Verloc port: 1790, Http Port: 8000

You are bonding to wallet address: n1x42mm3gsdg808qu2n3ah4l4r9y7vfdvwkw8az6

2022-04-27T16:08:01.159Z INFO  nym_mixnode::node > Starting nym mixnode
2022-04-27T16:08:01.490Z INFO  nym_mixnode::node > Starting node stats controller...
2022-04-27T16:08:01.490Z INFO  nym_mixnode::node > Starting packet delay-forwarder...
2022-04-27T16:08:01.490Z INFO  nym_mixnode::node > Starting socket listener...
2022-04-27T16:08:01.490Z INFO  nym_mixnode::node::listener > Running mix listener on "62.240.134.46:1789"
2022-04-27T16:08:01.490Z INFO  nym_mixnode::node           > Starting the round-trip-time measurer...

```
~~~

If everything worked, you'll see your node running on the either the [Sandbox testnet network explorer](https://sandbox-explorer.nymtech.net) or the [mainnet network explorer](https://explorer.nymtech.net), depending on which network you're running.

Note that your node's public identity key is displayed during startup, you can use it to identify your node in the list.

Keep reading to find out more about configuration options or troubleshooting if you're having issues. There are also some tips for running on AWS and other cloud providers, some of which require minor additional setup.

Also have a look at the saved configuration files in `$HOME/.nym/mixnodes/` to see more configuration options.

### Describe your mix node (optional)

In order to easily identify your node via human-readable information later on in the development of the testnet when delegated staking is implemented, you can `describe` your mixnode with the following command:

```
./nym-mixnode describe --id winston-smithnode
```

~~~admonish example collapsible=true title="Console output"
```
name: winston-smithnode
description: nym-mixnode hosted on Linode VPS in <location> with the following specs: <specs>.
link, e.g. https://mixnode.yourdomain.com: mixnode.mydomain.net
location, e.g. City: London, Country: UK: <your_location>

This information will be shown in the mixnode's page in the Network Explorer, and help people make delegated staking decisions.
```
~~~

> Remember to restart your mix node process in order for the new description to be propogated

### Upgrading your mix node

Upgrading your node is a two-step process:
* Updating the binary and `config.toml` on your VPS
* Updating the node information in the [mixnet smart contract](../nyx/mixnet-contract.md). **This is the information that is present on the [mixnet explorer](https://explorer.nymtech.net)**.

#### Step 1: upgrading your binary
Follow these steps to upgrade your mix node binary and update its config file:
* pause your mix node process.
* replace the existing binary with the newest binary (which you can either compile yourself or grab from our [releases page](https://github.com/nymtech/nym/releases)).
* re-run `init` with the same values as you used initially. **This will just update the config file, it will not overwrite existing keys**.
* restart your mix node process with the new binary.

> Do **not** use the `upgrade` command: there is a known error with the command that will be fixed in a subsequent release.

#### Step 2: updating your node information in the smart contract
Follow these steps to update the information about your mix node which is publically avaliable from the [Nym API](https://validator.nymtech.net/api/swagger/index.html) and information displayed on the [mixnet explorer](https://explorer.nymtech.net).

You can either do this graphically via the Desktop Wallet, or the CLI.

#### Updating node information via the Desktop Wallet
* Navigate to the `Bonding` page and click the `Node Settings` link in the top right corner:
![Bonding page](../images/wallet-screenshots/bonding.png)

* Update the fields in the `Node Settings` page and click `Submit changes to the blockchain`.
![Node Settings Page](../images/wallet-screenshots/node_settings.png)

#### Updating node information via the CLI
If you want to bond your mix node via the CLI, then check out the [relevant section in the Nym CLI](../tools/nym-cli.md#upgrade-a-mix-node) docs.

### Displaying mix node information

You can always check the details of your mix node with the `node-details` command:

```
./nym-mixnode node-details --id winston-smithnode
```

~~~admonish example collapsible=true title="Console output"
```

Identity Key: GWrymUuLaxVHSs8iE7YW48MB81npnKjrVuJzJsGkeji6
Sphinx Key: FU89ULkS4YYDXcm5jShhJvoit7H4jG4EXHxRKbS9cXSJ
Host: 62.240.134.46 (bind address: 62.240.134.46)
Version: {{mix_node_release_version}}
Mix Port: 1789, Verloc port: 1790, Http Port: 8000

You are bonding to wallet address: n1x42mm3gsdg808qu2n3ah4l4r9y7vfdvwkw8az6

```
~~~

## VPS Setup and Automation
### Configure your firewall
The following commands will allow you to set up a firewall using `ufw`.

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

Finally open your mix node's p2p port, as well as ports for ssh, http, and https connections, and ports `8000` and `1790` for verloc and measurement pings:

```
sudo ufw allow 1789,1790,8000,22,80,443/tcp
# check the status of the firewall
sudo ufw status
```

For more information about your mix node's port configuration, check the [mix node port reference table](./mix-node-setup.md#mixnode-port-reference) below.

### Automating your mix node with systemd

It's useful to have the mix node automatically start at system boot time. Here's a systemd service file to do that:

```ini
[Unit]
Description=Nym Mixnode ({{mix_node_release_version}})
StartLimitInterval=350
StartLimitBurst=10

[Service]
User=nym
LimitNOFILE=65536
ExecStart=/home/nym/nym-mixnode run --id mix0100
KillSignal=SIGINT
Restart=on-failure
RestartSec=30

[Install]
WantedBy=multi-user.target
```

Put the above file onto your system at `/etc/systemd/system/nym-mixnode.service`.

Change the path in `ExecStart` to point at your mix node binary (`nym-mixnode`), and the `User` so it is the user you are running as.

If you have built nym on your server, and your username is `jetpanther`, then the start command might look like this:

`ExecStart=/home/jetpanther/nym/target/release/nym-mixnode run --id your-id`. Basically, you want the full `/path/to/nym-mixnode run --id whatever-your-node-id-is`

Then run:

```
systemctl enable nym-mixnode.service
```

Start your node:

```
service nym-mixnode start
```

This will cause your node to start at system boot time. If you restart your machine, the node will come back up automatically.

You can also do `service nym-mixnode stop` or `service nym-mixnode restart`.

Note: if you make any changes to your systemd script after you've enabled it, you will need to run:

```
systemctl daemon-reload
```

This lets your operating system know it's ok to reload the service configuration.

#### Setting the ulimit

Linux machines limit how many open files a user is allowed to have. This is called a `ulimit`.

`ulimit` is 1024 by default on most systems. It needs to be set higher, because mix nodes make and receive a lot of connections to other nodes.

If you see errors such as:

```
Failed to accept incoming connection - Os { code: 24, kind: Other, message: "Too many open files" }
```

This means that the operating system is preventing network connections from being made.

##### Set the ulimit via `systemd` service file

Query the `ulimit` of your mix node with:

```
grep -i "open files" /proc/$(ps -A -o pid,cmd|grep nym-mixnode | grep -v grep |head -n 1 | awk '{print $1}')/limits
```

You'll get back the hard and soft limits, which looks something like this:

```
Max open files            65536                65536                files
```

If your output is **the same as above**, your node will not encounter any `ulimit` related issues.

However if either value is `1024`, you must raise the limit via the systemd service file. Add the line:

```
LimitNOFILE=65536
```

Reload the daemon:

```
systemctl daemon-reload
```

or execute this as root for system-wide setting of `ulimit`:

```
echo "DefaultLimitNOFILE=65535" >> /etc/systemd/system.conf
```

Reboot your machine and restart your node. When it comes back, use `cat /proc/$(pidof nym-mixnode)/limits | grep "Max open files"` to make sure the limit has changed to 65535.

##### Set the ulimit on `non-systemd` based distributions

Edit `etc/security/conf` and add the following lines:

```
# Example hard limit for max opened files
username        hard nofile 4096
# Example soft limit for max opened files
username        soft nofile 4096
```

Then reboot your server and restart your mixnode.

## Node Families

Node family involves setting up a group of mix nodes that work together to provide greater privacy and security for network communications. This is achieved by having the nodes in the family share information and routes, creating a decentralized network that makes it difficult for third parties to monitor or track communication traffic.

### Create a Node Family

To create a Node family, you will need to install and configure multiple mix nodes, and then use the CLI to link them together into a family. Once your Node family is up and running, you can use it to route your network traffic through a series of nodes, obscuring the original source and destination of the communication.

You can use either `nym-cli` which can be downloaded from the [release page](https://github.com/nymtech/nym/releases) or compiling `nyxd`.


`/path/to/the/release` and run the following on the family head to obtain the signature for the member:

```
./nym-mixnode sign --id winston-smithnode --text <text>
```

~~~admonish example collapsible=true title="Console output"
```



      _ __  _   _ _ __ ___
     | '_ \| | | | '_ \ _ \
     | | | | |_| | | | | | |
     |_| |_|\__, |_| |_| |_|
            |___/

             (nym-mixnode - version {{mix_node_release_version}})

Signing the text "APxUbCmGp4K9qDzvwVADJFNu8S3JV1AJBw7q6bS5KN9E" using your mixnode's Ed25519 identity key...
The base58-encoded signature on 'APxUbCmGp4K9qDzvwVADJFNu8S3JV1AJBw7q6bS5KN9E' is: 2ZuCFYU91pvEcgAj6EzU33oozazvsRAoxP7NQHFM6Xy6AkJrzgCZdnsnZYAmxFtqe8Su17KXwpTHQtkVmAnAiV4H

```
~~~

Using `nym-cli`:

> `--mnemonic` is the mnemonic of the member wanting to be the head of family.

```
/nym-cli cosmwasm execute <wallet-address> '{"create_family": {"signature": "<base58-encoded-signature>","family_head": "<text>","owner_signature":"<node owner signature>","label": "<node label>"}}' --mnemonic <mnemonic from node to be the head>
```

Using `nyxd`:

> `--from` is mnemonic of the member wanting to join the family.

```
./nyxd tx wasm execute ${MIXNET-CONTRACT} '{"join_family": {"signature": "<base58-encoded-signature>","family_head": "<text>"}}' --node ${VALIDATOR-ENDPOINT} --from mix1 --chain-id nyx --gas-prices 0.025unym --gas auto --gas-adjustment 1.3 -y -b block
```

To get the node owner signature, use:

`./nym-mixnode node-details --id <id>`

### Joining a Node Family

`/path/to/the/release` and run the following on the family head to obtain the signature for the member:

```
./nym-mixnode sign --id mixnode --text <text>
```

~~~admonish example collapsible=true title="Console output"
      _ __  _   _ _ __ ___
     | '_ \| | | | '_ \ _ \
     | | | | |_| | | | | | |
     |_| |_|\__, |_| |_| |_|
            |___/

             (nym-mixnode - version {{mix_node_release_version}})


Signing the text "4yRfauFzZnejJhG2FACTVQ7UnYEcFUYw3HzXrmuwLMaR" using your mixnode's Ed25519 identity key...
The base58-encoded signature on '4yRfauFzZnejJhG2FACTVQ7UnYEcFUYw3HzXrmuwLMaR' is: 4By7EQEMM8BAt6ptxJyeGqpoxHWxeRUhyJ4wMr2x3mXSQD9nvttkvd7tgP1uKu2ktJjB2bLzD1oaZ33d2Wv5eYWp
~~~

Using `nym-cli`:

```
./nym-cli cosmwasm execute <wallet-address> '{"join_family": {"signature": "<base58-encoded-signature>","family_head": "<text>","owner_signautre": "<owner signature from node to join>", "label":"<node to join label>"}}'  --mnemonic <mnemonic-from-node-to-join>
```

Using `nyxd`:

```
./nyxd tx wasm execute ${MIXNET-CONTRACT} '{"join_family": {"signature": "<base58-encoded-signature>","family_head": "<text>"}}' --node ${VALIDATOR-ENDPOINT} --from mix1 --chain-id nyx --gas-prices 0.025unym --gas auto --gas-adjustment 1.3 -y -b block
```


To get the node owner signature, use:

`./nym-mixnode node-details --id <id>`


### Leaving a family
If wanting to leave, run the same initial command as above, followed by:

Using `nym-cli`:

```
./nym-cli cosmwasm execute n17srjznxl9dvzdkpwpw24gg668wc73val88a6m5ajg6ankwvz9wtst0cznr '{"leave_family": {"signature": "<base58-encoded-signature>","family_head": "<text>","owner_signautre": "<owner signature from node to leave>"}}'  --mnemonic <mnemonic-from-node-to leave>
```

Using `nyxd`:

```
./nyxd tx wasm execute ${MIXNET-CONTRACT} '{"join_family": {"signature": "<base58-encoded-signature>","family_head": "<text>"}}' --node ${VALIDATOR-ENDPOINT} --from mix1 --chain-id nyx --gas-prices 0.025unym --gas auto --gas-adjustment 1.3 -y -b block
```



## Checking that your node is mixing correctly

### Network explorers
Once you've started your mix node and it connects to the validator, your node will automatically show up in the 'Mix nodes' section of either the Nym Network Explorers:

- [Mainnet](https://explorer.nymtech.net/overview)
- [Sandbox testnet](https://sandbox-explorer.nymtech.net/)

Enter your **identity key** to find your node. There are numerous statistics about your node on that page that are useful for checking your uptime history, packets mixed, and any delegations your node may have.

There are also 2 community explorers which have been created by [Nodes Guru](https://nodes.guru):

- [Mainnet](https://mixnet.explorers.guru/)
- [Sandbox testnet](https://sandbox.mixnet.explorers.guru/)

For more details see [Troubleshooting FAQ](../nodes/troubleshooting.md)

### Virtual IPs and hosting via Google & AWS
On some services (AWS, Google, etc), the machine's available bind address is not the same as the public IP address. In this case, bind `--host` to the local machine address returned by `ifconfig`, but also specify `--announce-host` with the public IP. Please make sure that you pass the correct, routable `--announce-host`.

For example, on a Google machine, you may see the following output from the `ifconfig` command:

```
ens4: flags=4163<UP,BROADCAST,RUNNING,MULTICAST>  mtu 1460
        inet 10.126.5.7  netmask 255.255.255.255  broadcast 0.0.0.0
        ...
```

The `ens4` interface has the IP `10.126.5.7`. But this isn't the public IP of the machine, it's the IP of the machine on Google's internal network. Google uses virtual routing, so the public IP of this machine is something else, maybe `36.68.243.18`.

`nym-mixnode init --host 10.126.5.7`, initalises the mix node, but no packets will be routed because `10.126.5.7` is not on the public internet.

Trying `nym-mixnode init --host 36.68.243.18`, you'll get back a startup error saying `AddrNotAvailable`. This is because the mix node doesn't know how to bind to a host that's not in the output of `ifconfig`.

The right thing to do in this situation is `nym-mixnode init --host 10.126.5.7 --announce-host 36.68.243.18`.

This will bind the mix node to the available host `10.126.5.7`, but announce the mix node's public IP to the directory server as `36.68.243.18`. It's up to you as a node operator to ensure that your public and private IPs match up properly.

## Metrics / API endpoints
The mix node binary exposes several API endpoints that can be pinged in order to gather information about the node, and the Nym API (previously 'Validator API') exposes numerous mix node related endpoints which provide network-wide information about mix nodes, the network topology (the list of avaliable mix nodes for packet routing), and information regarding uptime monitoring and rewarding history.

### Mix node API endpoints
Since the mix node binary exposes several API endpoints itself, you can ping these easily via curl:

| Endpoint             | Description                                                                           | Command                                                                                |
| -------------------- | ------------------------------------------------------------------------------------- | -------------------------------------------------------------------------------------- |
| `/description`       | Returns the description of the node set with the `describe` command                   | `curl <NODE_IP_ADDRESS>:8000/description`                                              |
| `/hardware`          | Returns the hardware information of the node                                          | `curl <NODE_IP_ADDRESS>:8000/hardware`                                                 |
| `/verloc`            | Returns the verloc information of the node, updated every 12 hours                    | `curl <NODE_IP_ADDRESS>:8000/verloc`                                                   |

The code for exposed API endpoints can be found [here](https://github.com/nymtech/nym/tree/release/{{platform_release_version}}/mixnode/src/node/http).

> You can get more detailed info by appending `?debug` to the URL, like so: `curl http://<NODE_IP_ADDRESS>:8000/stats?debug`

### Mix node related Nym API (previously 'Validator API') endpoints
Numerous endpoints are documented on the Nym API (previously 'Validator API')'s [Swagger Documentation](https://validator.nymtech.net/api/swagger/index.html). There you can also try out various requests from your broswer, and download the response from the API. Swagger will also show you what commands it is running, so that you can run these from an app or from your CLI if you prefer.

#### Mix node Reward Estimation API endpoint

The Reward Estimation API endpoint allows mix node operators to estimate the rewards they could earn for running a Nym mixnode with a specific `mix_id`.

> The `{mix_id}` can be found in the "Mix ID" column of the [Network Explorer](https://explorer.nymtech.net/network-components/mixnodes/active).

The endpoint is a particularly common for mix node operators as it can provide an estimate of potential earnings based on factors such as the amount of traffic routed through the mixnode, the quality of the mix node's performance, and the overall demand for mix nodes in the network. This information can be useful for mix node operators in deciding whether or not to run a mix node and in optimizing its operations for maximum profitability.

Using this API endpoint returns information about the Reward Estimation:

```
/status/mixnode/{mix_id}/reward-estimation
```

Query Response:

```
    "estimation": {
        "total_node_reward": "942035.916721770541325331",
        "operator": "161666.263307386408152071",
        "delegates": "780369.65341438413317326",
        "operating_cost": "54444.444444444444444443"
    },
```

> The unit of value is measured in `uNYM`.

- `estimated_total_node_reward` - An estimate of the total amount of rewards that a particular mix node can expect to receive during the current epoch. This value is calculated by the Nym Validator based on a number of factors, including the current state of the network, the number of mix nodes currently active in the network, and the amount of network traffic being processed by the mix node.

- `estimated_operator_reward` - An estimate of the amount of rewards that a particular mix node operator can expect to receive. This value is calculated by the Nym Validator based on a number of factors, including the amount of traffic being processed by the mix node, the quality of service provided by the mix node, and the operator's stake in the network.

- `estimated_delegators_reward` - An estimate of the amount of rewards that mix node delegators can expect to receive individually. This value is calculated by the Nym Validator based on a number of factors, including the amount of traffic being processed by the mix node, the quality of service provided by the mix node, and the delegator's stake in the network.

- `estimated_node_profit` - An estimate of the profit that a particular mix node operator can expect to earn. This value is calculated by subtracting the mix node operator's `operating_costs` from their `estimated_operator_reward` for the current epoch.

- `estimated_operator_cost` - An estimate of the total cost that a particular mix node operator can expect to incur for their participation. This value is calculated by the Nym Validator based on a number of factors, including the cost of running a mix node, such as server hosting fees, and other expenses associated with operating the mix node.

## Ports
All mix node-specific port configuration can be found in `$HOME/.nym/mixnodes/<your-id>/config/config.toml`. If you do edit any port configs, remember to restart your mix node.

### Mix node port reference
| Default port | Use                       |
| ------------ | ------------------------- |
| `1789`       | Listen for mixnet traffic |
| `1790`       | Listen for VerLoc traffic |
| `8000`       | Metrics http API endpoint |
