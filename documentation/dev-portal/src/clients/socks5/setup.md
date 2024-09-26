# Setup & Run Nym Socks5 Client

> `nym-socks5-client` now also supports SOCKS4 and SOCKS4A protocols as well as SOCKS5.

The Nym socks5 client allows you to proxy traffic from a desktop application through the mixnet, meaning you can send and receive information from remote application servers without leaking metadata which can be used to deanonymise you, even if you're using an encrypted application such as Signal.

```admonish info
Since the beginning of 2024 NymConnect is no longer maintained. Nym is developing a new client called [NymVPN](https://nymvpn.com), an application routing all users traffic thorugh the mixnet.
If users want to route their traffic through socks5 we advice to use this client. If you want to run deprecated NymConnect, visit [NymConnect archive page](../../archive/nym-connect.md) with setup and application examples.
```

## Setup

### Download or compile socks5 client

If you are using OSX or a Debian-based operating system, you can download the `nym-socks5-client` binary from our [Github releases page](https://github.com/nymtech/nym/releases).

If you are using a different operating system, head over to the [Building from Source](https://nymtech.net/docs/binaries/building-nym.html) page for instructions on how to build the repository from source.

### Initialise Socks5 Client

To initialise your `nym-socks5-client` you need to have an address of a Network Requester (NR). Nowadays NR is part of every Exit Gateway (`nym-node --mode exit-gateway`). The easiest way to get a NR address is to visit [harbourmaster.nymtech.net](https://harbourmaster.nymtech.net/) and open the tab called *SOCKS5 NETWORK REQUESTERS*. There you can filter the NR by Gateways identity address, and other options.

Use the following command to initialise `nym-socs5-client` where `<ID>` can be anything you want (it's only for local config file storage identification and never shared on the network) and `<PROVIDER>` is suplemented with a NR address:

```
./nym-socks5-client init --id <ID> --provider <PROVIDER>
```

~~~admonish tip
Another option to find a NR address associated with a Gateway is to query nodes [*Self Described* API endpoint](https://validator.nymtech.net/api/v1/gateways/described) where the NR address is noted like in this example:
```sh
"network_requester": {
    "address": "CyuN49nkyeuiLohSpV5A1MbSqcugHLJQ95B5HooCpjv8.CguTh45Vp99QuGWZRBKpBjZDQbsJaHaXqAMGyc4Qhkzp@2w5RduXRqxKgHt1wtp4qGA4AfXaBj8TuUj1LvcPe2Ea1",
    "uses_exit_policy": true
}
```
~~~

## Run

Now your client is initialised, start it with the following:

```
./nym-socks5-client run --id <ID>
```

## Useful commands

### Viewing Command `--help`

You can check that your binaries are properly compiled with:

```
./nym-socks5-client --help
```

~~~admonish example collapsible=true title="Console output"
```
<!-- cmdrun ../../../../../target/release/nym-socks5-client --help -->
```
~~~

You can check the necessary parameters for the available commands by running:

```
./nym-socks5-client <COMMAND> --help
```
For example `./nym-socks5-client run --help` will return all options associated with `run` command.

~~~admonish example collapsible=true title="Console output"
```
<!-- cmdrun ../../../../../target/release/nym-socks5-client run --help -->
```
~~~

### `build-info`

A `build-info` command prints the build information like commit hash, rust version, binary version just like what command `--version` does. However, you can also specify an `--output=json` flag that will format the whole output as a json, making it an order of magnitude easier to parse.

### Flags & Arguments

* `--id`: A local identifier so that you can name your clients and keep track of them on your local system; it is **never** transmitted over the network.

* `--use-reply-surbs`: This field denotes whether you wish to send [SURBs](https://nymtech.net/docs/architecture/traffic-flow.md#private-replies-using-surbs) along with your request. It defaults to `false` and must be explicitly set to `true` to activate.

* `--use-anonymous-replies `: Specifies whether this client is going to use an anonymous sender tag for communication with the service provider. While this is going to hide its actual address information, it will make the actual communication slower and consume nearly double the bandwidth as it will require sending reply SURBs.

* `--gateway`: By default your client will choose a random gateway to connect to. However, there are several options for choosing a gateway, if you do not want one that is randomly assigned to your client:

* `--latency-based-selection`: This flag will choose a gateway based on its location relative to your client. This argument means that to select a gateway, your client will:
	* fetch a list of all availiable gateways
	* send few ping messages to all of them, and measure response times.
	* create a weighted distribution to randomly choose one, favouring ones with lower latency.

> Note this doesn't mean that your client will pick the closest gateway to you, but it will be far more likely to connect to gateway with a 20ms ping rather than 200ms

## Configuring `nym-socks5-client`

When you initalise a client instance, a configuration directory will be generated and stored in `$HOME_DIR/.nym/socks5-clients/<YOUR_CLIENT_ID>/`.

```
tree $HOME/.nym/socks5-clients/<YOUR_CLIENT_ID>
├── config
│   └── config.toml
└── data
    ├── ack_key.pem
    ├── credentials_database.db
    ├── gateways_registrations.sqlite
    ├── persistent_reply_store.sqlite
    ├── private_encryption.pem
    ├── private_identity.pem
    ├── public_encryption.pem
    └── public_identity.pem
```

The `config.toml` file contains client configuration options, while the two `pem` files contain client key information.

The generated files contain the client name, public/private keypairs, and gateway address. The name `<YOUR_CLIENT_ID>` in the example above is just a local identifier so that you can name your clients.

### Configuring your client for Docker

By default, the native client listens to host `127.0.0.1`. However this can be an issue if you wish to run a client in a Dockerized environment, where it can be convenenient to listen on a different host such as `0.0.0.0`.

You can set this via the `--host` flag during either the `init` or `run` commands.

Alternatively, a custom host can be set in the `config.toml` file under the `socket` section. If you do this, remember to restart your client process.

### Automating your socks5 client with systemd

Create a service file for the socks5 client at `/etc/systemd/system/nym-socks5-client.service`:

```ini
[Unit]
Description=Nym Socks5 Client
StartLimitInterval=350
StartLimitBurst=10

[Service]
User=nym # replace this with whatever user you wish
LimitNOFILE=65536
ExecStart=/home/nym/nym-socks5-client run --id <your_id>
KillSignal=SIGINT
Restart=on-failure
RestartSec=30

[Install]
WantedBy=multi-user.target
```

Now enable and start your socks5 client:

```
systemctl enable nym-socks5-client.service
systemctl start nym-socks5-client.service

# you can always check your socks5 client has succesfully started with:
systemctl status nym-socks5-client.service
```
