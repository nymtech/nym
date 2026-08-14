# Service Providers

This directory contains the **gateway-internal service providers**:
infrastructure services that run embedded inside
[`nym-node`](../nym-node) (on exit gateways) and give mixnet clients a way
out to the regular internet.

## ip-packet-router

Routes IP packets tunneled through the mixnet. Clients wrap IP packets in
mixnet messages; the ip-packet-router unwraps them on the exit gateway and
forwards them to the public internet, returning the responses the same way.
This is the exit-side component used by NymVPN's mixnet mode.

## network-requester

Forwards application-level network requests (e.g. SOCKS5 traffic from
`nym-socks5-client`) made over the mixnet to public destinations permitted
by the [exit policy](../common/exit-policy), and relays the responses back.
This lets applications reach ordinary internet services while the network
observer — and the service being contacted — learns nothing about who is
connecting.

Both are wired into `nym-node` and enabled by node operators running an exit
gateway; they use the shared plumbing in [`common`](common).

## Building your own service provider

You don't need to run a node — or touch anything in this directory — to
offer a service over the mixnet. A service provider is simply a process
with a mixnet client: it has a nym address, listens for requests arriving
through the mixnet, and replies anonymously via SURBs. Anyone can build one
with the Rust SDK.

See the runnable echo service example and its developer guide:
[`sdk/rust/nym-sdk/examples/service-providers`](../sdk/rust/nym-sdk/examples/service-providers/README.md),
and the [Nym whitepaper](https://nym.com/nym-whitepaper.pdf) (§3.1) for what
service providers are designed for.
