# Gateways

> The Nym gateway was built in the [building nym](../binaries/building-nym.md) section. If you haven't yet built Nym and want to run the code, go there first.


```admonish info
As a result of [Project Smoosh](../faq/smoosh-faq.md), the current version of `nym-gateway` binary also contains `nym-network-requester` functionality which can be enabled [by the operator](./gateway-setup.md#initialising-gateway-with-network-requester). This combination is a basis of Nym exit gateway node - an essential piece in our new setup. Please read more in our [Project Smoosh FAQ](../faq/smoosh-faq.md) and [Exit Gateways Page](../legal/exit-gateway.md). We recommend operators begin to shift their setups to this new combined node, instead of operating two separate binaries.
```
> Any syntax in `<>` brackets is a user's unique variable. Exchange with a corresponding name without the `<>` brackets.

## Current version
```
<!-- cmdrun ../../../../target/release/nym-gateway --version | grep "Build Version" | cut -b 21-26  -->
```

## Preliminary steps

Make sure you do the preparation listed in the [preliminary steps page](../preliminary-steps.md) before setting up your gateway.


## Gateway setup
Now that you have built the codebase, set up your wallet, and have a VPS with the `nym-gateway` binary, you can set up your gateway with the instructions below.  

To begin, move to `/target/release` directory from which you run the node commands:

