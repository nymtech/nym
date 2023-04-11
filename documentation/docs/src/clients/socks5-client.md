# Socks5 Client

> The Nym socks5 client was built in the [building nym](../binaries/building-nym.md) section. If you haven't yet built Nym and want to run the code on this page, go there first.

Many existing applications are able to use either the SOCKS4, SOCKS4A, or SOCKS5 proxy protocols. If you want to send such an application's traffic through the mixnet, you can use the `nym-socks5-client` to bounce network traffic through the Nym network, like this:

```
                                                                              External Systems:
                                                                                     +--------------------+
                                                                             |------>| Monero blockchain  |
                                                                             |       +--------------------+
                                                                             |       +--------------------+
                                                                             |------>|    Email server    |
                                                                             |       +--------------------+
                                                                             |       +--------------------+
                                                                             |------>|    RPC endpoint    |
                                                                             |       +--------------------+
                                                                             |       +--------------------+
                                                                             |------>|       Website      |
                                                                             |       +--------------------+
                                                                             |       +--------------------+
  +----------------------------------+                                       |------>|       etc...       |
  | Mixnet:                          |                                       |       +--------------------+
  |       * Gateway your client is   |                                       |
  |       connected to               |          +--------------------+       |
  |       * Mix nodes 1 -> 3         |<-------->| Network requester  |<------+
  |       * Gateway that network     |          +--------------------+
  |       requester is connected to  |
  +----------------------------------+
           ^
           |
           |
           |
           |
           v
 +-------------------+
 | +---------------+ |
 | |  Nym client   | |
 | +---------------+ |
 |         ^         |
 |         |         |
 |         |         |
 |         |         |
 |         v         |
 | +---------------+ |
 | | Your app code | |
 | +---------------+ |
 +-------------------+
  Your Local Machine
```

There are 2 pieces of software that work together to send SOCKS traffic through the mixnet: the `nym-socks5-client`, and the `nym-network-requester`.

The `nym-socks5-client` allows you to do the following from your local machine:
* Take a TCP data stream from a application that can send traffic via SOCKS5.
* Chop up the TCP stream into multiple Sphinx packets, assigning sequence numbers to them, while leaving the TCP connection open for more data
* Send the Sphinx packets through the mixnet to a [network requester](../nodes/network-requester-setup.md). Packets are shuffled and mixed as they transit the mixnet.

The `nym-network-requester` then reassembles the original TCP stream using the packets' sequence numbers, and make the intended request. It will then chop up the response into Sphinx packets and send them back through the mixnet to your  `nym-socks5-client`. The application will then receive its data, without even noticing that it wasn't talking to a "normal" SOCKS5 proxy!

## Client setup
### Viewing command help

You can check that your binaries are properly compiled with:

```
./nym-socks5-client --help
```

```admonish example collapsible=true title="Console output"


         _ __  _   _ _ __ ___
        | '_ \| | | | '_ \ _ \
        | | | | |_| | | | | | |
        |_| |_|\__, |_| |_| |_|
                |___/

                (socks5 proxy - version {{platform_release_version}})


        nym-socks5-client {{platform_release_version}}
        Nymtech
        A SOCKS5 localhost proxy that converts incoming messages to Sphinx and sends them to a Nym address

        USAGE:
        nym-socks5-client [OPTIONS] <SUBCOMMAND>

        OPTIONS:
                --config-env-file <CONFIG_ENV_FILE>
                Path pointing to an env file that configures the client

        -h, --help
                Print help information

        -V, --version
                Print version information

        SUBCOMMANDS:
        completions          Generate shell completions
        generate-fig-spec    Generate Fig specification
        help                 Print this message or the help of the given subcommand(s)
        init                 Initialise a Nym client. Do this first!
        run                  Run the Nym client with provided configuration client optionally
                             overriding set parameters
        upgrade              Try to upgrade the client


```

You can check the necessary parameters for the available commands by running:

```
./nym-client <command> --help
```

### Initialising a new client instance

Before you can use the client, you need to initalise a new instance of it, which can be done with the following command:

```
./nym-socks5-client init --id <id> --provider <provider>
```

The `--id` in the example above is a local identifier so that you can name your clients; it is **never** transmitted over the network.

