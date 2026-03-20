# IpMixStream Architecture

## Overview

`IpMixStream` tunnels IP packets through the Nym mixnet to an IP Packet Router
(IPR) exit gateway. It provides a high-level API over a single `MixnetStream`,
which handles LP Stream framing and Sphinx packet transport automatically.

## Data Flow

```text
Client                                              IPR
  |                                                  |
  |-- IpPacketRequest (connect) ---- LP Stream ----->|
  |<--- IpPacketResponse (ips) ---- LP Stream (s=0) -|
  |                                                  |
  |-- IpPacketRequest (data) ------- LP Stream ----->| -> TUN -> internet
  |<--- IpPacketResponse (data) --- LP Stream (s=1+) | <- TUN <- internet
```

## Layer Stack

```text
IpMixStream          IPR protocol (connect, data, disconnect)
MixnetStream         AsyncRead + AsyncWrite, LP Stream framing, seq numbers
Stream Router        Dispatches inbound messages by stream_id
MixnetClient         Sphinx packet encryption, SURB management
Mixnet               Entry GW -> Mix1 -> Mix2 -> Mix3 -> Exit GW
```

## LP Stream Framing

All messages between client and IPR are wrapped in LP Stream frames:

- **Client -> IPR**: `MixnetStream.write()` wraps each write in an LP Stream
  Data frame (stream_id, sequence number, payload). The IPR detects
  `LpFrameKind::Stream` and strips the header before processing.

- **IPR -> Client**: Both inline responses (connect handshake, pong) and async
  TUN responses are wrapped in LP Stream frames with the same stream_id. The
  client's stream router dispatches by stream_id to the correct `MixnetStream`.

## IP Allocation

The IPR owns a subnet (`10.0.0.0/16` for IPv4, `fc00::/64` for IPv6) and
allocates addresses to clients on connect. Two modes are supported:

- **Dynamic** (used by `IpMixStream`): The IPR picks a random unused `IpPair`
  from the pool (up to 100 retries). `IpMixStream` uses this path.
- **Static**: The client requests specific IPs; the IPR checks availability.

On dynamic connect:

1. IPR calls `find_new_ip()` — random selection from the subnet
2. Registers the client (keyed by anonymous sender tag for v8, Nym address
   for v6/v7) with that IP pair in dual `HashMap<Ipv4Addr/Ipv6Addr, Client>`
3. Returns `DynamicConnectSuccess { ips }` to the client

The client uses the allocated IPv4/IPv6 as its source address when constructing
packets. When the TUN device receives a response destined for that IP, the IPR
looks up which client owns it and forwards the packet back through the mixnet.

Reserved addresses: `10.0.0.1` / `fc00::1` (TUN gateway, never allocated).

## Connection Lifecycle

1. `IpMixStream::new()` discovers the best IPR via Nym API
2. Opens a `MixnetStream` to the IPR (`client.open_stream()`)
3. Sends a dynamic connect request, IPR allocates an `IpPair`
4. Ready for `send_ip_packet()` / `handle_incoming()` loop
5. `disconnect()` shuts down the mixnet client
