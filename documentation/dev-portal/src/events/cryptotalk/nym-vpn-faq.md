# Frequently Asked Questions

This is a FAQ page tailored for this event. If you interested to read more about Nym platform, you can have a look at [Nym general FAQ](https://nymtech.net/developers/faq/general-faq.html) and read through Nym's technical [documentation](https://nymtech.net/docs), [Developer Portal](https://nymtech.net/developers) and [Operators Guide](https://nymtech.net/operators).

## NymVPN

If this your first time hearing about NymVPN, make sure you read [NymVPN webpage](https://nymvpn.com/en), the official [support & FAQ page](https://nymvpn.com/en/support) and our guide on how to [install, run and test](./nym-vpn.md) the client.

Below are some extra FAQs which came out during the event.

### What's the difference between 2-hops and 5-hops

The default is 5-hops (including Entry and Exit Gateways), which means that the traffic goes from the local client to Entry Gateway -> through 3 layers of Mix Nodes -> to Exit Gateway -> internet. this option uses all the Nym Mixnet features for maximum privacy.

```
                      ┌─►mix──┐  mix     mix
                      │       │
            Entry     │       │                   Exit
client ───► Gateway ──┘  mix  │  mix  ┌─►mix ───► Gateway ───► internet
                              │       │
                              │       │
                         mix  └─►mix──┘  mix
```

The 2-hop option is going from the local client -> Entry Gateway -> directly to Exit Gateway -> internet. This option is good for operations demanding faster connection. Keep in mind that this setup by-passes the 3 layers of Mix Nodes. The anonymising features done by your local client like breaking data into same-size packets with inserting additional "dummy" ones to break the time and volume patterns is done in both options.

```
            Entry         Exit
client ───► Gateway ────► Gateway ───► internet
```

We highly recommend to read more about [Nym network overview](https://nymtech.net/docs/architecture/network-overview.html) and the [Mixnet traffic flow](https://nymtech.net/docs/architecture/traffic-flow.html).

### Why do I see different sizes of packets in my terminal log?

One of features of Nym Mixnet's clients is to break data into the same size packets called Sphinx, which is currently ~2kb. When running NymVPN, the data log shows payload sizes, which are the raw sizes of the IP packets, not Sphinx. The payload sizes will be capped by the configured MTU, which is set around 1500 bytes.

### What is 'poisson filter' about?

By default `--enable-poisson` is disabled and packets are sent from the local client to the Entry Gateway as quickly as possible. With the poisson process enabled the Nym client will send packets at a steady stream to the Entry Gateway. By default it's on average one sphinx packet per 20ms, but there is some randomness (poisson distribution). When there are no real data to fill the sphinx packets with, cover packets are generated instead.

Enabling the poisson filter is one of the key mechanisms to de-correlate input and output traffic to the Mixnet. The performance impact however is dramatic:
1 packer per 20ms is 50 packets / sec so ballpark 100kb/s.
For mobile clients that means constantly sending data eating up data allowance.


## Nym Mixnet Architecture and Rewards

We have a list of questions related to Nym Nodes and the incentives behind running them under [FAQ pages](https://nymtech.net/operators/faq/mixnodes-faq.html) in our [Operators Guide](https://nymtech.net/operators). For better knowledge about Nym architecture we recommend to read [Nym network overview](https://nymtech.net/docs/architecture/network-overview.html) and the [Mixnet traffic flow](https://nymtech.net/docs/architecture/traffic-flow.html) in our [technical documentation](https://nymtech.net/docs).

## Project Smoosh

Project Smoosh is a code name for a process in which different components of Nym Mixnet architecture get *smooshed* into one binary. Check out [Smoosh FAQ](https://nymtech.net/operators/faq/smoosh-faq.html) in Operators Guide to read more.

## Exit Gateway

Part of the the transition under code name [Project Smoosh](./nym-vpn-faq.md#project-smoosh) is a creation of [Nym Exit Gateway](https://nymtech.net/operators/legal/exit-gateway.html) functionality. The operators running Gateways would have to “open” their nodes to a wider range of online services, in a similar fashion to Tor exit relays. The main change will be to expand the original short [allowed.list](https://nymtech.net/.wellknown/network-requester/standard-allowed-list.txt) to a more permissive setup. An [exit policy](https://nymtech.net/.wellknown/network-requester/exit-policy.txt) will constrain the hosts that the users of the Nym VPN and Mixnet can connect to. This will be done in an effort to protect the operators, as Gateways will act both as SOCKS5 Network Requesters, and exit nodes for IP traffic from Nym VPN and Mixnet clients.

* Read more how the exit policy gets implemented [here](https://nymtech.net/operators/faq/smoosh-faq.html#how-will-the-exit-policy-be-implemented)
* Check out [Nym Operators Legal Forum](https://nymtech.net/operators/legal/exit-gateway.html)
* Do reach out to us during 37c3 with any experiences you may have running Tor Exit relays or legal findings and suggestions for Nym Exit Gateway operators

## Nym Integrations and SDKs

If you are a dev who is interested to integrate Nym, have a look on our SDK tutorials:

* [Rust SDKs](https://nymtech.net/developers/tutorials/cosmos-service/intro.html)
* [TypeScript SDKs](https://sdk.nymtech.net/)
* [Integration FAQ](https://nymtech.net/developers/faq/integrations-faq.html)