The `--provider` field needs to be filled with the Nym address of a Network Requester that can make network requests on your behalf. If you don't want to [run your own](../nodes/network-requester-setup.md) you can select one from the [mixnet explorer](https://explorer.nymtech.net/network-components/service-providers) by copying its `Client ID` and using this as the value of the `--provider` flag. Alternatively, you could use [this list](https://harbourmaster.nymtech.net/).

Since the nodes on this list are the infrastructure for [Nymconnect](https://nymtech.net/developers/quickstart/nymconnect-gui.html) they will support all apps on the [default whitelist](../nodes/network-requester-setup.md#network-requester-whitelist): Keybase, Telegram, Electrum, Blockstream Green, and Helios.

#### Choosing a Gateway
By default - as in the example above - your client will choose a random gateway to connect to.

However, there are several options for choosing a gateway, if you do not want one that is randomly assigned to your client:
* If you wish to connect to a specific gateway, you can specify this with the `--gateway` flag when running `init`.
* You can also choose a gateway based on its location relative to your client. This can be done by appending the `--latency-based-selection` flag to your `init` command. This command means that to select a gateway, your client will:
	* fetch a list of all availiable gateways
	* send few ping messages to all of them, and measure response times.
	* create a weighted distribution to randomly choose one, favouring ones with lower latency.

> Note this doesn't mean that your client will pick the closest gateway to you, but it will be far more likely to connect to gateway with a 20ms ping rather than 200ms

### Running the socks5 client

You can run the initalised client by doing this:

```
./nym-socks5-client run --id <id>
```

~~~admonish example collapsible=true title="Console output"

```
2022-04-27T16:15:45.843Z INFO  nym_socks5_client::client > Starting nym client
2022-04-27T16:15:45.889Z INFO  nym_socks5_client::client > Obtaining initial network topology
2022-04-27T16:15:51.470Z INFO  nym_socks5_client::client > Starting topology refresher...
2022-04-27T16:15:51.470Z INFO  nym_socks5_client::client > Starting received messages buffer controller...
2022-04-27T16:15:51.648Z INFO  gateway_client::client    > Claiming more bandwidth for your tokens. This will use 1 token(s) from your wallet. Stop the process now if you don't want that to happen.
2022-04-27T16:15:51.648Z WARN  gateway_client::client    > Not enough bandwidth. Trying to get more bandwidth, this might take a while
2022-04-27T16:15:51.648Z INFO  gateway_client::client    > The client is running in disabled credentials mode - attempting to claim bandwidth without a credential
2022-04-27T16:15:51.706Z INFO  nym_socks5_client::client > Starting mix traffic controller...
2022-04-27T16:15:51.706Z INFO  nym_socks5_client::client > Starting real traffic stream...
2022-04-27T16:15:51.706Z INFO  nym_socks5_client::client > Starting loop cover traffic stream...
2022-04-27T16:15:51.707Z INFO  nym_socks5_client::client > Starting socks5 listener...
2022-04-27T16:15:51.707Z INFO  nym_socks5_client::socks::server > Listening on 127.0.0.1:1080
2022-04-27T16:15:51.707Z INFO  nym_socks5_client::client> Client startup finished!
2022-04-27T16:15:51.707Z INFO  nym_socks5_client::client> The address of this client is: BFKhbyNsSVwbsGSLwHDkfwH5mwZqZYpnpNjjV7Xo25Xc.EFWd1geWspzyVeinwXrY5fCBMRtAKV1QmK1CNFhAA8VG@BNjYZPxzcJwczXHHgBxCAyVJKxN6LPteDRrKapxWmexv
2022-04-27T16:15:51.707Z INFO  nym_socks5_client::socks::server > Serving Connections...
```
~~~

## Using your Socks5 Client

After completing the steps above, your local Socks5 Client will be listening on `localhost:1080` ready to proxy traffic to the Network Requester set as the `--provider` when initialising.

When trying to connect your app, generally the proxy settings are found in `settings->advanced` or `settings->connection`.

Here is an example of setting the proxy connecting in Blockstream Green:

![Blockstream Green settings](../images/wallet-proxy-settings/blockstream-green.gif)

Most wallets and other applications will work basically the same way: find the network proxy settings, enter the proxy url (host: **localhost**, port: **1080**).

In some other applications, this might be written as **localhost:1080** if there's only one proxy entry field.
