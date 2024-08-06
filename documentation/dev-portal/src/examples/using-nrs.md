# Apps Using Network Requesters
These applications utilise custom app logic in the user-facing apps in order to communicate using the mixnet as a transport layer, without having to rely on custom server-side logic. Instead, they utilise existing Nym infrastructure - Network Requesters - now embeded in `nym-node` running as `exit-gateway`.

If you are sending 'normal' application traffic, and/or don't require and custom logic to be happening on the 'other side' of the mixnet, this is most likely the best option to take as a developer who wishes to privacy-enhance their application.

> Nym will soon be switching from a whitelist-based approach to a blocklist-based approach to filtering traffic. As such, it will soon be even easier for developers to utilise the mixnet, as they will not have to run their own NRs or have to add their domains to the whitelist

- DarkFi over Nym leverages Nym’s mixnet as a pluggable transport for DarkIRC, their p2p IRC variant. Users can anonymously connect to peers over the network, ensuring secure and private communication within the DarkFi ecosystem. Written in **Rust**.
  - [Docs](https://darkrenaissance.github.io/darkfi/clients/nym_outbound.html?highlight=nym#3--run)
  - [Github](https://github.com/darkrenaissance/darkfi/tree/master/doc)

- MiniBolt is a complete guide to building a Bitcoin & Lightning full node on a personal computer. It has the capacity to run network traffic (transactions and syncing) over the mixnet, so you can privately sync your node and not expose your home IP to the wider world when interacting with the rest of the network!
  - [Docs](https://v2.minibolt.info/bonus-guides/system/nym-mixnet#proxying-bitcoin-core)
  - [Codebase](https://github.com/minibolt-guide/minibolt)

- Email over Nym is a set of configuration options to set up a Network Requester to send and recieve emails over Nym, using something like Thunderbird.
  - [Codebase](https://github.com/dial0ut/nymstr-email)
