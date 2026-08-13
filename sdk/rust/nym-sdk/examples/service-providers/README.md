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
   │──open_stream + SURBs────▶│  Sphinx packets,         │
   │   write("echo request")  │  3 mix hops              │
   │                          │─────────────────────────▶│ listener.accept()
   │                          │        sees only an anonymous stream
   │                          │                          │
   │                          │◀──stream.write(json)─────│ reply rides the
   │◀─────────────────────────│                          │ client's SURBs
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

The shape is the same as a classic socket server. Follow along in
[`echo-service/main.rs`](echo-service/main.rs):

1. **Build and connect a client, then activate stream mode.**

   ```rust
   let client = MixnetClientBuilder::new_ephemeral()
       .debug_config(privacy_config())
       .build()?;
   let mut client = client.connect_to_mixnet().await?;
   println!("listening on: {}", client.nym_address());

   let mut listener = client.listener()?;
   ```

2. **Accept incoming streams, one task per client.** `MixnetStream`
   implements `AsyncRead + AsyncWrite` — the same traits as a TCP socket —
   so standard tokio I/O works unchanged:

   ```rust
   while let Some(stream) = listener.accept().await {
       tokio::spawn(handle_stream(stream));
   }
   ```

3. **Read the request, write the reply.** Writes on an accepted stream are
   routed through the SURBs (Single Use Reply Blocks) the remote peer
   attached when opening the stream — pre-built anonymous reply routes.
   The service never learns the peer's address; the SDK replenishes SURBs
   automatically when a conversation runs long:

   ```rust
   let n = stream.read(&mut buf).await?;
   stream.write_all(&response_bytes).await?;
   stream.flush().await?;
   ```

Your protocol is whatever you put in the stream bytes — the echo example
uses JSON. If you want the lower-level message API instead of streams
(`wait_for_messages` + `send_reply()` on a raw `sender_tag`), see
[`surb_reply`](../surb_reply.rs) — streams use exactly that machinery under
the hood.

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

## Real world performance of mixnet mode on the Nym network

> **Warning:** the sending rate of a service provider using the default
> Poisson process caps it at roughly **55 packets per second**: the main
> packet stream releases one packet every 20 ms on average
> (`message_sending_average_delay`) and the loop cover stream one every
> 200 ms (`loop_cover_traffic_average_delay`) — see the defaults in
> [`common/client-core/config-types`](../../../../../common/client-core/config-types/src/lib.rs).
> Each Sphinx packet carries ~2 kB of payload, so real throughput tops out
> at **~100 kB/s** — and that is the ceiling, shared across everything the
> service sends.

This is by design: a constant, data-independent packet rate is exactly what
makes traffic analysis fail. But it has real operational consequences:

- **It limits how many clients you can serve.** Every reply to every client
  comes out of the same ~50 real packets/s budget. A service answering with
  2 kB JSON payloads can sustain at most a few dozen requests per second in
  total, not per client — plan capacity (or shard across multiple service
  provider instances) accordingly.
- **Cover traffic is expensive.** When the service has nothing to send, it
  sends anyway — cover packets fill every gap in the Poisson schedule. You
  pay the full ~110 kB/s of Sphinx traffic (packets in both directions)
  around the clock, whether you have one user or none.
- **The client never stops.** Packet generation, Sphinx encryption, and the
  gateway connection run continuously, so your CPU keeps spinning and your
  gateway keeps forwarding 24/7 — idle looks exactly like busy, on your
  machine and on the wire. That's the unobservability property doing its
  job.
- **You can trade privacy for throughput, but understand the cost.**
  `DebugConfig` lets you raise the sending rate, disable the Poisson
  process (`set_no_poisson_process()`), or turn off cover traffic
  (`set_no_cover_traffic()`). Every one of those steps makes your service's
  traffic more correlatable — the whitepaper's §4.8 discusses this
  latency/bandwidth/privacy trade-off. If you turn them all off you have
  encrypted multi-hop routing, but no longer meaningful protection against
  the global observer the mixnet is designed to defeat.

## Further reading

- [The Nym whitepaper](https://nym.com/nym-whitepaper.pdf) — §3.1 (service
  providers), §4.5 (SURBs), §4.6 (cover traffic)
- [`stream_simple_read_write`](../stream_simple_read_write.rs) — concurrent
  streams over the mixnet in more detail
- [`surb_reply`](../surb_reply.rs) — the minimal SURB reply mechanism in
  isolation
