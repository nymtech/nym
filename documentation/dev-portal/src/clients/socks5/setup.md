# Setup

> `nym-socks5-client` now also supports SOCKS4 and SOCKS4A protocols as well as SOCKS5.

The Nym socks5 client allows you to proxy traffic from a desktop application through the mixnet, meaning you can send and receive information from remote application servers without leaking metadata which can be used to deanonymise you, even if you're using an encrypted application such as Signal.

```admonish info
Since the beginning of 2024 NymConnect is no longer maintained. Nym is developing a new client called [NymVPN](https://nymvpn.com), an application routing all users traffic thorugh the mixnet.
If users want to route their traffic through socks5 we advice to use this client. If you want to run deprecated NymConnect, visit [NymConnect archive page](../../archive/nym-connect.md) with setup and application examples.
```

## Setup and Run

### Download or compile socks5 client

If you are using OSX or a Debian-based operating system, you can download the `nym-socks5-client` binary from our [Github releases page](https://github.com/nymtech/nym/releases).

If you are using a different operating system, head over to the [Building from Source](https://nymtech.net/docs/binaries/building-nym.html) page for instructions on how to build the repository from source.

### Initialise your socks5 client

To initialise your `nym-socks5-client` you need to have an address of a Network Requester (NR). Nowadays NR is part of every Exit Gateway (`nym-node --mode exit-gateway`). The easiest way to get a NR address is to visit [harbourmaster.nymtech.net](https://harbourmaster.nymtech.net/) and scroll down to *SOCKS5 Network Requesters* table. There you can filter the NR by Gateways identity address, and other options.

Use the following command to initialise `nym-socs5-client` where `<ID>` can be anything you want (it's only for local config file storage) and `<PROVIDER>` is suplemented with a NR address:

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

### Start your socks5 client
Now your client is initialised, start it with the following:

```
./nym-socks5-client run --id <ID>
```
