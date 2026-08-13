# Nym Service Provider Examples

This directory contains runnable examples of a **Nym service provider**: a
service that is reachable anonymously through the Nym mixnet.

## What is a Nym service provider?

From the [Nym whitepaper](https://nym.com/nym-whitepaper.pdf) (§3.1):

> The Nym network is not a standalone service; it is an infrastructure that
> supports privacy for a broad range of third-party applications and services
> accessible through it. Service providers can send and receive messages
> through the Nym network to privately communicate with their users.

Concretely, a service provider is any process that runs a mixnet client:

- it has a **nym address** that clients can send messages to,
- it **listens** for incoming messages arriving through the mixnet,
- it **replies via SURBs** (Single Use Reply Blocks, whitepaper §4.5) —
  pre-built reply routes bundled with each request — so it can answer
  without ever learning who asked.

Because requests traverse the mixnet, the service cannot see its users' IP
addresses or nym addresses, and a global network observer cannot link users
to the services they access (whitepaper §3.4–3.5). Service providers can also
act as bridges between the mixnet and external systems that know nothing
about Nym — the whitepaper's example is a provider that relays Bitcoin
transactions received over the mixnet to the Bitcoin peer-to-peer network.

## The echo example

Two examples demonstrate the pattern end to end:

- [`echo-service`](echo-service/main.rs) — a long-running service provider.
  For every request it receives, it replies (via SURBs) with a JSON payload:

  ```json
  {
    "message": "hello",
    "timestamp_utc": "2026-08-13T12:34:56.789012+00:00",
    "request_id": "3f2b6d9c-5a1e-4c7b-9e0d-8f4a2b6c1d5e"
  }
  ```

- [`echo-client`](echo-client/main.rs) — sends one request to a running
  echo service, prints the parsed reply, and exits.

```
 echo-client                mixnet                  echo-service
   │                          │                          │
   │──request + SURBs────────▶│  Sphinx packets,         │
   │   (bundled by default)   │  3 mix hops              │
   │                          │─────────────────────────▶│
   │                          │           sees only an opaque sender tag
   │                          │                          │
   │                          │◀──send_reply(tag, json)──│
   │◀─────────────────────────│                          │
   │                          │                          │
   │   cover traffic loops run continuously on both sides│
```

### Running it

Requires network access; connecting to the mixnet takes a few seconds.

Terminal 1 — start the service and note the address it prints:

```sh
cargo run --example echo-service
# echo-service listening on: DguTcdkWWtDyUFLvQxRdcA8qFhuWiV8UuHfLXe9xpSQq.gy...@62F81C9GrHDRja9WCqozemRFSzFPMecY85MbGwn6efve
```

Terminal 2 — send a request to that address:

```sh
cargo run --example echo-client -- <address-printed-by-the-service>
```

The client prints the JSON reply and exits. The service keeps running and
answers any number of requests — from any number of clients — without ever
learning who they are. Keys are ephemeral in both examples, so the service
gets a fresh address each run.

## How to implement your own service provider

The whole pattern is ~30 lines. Follow along in
[`echo-service/main.rs`](echo-service/main.rs):

1. **Build and connect a client.**

   ```rust
   let client = MixnetClientBuilder::new_ephemeral()
       .debug_config(privacy_config())
       .build()?;
   let mut client = client.connect_to_mixnet().await?;
   println!("listening on: {}", client.nym_address());
   ```

2. **Loop over incoming messages.** Skip empty ones — they are SURB
   replenishment requests the SDK handles for you.

3. **Reply with `send_reply()`.** Every incoming message carries an optional
   `sender_tag: AnonymousSenderTag` — an opaque token pointing at the SURBs
   the requester bundled. It is the only thing you ever learn about them,
   and it is all you need to answer:

   ```rust
   client.send_reply(sender_tag, response_bytes).await?;
   ```

   Each SURB is single-use; the SDK requests more from the client
   automatically when a conversation runs long.

Your protocol is whatever you put in the message bytes — the echo example
uses JSON.

### Privacy configuration

Both examples construct their [`DebugConfig`] explicitly to make the mixnet's
privacy features visible (they are all **on by default**):

- **Timing obfuscation** — real messages leave the client at randomized
  Poisson intervals (`traffic.message_sending_average_delay`), with dummy
  packets filling the gaps, and every packet is independently delayed at
  each mix hop (`traffic.average_packet_delay`).
- **Cover traffic** — the client continuously sends loop packets to itself
  (`cover_traffic.loop_cover_traffic_average_delay`), so an observer cannot
  even tell *whether* it is communicating (unobservability, whitepaper §4.6).

`DebugConfig` also exposes `set_no_cover_traffic()` and
`set_no_poisson_process()`. These exist for debugging and testing — flipping
them off in production removes exactly the protections that make the mixnet
more than an encrypted proxy.

### Credentials: free mode

The examples run in **free mode**: they never call
`enable_credentials_mode()`, so no zk-nym credentials are acquired or
presented. The network does not currently enforce zk-nyms for mixnet mode,
so anyone can run these examples on mainnet without holding NYM tokens. For
the credentialed bandwidth flow, see the [`bandwidth`](../bandwidth.rs)
example.

### Going to production

- **Persist your keys.** Ephemeral clients get a new nym address every run;
  a real service's address is its identity. Use persistent storage as shown
  in [`builder_with_storage`](../builder_with_storage.rs).
- **Don't confuse this with gateway-internal service providers.** The
  [`/service-providers`](../../../../../service-providers/README.md)
  directory in this repository contains infrastructure services
  (`ip-packet-router`, `network-requester`) that run embedded inside
  `nym-node`. An SDK-level service provider like this one is a standalone
  process — no node operation or registration required.

## Further reading

- [The Nym whitepaper](https://nym.com/nym-whitepaper.pdf) — §3.1 (service
  providers), §4.5 (SURBs), §4.6 (cover traffic)
- [`surb_reply`](../surb_reply.rs) — the minimal SURB reply mechanism in
  isolation
