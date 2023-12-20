# Frequently Asked Questions

Besides the FAQ for CCC 2023 event listed below, you can have a look at [Nym general FAQ](https://nymtech.net/developers/faq/general-faq.html) and read through Nym's technical [documentation](https://nymtech.net/docs), [Developer Portal](https://nymtech.net/developers) and [Operators Guide](https://nymtech.net/operators).

## Nym Mixnet Architecture and Rewards

We have a list of questions related to Nym Nodes and the incentives behind running them under [FAQ pages](https://nymtech.net/operators/faq/mixnodes-faq.html) in our [Operators Guide](https://nymtech.net/operators).

## Project Smoosh

Project Smoosh is a code name for a process in which different components of Nym Mixnet architecture get *smooshed* into one binary. 

Project Smoosh will have four steps, please follow the table below to track the dynamic progress:

| **Step** | **Status** |
| :--- | :--- |
| **1.** Combine the `nym-gateway` and `nym-network-requester` into one binary | ✅ done |
| **2.** Create [Exit Gateway](https://nymtech.net/operators/legal/exit-gateway.html): Take the `nym-gateway` binary including `nym-network-requester` combined in \#1 and switch from [`allowed.list`](https://nymtech.net/.wellknown/network-requester/standard-allowed-list.txt) to a new [exit policy](https://nymtech.net/.wellknown/network-requester/exit-policy.txt) | ✅ done |
| **3.** Combine all the nodes in the Nym Mixnet into one binary, that is `nym-mixnode`, `nym-gateway` (entry and exit) and `nym-network-requester`. | 🛠️ in progress |
| **4.** Adjust reward scheme to incentivise and reward Exit Gateways as a part of `nym-node` binary, implementing [zkNym credentials](https://youtu.be/nLmdsZ1BsQg?t=1717). | 🛠️ in progress |

These steps will be staggered over time - period of several months, and will be implemented one by one with enough time to take in feedback and fix bugs in between.  
Generally, the software will be the same, just instead of multiple binaries, there will be one Nym Node (`nym-node`) binary. Delegations will remain on as they are now, per our token economics (staking, saturation etc)

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

## NymVPN

Make sure you read [NymVPV webpage](https://nymvpn.com/en) and our [guide to install, run and test](./nym-vpn.md) the client. 

<!--

Reach out to Marc and Romain about gathered FAQ

-->
