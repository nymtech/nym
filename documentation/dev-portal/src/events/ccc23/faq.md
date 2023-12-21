# Frequently Asked Questions

Besides the FAQ for CCC 2023 event listed below, you can have a look at [Nym general FAQ](https://nymtech.net/developers/faq/general-faq.html) and read through Nym's technical [documentation](https://nymtech.net/docs), [Developer Portal](https://nymtech.net/developers) and [Operators Guide](https://nymtech.net/operators).

## NymVPN

Make sure you read [NymVPV webpage](https://nymvpn.com/en) and our [guide to install, run and test](./nym-vpn.md) the client. 

### Why do I see different sizes of packets in my terminal log?

One of features of Nym Mixnet's clients is to break data into the same size packets called Sphinx, which is currently ~2kb. When running NymVPN, the data log shows payload sizes, which are the raw sizes of the IP packets, not Sphinx. The payload sizes will be capped by the configured MTU, which is set around 1500 bytes.

## Nym Mixnet Architecture and Rewards

We have a list of questions related to Nym Nodes and the incentives behind running them under [FAQ pages](https://nymtech.net/operators/faq/mixnodes-faq.html) in our [Operators Guide](https://nymtech.net/operators).

## Project Smoosh

Project Smoosh is a code name for a process in which different components of Nym Mixnet architecture get *smooshed* into one binary. Check out [Smoosh FAQ](https://nymtech.net/operators/faq/smoosh-faq.html) in Operators Guide to read more. 

## Exit Gateway

Part of the the transition under code name [Project Smoosh](./faq.md#project-smoosh) is a creation of [Nym Exit Gateway](https://nymtech.net/operators/legal/exit-gateway.html) functionality. The operators running Gateways would have to “open” their nodes to a wider range of online services, in a similar fashion to Tor exit relays. The main change will be to expand the original short [allowed.list](https://nymtech.net/.wellknown/network-requester/standard-allowed-list.txt) to a more permissive setup. An [exit policy](https://nymtech.net/.wellknown/network-requester/exit-policy.txt) will constrain the hosts that the users of the Nym VPN and Mixnet can connect to. This will be done in an effort to protect the operators, as Gateways will act both as SOCKS5 Network Requesters, and exit nodes for IP traffic from Nym VPN and Mixnet clients.

* Read more how the exit policy gets implemented [here](https://nymtech.net/operators/faq/smoosh-faq.html#how-will-the-exit-policy-be-implemented)
* Check out [Nym Operators Legal Forum](https://nymtech.net/operators/legal/exit-gateway.html)
* Do reach out to us during CCC 2023 with any experiences you may have running Tor Exit relays or legal findings and suggestions for Nym Exit Gateway operators 

## Nym Integrations and SDKs

If you are a dev who is interested to integrate Nym, have a look on our SDK tutorials:

* [Rust SDKs](https://nymtech.net/developers/tutorials/cosmos-service/intro.html)
* [TypeScript SDKs](https://sdk.nymtech.net/)
* [Integration FAQ](https://nymtech.net/developers/faq/integrations-faq.html)



<!--

Reach out to Marc and Romain about gathered FAQ

-->
