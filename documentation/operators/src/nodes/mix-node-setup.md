<!---
TODO
- [ ] Go through the entire doc and fix typos, coherency and conventions
- [x] Add a short point to run a mix node on tmux shell on VPS
- [x] Add path to target release
- [x] Change variables into a unified and explicit ones
- [x] Run the process along and create wallet screenshots of mixnode, not gateway
- [x] Add step 2/3 in the wallet bonding
- [x] Better picture alignment.
- [x] Add second console output to Initialising your mixnode manually as the automatized is not working
- [ ] Point users to understand the requirements for becoming a part of the active set.
- [ ] Include how to move a mix node from one VPS to another while keeping the delegation etc (ie init a new mix node and move `/home/nym/.nym/mixnodes/data/*` there ?)
- [ ] Explain more about self hosted nodes in the 6.1 not only as a troubleshooting
--->

# Mix Nodes

> The Nym mix node binary was built in the [building nym](https://nymtech.net/docs/binaries/building-nym.html) section. If you haven't yet built Nym and want to run the code, go there first.

> Any syntax in `<>` brackets is a user's unique variable. Exchange with a corresponding name without the `<>` brackets.

## Current version
```
<!-- cmdrun ../../../../target/release/nym-mixnode --version | grep "Build Version" | cut -b 21-26  -->
```

The `nym-mixnode` binary is currently one point version ahead of the rest of the platform binaries due to a patch applied between releases.

## Preliminary steps

There are a couple of steps that need completing before starting to set up your mix node:

- preparing your [wallet](https://nymtech.net/docs/wallet/desktop-wallet.html)
- requisitioning a VPS (Virtual Private Server)

### Wallet preparation

#### Mainnet

Before you initialise and run your mixnode, head to our [website](https://nymtech.net/download/) and download the Nym wallet for your operating system. If pre-compiled binaries for your operating system aren't available, you can build the wallet yourself with instructions [here](https://nymtech.net/docs/wallet/desktop-wallet.html).

If you don't already have one, please create a Nym address using the wallet, and fund it with tokens. The minimum amount required to bond a mixnode is 100 `NYM`, but make sure you have a bit more to account for gas costs.

`NYM` can be purchased via Bity from the wallet itself with BTC or fiat, and is currently present on several [exchanges](https://www.coingecko.com/en/coins/nym#markets).

> Remember that you can **only** use Cosmos `NYM` tokens to bond your mixnode. You **cannot** use ERC20 representations of `NYM` to run a node.

#### Sandbox testnet

Make sure to download a wallet and create an account as outlined above. Then head to our [token faucet](https://faucet.nymtech.net/) and get some tokens to use to bond it.

### VPS Hardware Specs

You will need to rent a VPS to run your mix node on. One key reason for this is that your node **must be able to send TCP data using both IPv4 and IPv6** (as other nodes you talk to may use either protocol).

For the moment, we haven't put a great amount of effort into optimizing concurrency to increase throughput, so don't bother provisioning a beastly server with multiple cores. This will change when we get a chance to start doing performance optimizations in a more serious way. Sphinx packet decryption is CPU-bound, so once we optimize, more fast cores will be better.

For now, see the below rough specs:

- Processors: 2 cores are fine. Get the fastest CPUs you can afford.
- RAM: Memory requirements are very low - typically a mix node may use only a few hundred MB of RAM.
- Disks: The mixnodes require no disk space beyond a few bytes for the configuration files.

## Mix node setup

Now that you have built the [codebase](https://nymtech.net/docs/binaries/building-nym.html), set up your [wallet](https://nymtech.net/docs/wallet/desktop-wallet.html), and have a VPS with the `nym-mixnode` binary, you can set up your mix node with the instructions below.

To begin, move to `/taget/release` directory from which you run the node commands:

```
cd target/release
```

### Viewing command help

You can check that your binaries are properly compiled with:

```
./nym-mixnode --help
```

Which should return a list of all available commands.

~~~admonish example collapsible=true title="Console output"
```
<!-- cmdrun ../../../../target/release/nym-mixnode --help -->
```
~~~

You can also check the various arguments required for individual commands with:

```
./nym-mixnode <COMMAND> --help
```

### Initialising your mix node

To check available configuration options for initializing your node use:

```
./nym-mixnode init --help
```

~~~admonish example collapsible=true title="Console output"
```
 <!-- cmdrun ../../../../target/release/nym-mixnode init --help -->
```
~~~

Initalise your mixnode with the following command, replacing the value of `--id` with the moniker you wish to give your mixnode, and the `--wallet-address` with the Nym address you created earlier. Your `--host` must be publicly routable on the internet in order to mix packets, and can be either an Ipv4 or IPv6 address. The `$(curl ifconfig.me)` command returns your IP automatically using an external service. If you enter your IP address manually, enter it **without** any port information.

```
./nym-mixnode init --id <NODE_NAME> --host $(curl ifconfig.me) --wallet-address <WALLET_ADDRESS>
```

<!---serinko: The automatized command did not work, printing the output manually--->
~~~admonish example collapsible=true title="Console output"
```
.nym-mixnode init --id <YOUR_ID> --host $(curl ifconfig.me) --wallet-address <WALLET_ADDRESS>


Initialising mixnode <YOUR_ID>...
Saved mixnet identity and sphinx keypairs
 2023-06-04T08:20:32.862Z INFO  nym_config > Configuration file will be saved to "/home/<USER>/.nym/mixnodes/<YOUR_ID>/config/config.toml"
Saved configuration file to "/home/<USER>/.nym/mixnodes/<YOUR_ID>/config/config.toml"
Mixnode configuration completed.

      _ __  _   _ _ __ ___
     | '_ \| | | | '_ \ _ \
     | | | | |_| | | | | | |
     |_| |_|\__, |_| |_| |_|
            |___/

             (nym-mixnode - version {{mix_node_release_version}})


Identity Key: DhmUYedPZvhP9MMwXdNpPaqCxxTQgjAg78s2nqtTTiNF","version":"{{mix_node_release_version}}"},"cost_params
Sphinx Key: CfZSy1jRfrfiVi9JYexjFWPqWkKoY72t7NdpWaq37K8Z
Host: 62.240.134.189 (bind address: 62.240.134.189)
Version: {{mix_node_release_version}}
Mix Port: 1789, Verloc port: 1790, Http Port: 8000
```
~~~

> The `init` command will refuse to destroy existing mix node keys.

During the `init` process you will have the option to change the `http_api`, `verloc` and `mixnode` ports from their default settings. If you wish to change these in the future you can edit their values in the `config.toml` file created by the initialization process, which is located at `~/.nym/mixnodes/<YOUR_ID>/`.

### Bonding your mix node

```admonish caution
From `v1.1.3`, if you unbond your mixnode that means you are leaving the mixnet and you will lose all your delegations (permanently). You can join again with the same identity key, however, you will start with **no delegations**.
```

#### Bond via the Desktop wallet (recommended)

You can bond your mix node via the Desktop wallet.

* Open your wallet, and head to the `Bond` page, then select the node type `Mixnode` and input your node details. Press `Next`.

* Enter the `Amount`, `Operating cost` and `Profit margin` and press `Next`.

* You will be asked to run a the `sign` command with your `gateway` - copy and paste the long signature as the value of `--contract-msg` and run it. 

```
./nym-mixnode sign --id <YOUR_ID> --contract-msg <PAYLOAD_GENERATED_BY_THE_WALLET>
```

It will look something like this:

~~~admonish example collapsible=true title="Console output"
```
./nym-mixnode sign --id upgrade_test --contract-msg 22Z9wt4PyiBCbMiErxj5bBa4VCCFsjNawZ1KnLyMeV9pMUQGyksRVANbXHjWndMUaXNRnAuEVJW6UCxpRJwZe788hDt4sicsrv7iAXRajEq19cWPVybbUqgeo76wbXbCbRdg1FvVKgYZGZZp8D72p5zWhKSBRD44qgCrqzfV1SkiFEhsvcLUvZATdLRocAUL75KmWivyRiQjCE1XYEWyRH9yvRYn4TymWwrKVDtEB63zhHjATN4QEi2E5qSrSbBcmmqatXsKakbgSbQoLsYygcHx7tkwbQ2HDYzeiKP1t16Rhcjn6Ftc2FuXUNnTcibk2LQ1hiqu3FAq31bHUbzn2wiaPfm4RgqTwGM4eqnjBofwR3251wQSxbYwKUYwGsrkweRcoPuEaovApR9R19oJ7GVG5BrKmFwZWX3XFVuECe8vt1x9MY7DbQ3xhAapsHhThUmzN6JPPU4qbQ3PdMt3YVWy6oRhap97ma2dPMBaidebfgLJizpRU3Yu7mtb6E8vgi5Xnehrgtd35gitoJqJUY5sB1p6TDPd6vk3MVU1zqusrke7Lvrud4xKfCLqp672Bj9eGb2wPwow643CpHuMkhigfSWsv9jDq13d75EGTEiprC2UmWTzCJWHrDH7ka68DZJ5XXAW67DBewu7KUm1jrJkNs55vS83SWwm5RjzQLVhscdtCH1Bamec6uZoFBNVzjs21o7ax2WHDghJpGMxFi6dmdMCZpqn618t4

      _ __  _   _ _ __ ___
     | '_ \| | | | '_ \ _ \
     | | | | |_| | | | | | |
     |_| |_|\__, |_| |_| |_|
            |___/

             (nym-mixnode - version {{mix_node_release_version}})


>>> attempting to sign 22Z9wt4PyiBCbMiErxj5bBa4VCCFsjNawZ1KnLyMeV9pMUQGyksRVANbXHjWndMUaXNRnAuEVJW6UCxpRJwZe788hDt4sicsrv7iAXRajEq19cWPVybbUqgeo76wbXbCbRdg1FvVKgYZGZZp8D72p5zWhKSBRD44qgCrqzfV1SkiFEhsvcLUvZATdLRocAUL75KmWivyRiQjCE1XYEWyRH9yvRYn4TymWwrKVDtEB63zhHjATN4QEi2E5qSrSbBcmmqatXsKakbgSbQoLsYygcHx7tkwbQ2HDYzeiKP1t16Rhcjn6Ftc2FuXUNnTcibk2LQ1hiqu3FAq31bHUbzn2wiaPfm4RgqTwGM4eqnjBofwR3251wQSxbYwKUYwGsrkweRcoPuEaovApR9R19oJ7GVG5BrKmFwZWX3XFVuECe8vt1x9MY7DbQ3xhAapsHhThUmzN6JPPU4qbQ3PdMt3YVWy6oRhap97ma2dPMBaidebfgLJizpRU3Yu7mtb6E8vgi5Xnehrgtd35gitoJqJUY5sB1p6TDPd6vk3MVU1zqusrke7Lvrud4xKfCLqp672Bj9eGb2wPwow643CpHuMkhigfSWsv9jDq13d75EGTEiprC2UmWTzCJWHrDH7ka68DZJ5XXAW67DBewu7KUm1jrJkNs55vS83SWwm5RjzQLVhscdtCH1Bamec6uZoFBNVzjs21o7ax2WHDghJpGMxFi6dmdMCZpqn618t4
>>> decoding the message...
>>> message to sign: {"nonce":0,"algorithm":"ed25519","message_type":"mixnode-bonding","content":{"sender":"n1eufxdlgt0puwrwptgjfqne8pj4nhy2u5ft62uq","proxy":null,"funds":[{"denom":"unym","amount":"100000000"}],"data":{"mix_node":{"host":"62.240.134.189","mix_port":1789,"verloc_port":1790,"http_api_port":8000,"sphinx_key":"CfZSy1jRfrfiVi9JYexjFWPqWkKoY72t7NdpWaq37K8Z","identity_key":"DhmUYedPZvhP9MMwXdNpPaqCxxTQgjAg78s2nqtTTiNF","version":"1.1.14"},"cost_params":{"profit_margin_percent":"0.1","interval_operating_cost":{"denom":"unym","amount":"40000000"}}}}}
```
~~~

* Copy the resulting signature:

```
>>> The base58-encoded signature is:
2GbKcZVKFdpi3sR9xoJWzwPuGdj3bvd7yDtDYVoKfbTWdpjqAeU8KS5bSftD5giVLJC3gZiCg2kmEjNG5jkdjKUt
```

* And paste it into the wallet nodal, press `Next` and confirm the transaction.

![Paste Signature](../images/wallet-screenshots/wallet-sign.png)

* Your node will now be bonded and ready to mix at the beginning of the next epoch (at most 1 hour).

> You are asked to `sign` a transaction on bonding so that the mixnet smart contract is able to map your nym address to your node. This allows us to create a nonce for each account and defend against replay attacks.

#### Bond via the CLI (power users)
If you want to bond your mix node via the CLI, then check out the [relevant section in the Nym CLI](https://nymtech.net/docs/tools/nym-cli.html#bond-a-mix-node) docs.

### Running your mix node

Now you've bonded your mix node, run it with:

```
./nym-mixnode run --id <YOUR_ID>
```

If everything worked, you'll see your node running on the either the [Sandbox testnet network explorer](https://sandbox-explorer.nymtech.net) or the [mainnet network explorer](https://explorer.nymtech.net), depending on which network you're running.

Note that your node's public identity key is displayed during startup, you can use it to identify your node in the list.

<!---serinko - propose to drop this sentence:

Keep reading to find out more about configuration options or troubleshooting if you're having issues. There are also some tips for running on AWS and other cloud providers, some of which require minor additional setup.

--->

Have a look at the saved configuration files in `$HOME/.nym/mixnodes/` to see more configuration options.

### Describe your mix node (optional)

In order to easily identify your node via human-readable information later on in the development of the testnet when delegated staking is implemented, you can `describe` your mixnode with the following command:

```
./nym-mixnode describe --id <YOUR_ID>
```

> Remember to restart your mix node process in order for the new description to be propagated.

### Upgrading your mix node

Upgrading your node is a two-step process:
* Updating the binary and `~/.nym/mixnodes/<YOUR_ID>/config.toml` on your VPS
* Updating the node information in the [mixnet smart contract](../nyx/mixnet-contract.md). **This is the information that is present on the [mixnet explorer](https://explorer.nymtech.net)**.

#### Step 1: Upgrading your binary
Follow these steps to upgrade your mix node binary and update its config file:
* pause your mix node process.
* replace the existing binary with the newest binary (which you can either [compile yourself](https://nymtech.net/docs/binaries/building-nym.html) or grab from our [releases page](https://github.com/nymtech/nym/releases)).
* re-run `init` with the same values as you used initially. **This will just update the config file, it will not overwrite existing keys**.
* restart your mix node process with the new binary.

#### Step 2: Updating your node information in the smart contract
Follow these steps to update the information about your mix node which is publically avaliable from the [Nym API](https://validator.nymtech.net/api/swagger/index.html) and information displayed on the [mixnet explorer](https://explorer.nymtech.net).

You can either do this graphically via the Desktop Wallet, or the CLI.

#### Updating node information via the Desktop Wallet
* Navigate to the `Bonding` page and click the `Node Settings` link in the top right corner:
\
![Bonding page](../images/wallet-screenshots/bonding.png)

* Update the fields in the `Node Settings` page and click `Submit changes to the blockchain`.
\
![Node Settings Page](../images/wallet-screenshots/node_settings.png)

#### Updating node information via the CLI
If you want to bond your mix node via the CLI, then check out the [relevant section in the Nym CLI](../../documentation/docs/src/tools/nym-cli.md#upgrade-a-mix-node) docs.

### Displaying mix node information

You can always check the details of your mix node with the `node-details` command:

```
./nym-mixnode node-details --id <YOUR_ID>
```

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

Finally open your mix node's p2p port, as well as ports for ssh and ports `8000` and `1790` for verloc and measurement pings:

```
sudo ufw allow 1789,1790,8000,22/tcp

# check the status of the firewall
sudo ufw status
```

For more information about your mix node's port configuration, check the [mix node port reference table](./mix-node-setup.md#mixnode-port-reference) below.

### Automating your mix node with tmux and systemd

It's useful to have the mix node automatically start at system boot time. 

#### tmux

One way is to use `tmux` shell on top of your current VPS terminal. Tmux is a terminal multiplexer, it allows you to create several terminal windows and panes from a single terminal. Processes started in `tmux` keep running after closing the terminal as long as the given `tmux` window was not terminated.  

Use the following command to get `tmux`.  

Platform|Install Command
---|---
Arch Linux|`pacman -S tmux`
Debian or Ubuntu|`apt install tmux`
Fedora|`dnf install tmux`
RHEL or CentOS|`yum install tmux`
macOS (using Homebrew|`brew install tmux`
macOS (using MacPorts)|`port install tmux`
openSUSE|`zypper install tmux`
  
In case it didn't work for your distribution, see how to build `tmux` from [version control](https://github.com/tmux/tmux#from-version-control).  

**Running tmux**

No when you installed tmux on your VPS, let's run a mixnode on tmux, which allows you to detach your terminal and let the mixnode run on its own on the VPS.

* Pause your mixnode
* Start tmux with the command 
```
tmux
```
* The terminal should stay in the same directory, just the layout changed into tmux default layout.
* Start the mixnode again with a command:
```
./nym-mixnode run --id <YOUR_ID>
```
* Now, without closing the tmux window, you can close the whole terminal and the mixnode (and any other process running in tmux) will stay active.
* Next time just start your teminal, ssh into the VPS and run the following command to attach back to your previous session:
```
tmux attach session
```
* To see keybinding options of tmux press `ctrl`+`b` and after 1 second `?`

#### systemd

Here's a systemd service file to do that:

```ini
[Unit]
Description=Nym Mixnode ({{mix_node_release_version}})
StartLimitInterval=350
StartLimitBurst=10

[Service]
User=<USER>
LimitNOFILE=65536
ExecStart=/home/<USER>/<PATH>/nym-mixnode run --id <YOUR_ID>
KillSignal=SIGINT
Restart=on-failure
RestartSec=30

[Install]
WantedBy=multi-user.target
```

Put the above file onto your system at `/etc/systemd/system/nym-mixnode.service`.

Change the `<PATH>` in `ExecStart` to point at your mix node binary (`nym-mixnode`), and the `<USER>` so it is the user you are running as.

If you have built nym in the `$HOME` directory on your server, and your username is `jetpanther`, then the start command might look like this:

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

In case you chose tmux option for mixnode automatization, see your `ulimit` list by running:

```
ulimit -a

# watch for the output line -n
-n: file descriptors          1024      
```

You can change it either by running a command:

```
ulimit -u -n 4096
```

or editing `etc/security/conf` and add the following lines:

```
# Example hard limit for max opened files
username        hard nofile 4096

# Example soft limit for max opened files
username        soft nofile 4096
```

Then reboot your server and restart your mixnode.

## Node Description
Node description is a short text that describes your node. It is displayed in the `./nym-mixnode list` command and in the `./nym-mixnode node-details` command. It also shows up in the node explorer to let people know what your node is about and link to your website.

To set your node description, create a file called `description.toml` and put it in the same directory as your `config.toml` file (`~/.nym/mixnodes/<YOUR_ID>/description.toml`). The file should look like this example:

```toml
name = "Winston Smith"
description = "I am the Sphinx"
link = "https://nymtech.net"
location = "Giza, Egypt"
```

You will need to restart your node for the changes to take effect.

## Node Families

Node family involves setting up a group of mix nodes that work together to provide greater privacy and security for network communications. This is achieved by having the nodes in the family share information and routes, creating a decentralized network that makes it difficult for third parties to monitor or track communication traffic.

### Create a Node Family

To create a Node family, you will need to install and configure multiple mix nodes, and then use the CLI to link them together into a family. Once your Node family is up and running, you can use it to route your network traffic through a series of nodes, obscuring the original source and destination of the communication.

You can use either `nym-cli` which can be downloaded from the [release page](https://github.com/nymtech/nym/releases) or compiling `nyxd`.


Change directory by `cd <PATH>/<TO>/<THE>/<RELEASE>` and run the following on the family head to obtain the signature for the member:

```
./nym-mixnode sign --id <YOUR_ID> --text <TEXT>
```

~~~admonish example collapsible=true title="Console output"
```
 <!-- cmdrun ../../../../target/release/nym-mixnode sign --id YOUR_ID --text "TEXT" -->
```
~~~

Using `nym-cli`:

> `--mnemonic` is the mnemonic of the member wanting to be the head of family.

```
/nym-cli cosmwasm execute <WALLET_ADDRESS> '{"create_family": {"signature": "<base58-encoded-signature>","family_head": "<TEXT>","owner_signature":"<NODE_OWNER_SIGNATURE>","label": "<NODE_LABEL>"}}' --mnemonic <MNEMONIC_FROM_THE_NODE_TO_THE_HEAD>
```

Using `nyxd`:

> `--from` is mnemonic of the member wanting to join the family.

```
./nyxd tx wasm execute ${MIXNET-CONTRACT} '{"join_family": {"signature": "<base58-encoded-signature>","family_head": "<TEXT>"}}' --node ${VALIDATOR-ENDPOINT} --from mix1 --chain-id nyx --gas-prices 0.025unym --gas auto --gas-adjustment 1.3 -y -b block
```

To get the node owner signature, use:

`./nym-mixnode node-details --id <NODE_ID>`

### Joining a Node Family

Change directory by `cd <PATH>/<TO>/<THE>/<RELEASE>` and run the following on the family head to obtain the signature for the member:

```
./nym-mixnode sign --id <YOUR_ID> --text <TEXT>
```

~~~admonish example collapsible=true title="Console output"
```
 <!-- cmdrun ../../../../target/release/nym-mixnode sign --id YOUR_ID --text "TEXT" -->
```
~~~

Using `nym-cli`:

```
./nym-cli cosmwasm execute <WALLET_ADDRESS> '{"join_family": {"signature": "<base58-encoded-signature>","family_head": "<TEXT>","owner_signautre": "<OWNER_SIGNATURE_FROM_NODE_TO_JOIN>", "label":"<NODE_TO_JOIN_LABEL>"}}'  --mnemonic <MNEMONIC_FROM_NODE_TO_JOIN>
```

Using `nyxd`:

```
./nyxd tx wasm execute ${MIXNET-CONTRACT} '{"join_family": {"signature": "<base58-encoded-signature>","family_head": "<TEXT>"}}' --node ${VALIDATOR-ENDPOINT} --from mix1 --chain-id nyx --gas-prices 0.025unym --gas auto --gas-adjustment 1.3 -y -b block
```


To get the node owner signature, use:

`./nym-mixnode node-details --id <NODE_ID>`


### Leaving a family
If wanting to leave, run the same initial command as above, followed by:

Using `nym-cli`:
<!---the sting under shall be changed to <NODE_ADDRESS>? --->
```
./nym-cli cosmwasm execute <WALLET_ADDRESS> '{"leave_family": {"signature": "<base58-encoded-signature>","family_head": "<TEXT>","owner_signautre": "<OWNER_IGNATURE_FROM_NODE_TO_LEAVE>"}}'  --mnemonic <MNEMONIC_FROM_NODE_TO_LEAVE>
```

Using `nyxd`:

```
./nyxd tx wasm execute ${MIXNET-CONTRACT} '{"join_family": {"signature": "<base58-encoded-signature>","family_head": "<TEXT>"}}' --node ${VALIDATOR-ENDPOINT} --from mix1 --chain-id nyx --gas-prices 0.025unym --gas auto --gas-adjustment 1.3 -y -b block
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

<!---Enter information how to higher chances to become a part of an active set--->

### Virtual IPs and hosting via Google & AWS
For tru internet decentralization we encourage operators to use diverse VPS providers instead of the largest companies offering such services. If for some reasons you have already running AWS or Google and want to setup a mixnode there, please read the following.

On some services (AWS, Google, etc) the machine's available bind address is not the same as the public IP address. In this case, bind `--host` to the local machine address returned by `$(curl ifconfig.me)`, but also specify `--announce-host` with the public IP. Please make sure that you pass the correct, routable `--announce-host`.

For example, on a Google machine, you may see the following output from the `ifconfig` command:

```
ens4: flags=4163<UP,BROADCAST,RUNNING,MULTICAST>  mtu 1460
        inet 10.126.5.7  netmask 255.255.255.255  broadcast 0.0.0.0
        ...
```

The `ens4` interface has the IP `10.126.5.7`. But this isn't the public IP of the machine, it's the IP of the machine on Google's internal network. Google uses virtual routing, so the public IP of this machine is something else, maybe `36.68.243.18`.

`./nym-mixnode init --host 10.126.5.7`, initalises the mix node, but no packets will be routed because `10.126.5.7` is not on the public internet.

Trying `nym-mixnode init --host 36.68.243.18`, you'll get back a startup error saying `AddrNotAvailable`. This is because the mix node doesn't know how to bind to a host that's not in the output of `ifconfig`.

The right thing to do in this situation is to init with a command:
```
./nym-mixnode init --host 10.126.5.7 --announce-host 36.68.243.18
```

This will bind the mix node to the available host `10.126.5.7`, but announce the mix node's public IP to the directory server as `36.68.243.18`. It's up to you as a node operator to ensure that your public and private IPs match up properly.

To find the right IP configuration, contact your VPS provider for support.

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

> You can get more detailed info by appending `?debug` to the URL, like so: 
> ```
> curl http://<NODE_IP_ADDRESS>:8000/stats?debug
> ```

### Mix node related Nym API (previously 'Validator API') endpoints
Numerous endpoints are documented on the Nym API (previously 'Validator API')'s [Swagger Documentation](https://validator.nymtech.net/api/swagger/index.html). There you can also try out various requests from your browser, and download the response from the API. Swagger will also show you what commands it is running, so that you can run these from an app or from your CLI if you prefer.

#### Mix node Reward Estimation API endpoint

The Reward Estimation API endpoint allows mixnode operators to estimate the rewards they could earn for running a Nym mixnode with a specific `MIX_ID`.

> The `<MIX_ID>` can be found in the "Mix ID" column of the [Network Explorer](https://explorer.nymtech.net/network-components/mixnodes/active).

The endpoint is a particularly common for mixnode operators as it can provide an estimate of potential earnings based on factors such as the amount of traffic routed through the mixnode, the quality of the mixnode's performance, and the overall demand for mixnodes in the network. This information can be useful for mixnode operators in deciding whether or not to run a mix node and in optimizing its operations for maximum profitability.

Using this API endpoint returns information about the Reward Estimation:

```
/status/mixnode/<MIX_ID>/reward-estimation
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
All mix node-specific port configuration can be found in `$HOME/.nym/mixnodes/<YOUR_ID>/config/config.toml`. If you do edit any port configs, remember to restart your mix node.

### Mix node port reference
| Default port | Use                       |
| ------------ | ------------------------- |
| `1789`       | Listen for mixnet traffic |
| `1790`       | Listen for VerLoc traffic |
| `8000`       | Metrics http API endpoint |
/