```
cd target/release
```

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
./nym-gateway <COMMAND> --help
```
> Adding `--no-banner` startup flag will prevent Nym banner being printed even if run in tty environment.

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

The following command returns a gateway on your current IP with the `<ID>` of `supergateway`:

```
./nym-gateway init --id supergateway --host $(curl ifconfig.me)
```

~~~admonish example collapsible=true title="Console output"
```
<!-- cmdrun ../../../../target/release/nym-gateway init --id supergateway --host $(curl ifconfig.me) -->
```
~~~

The `$(curl ifconfig.me)` command above returns your IP automatically using an external service. Alternatively, you can enter your IP manually if you wish. If you do this, remember to enter your IP **without** any port information.


#### Initialising gateway with network requester

As some of the [Project Smoosh](../faq/smoosh-faq.md) changes getting implemented, network requester is smooshed with gateways. Such combination creates an exit gateway node, needed for new more open setup. 

An operator can initialise the exit gateway functionality by:

```
./nym-gateway init --id <ID> --host $(curl ifconfig.me) --with-network-requester
```

If we follow the previous example with `<ID>` chosen `superexitgateway`, adding the `--with-network-requester` flag, the outcome will be:


~~~admonish example collapsible=true title="Console output"
```
<!-- cmdrun ../../../../target/release/nym-gateway init --id superexitgateway --host $(curl ifconfig.me) --with-network-requester -->
```
~~~

You can see that the printed information besides *identity* and *sphinx keys* also includes a long string called *address*. This is the address to be provided to your local [socks5 client](https://nymtech.net/docs/clients/socks5-client.html) as a `--provider` if you wish to connect to your own exit gateway.  

#### Add network requester to existing gateway

If you already run a gateway and got it [upgraded](./maintenance.md#upgrading-your-node) to the [newest version](./gateway-setup.md#current-version), you can easily change its functionality to exit gateway. PAuse the gateway and run a command `setup-network-requester`.

See the options:

```
./nym-gateway setup-network-requester --help
```

~~~admonish example collapsible=true title="Console output"
```
<!-- cmdrun ../../../../target/release/nym-gateway setup-network-requester --help -->
```
~~~

Run with `--enabled true` flag choosing `<ID>` as `supergateway`:

```
./nym-gateway setup-network-requester --enabled true --id supergateway 
```

~~~admonish example collapsible=true title="Console output"
```
<!-- cmdrun ../../../../target/release/nym-gateway setup-network-requester --enabled true --id supergateway -->
```
~~~

In case there are any problems, you can also change it manually by editing the gateway config stored in `/home/user/.nym/gateways/<ID>/config/config.toml` where the line under `[network_requester]` needs to be edited from `false` to `true`.

```
[network_requester]
# Specifies whether network requester service is enabled in this process.
enabled = true
```

Save, exit and restart your gateway. Now it is a post-smooshed exit gateway.

All information about network requester part of your exit gateway is in `/home/user/.nym/gateways/snus/config/network_requester_config.toml`.

To read more about the configuration like whitelisted outbound requesters in `allowed.list` and other useful information, see the page [*Network Requester Whitelist*](network-requester-setup.md#using-your-network-requester).


```admonish info
Before you bond and run your gateway, please make sure the [firewall configuration](./maintenance.md#configure-your-firewall) is setup so your gateway can be reached from the outside.
```

### Bonding your gateway

#### Via the Desktop wallet

You can bond your gateway via the Desktop wallet.

1. Open your wallet, and head to the `Bonding` page, then select the node type `Gateway` and input your node details. Press `Next`.

2. Enter the `Amount`, `Operating cost` and press `Next`.

3. You will be asked to run a the `sign` command with your `gateway` - copy and paste the long signature as the value of `--contract-msg` and run it. 

```
./nym-gatewway sign --id <YOUR_ID> --contract-msg <PAYLOAD_GENERATED_BY_THE_WALLET>
```

It will look something like this:

~~~admonish example collapsible=true title="Console output"
```
./nym-gateway sign --id supergateway --contract-msg 2Mf8xYytgEeyJke9LA7TjhHoGQWNBEfgHZtTyy2krFJfGHSiqy7FLgTnauSkQepCZTqKN5Yfi34JQCuog9k6FGA2EjsdpNGAWHZiuUGDipyJ6UksNKRxnFKhYW7ri4MRduyZwbR98y5fQMLAwHne1Tjm9cXYCn8McfigNt77WAYwBk5bRRKmC34BJMmWcAxphcLES2v9RdSR68tkHSpy2C8STfdmAQs3tZg8bJS5Qa8pQdqx14TnfQAPLk3QYCynfUJvgcQTrg29aqCasceGRpKdQ3Tbn81MLXAGAs7JLBbiMEAhCezAr2kEN8kET1q54zXtKz6znTPgeTZoSbP8rzf4k2JKHZYWrHYF9JriXepuZTnyxAKAxvGFPBk8Z6KAQi33NRQkwd7MPyttatHna6kG9x7knffV6ebGzgRBf7NV27LurH8x4L1uUXwm1v1UYCA1WSBQ9Pp2JW69k5v5v7G9gBy8RUcZnMbeL26Qqb8WkuGcmuHhaFfoqSfV7PRHPpPT4M8uRqUyR4bjUtSJJM1yh6QSeZk9BEazzoJqPeYeGoiFDZ3LMj2jesbJweQR4caaYuRczK92UGSSqu9zBKmE45a


      _ __  _   _ _ __ ___
     | '_ \| | | | '_ \ _ \
     | | | | |_| | | | | | |
     |_| |_|\__, |_| |_| |_|
            |___/

             (nym-gateway - version v1.1.29)  


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

* And paste it into the wallet nodal, press `Next` and confirm the transaction.

![Paste Signature](../images/wallet-screenshots/wallet-gateway-sign.png)

* Your gateway is now bonded.

> You are asked to `sign` a transaction on bonding so that the mixnet smart contract is able to map your nym address to your node. This allows us to create a nonce for each account and defend against replay attacks.

#### Via the CLI (power users)
If you want to bond your mix node via the CLI, then check out the [relevant section in the Nym CLI](https://nymtech.net/docs/tools/nym-cli.html#bond-a-mix-node) docs.

### Running your gateway
The `run` command starts the gateway:

```
./nym-gateway run --id <ID>
```
## Maintenance

For gateway upgrade, firewall setup, port configuration, API endpoints, VPS suggestions, automation and more, see the [maintenance page](./maintenance.md)

