# SOCKS Proxy (CLI)

> `nym-socks5-client` now also supports SOCKS4 and SOCKS4A protocols as well as SOCKS5.

The Nym socks5 client allows you to proxy traffic from a desktop application through the mixnet, meaning you can send and receive information from remote application servers without leaking metadata which can be used to deanonymise you, even if you're using an encrypted application such as Signal.

## Setup and Run

### Download or compile socks5 client

If you are using OSX or a Debian-based operating system, you can download the `nym-socks5-client` binary from our [Github releases page](https://github.com/nymtech/nym/releases).

If you are using a different operating system, head over to the [Building from Source](https://nymtech.net/docs/binaries/building-nym.html) page for instructions on how to build the repository from source.

### Initialise your socks5 client

To initialise your `nym-socks5-client` you need to have an address of a Network Requester (NR). Nowadays NR is part of every Exit Gateway (`nym-node --mode exit-gateway`). The easiest way to get a NR address is to visit [harbourmaster.nymtech.net](https://harbourmaster.nymtech.net/) and scroll down to *SOCKS5 Network Requesters* table. There you can filter the NR by Gateways identity address, and other options.

Use the following command to initialise `nym-socs5-client` where `<ID>` can be anything you want (it's only for local config file storage) and `<PROVIDER>` is suplemented with a NR address:

```
./nym-socks5-client init --id <ID> --provider
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

### Start your socks5 client
Now your client is initialised, start it with the following:

```
./nym-socks5-client run --id <ID>
```

## Proxying traffic
After completing the steps above, your local `nym-socks5-client` will be listening on `localhost:1080` ready to proxy traffic to the Network Requester set as the `--provider` when initialising.

When trying to connect your app, generally the proxy settings are found in `settings->advanced` or `settings->connection`.

Here is an example of setting the proxy connecting in Blockstream Green:

![Blockstream Green settings](../images/blockstream-green.gif)

Most wallets and other applications will work basically the same way: find the network proxy settings, enter the proxy url (host: **localhost**, port: **1080**).

In some other applications, this might be written as **localhost:1080** if there's only one proxy entry field.

## Supported Applications

Any application which can be redirected over Socks5 proxy should work. Nym community has been successfully running over Nym Mixnet these applications:

- Bitcoin Electrum wallet
- Monero wallet (GUI and CLI with monerod)
- Telegram chat
- Element/Matrix chat
- Firo wallet
- ircd chat
- Blockstream Green

Keep in mind that Nym has been developing a new client **[NymVPN](https://nymvpn.com) (GUI and CLI) routing all users traffic through the Mixnet.**

## Further reading

If you want to dig more into the architecture and use of the socks5 client check out its documentation [here](https://nymtech.net/docs/clients/socks5-client.html).
