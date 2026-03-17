# IpMixStream Architecture

## Overview

`IpMixStream` tunnels IP packets through the Nym mixnet to an IP Packet Router
(IPR) exit gateway. It provides a high-level API over a single `MixnetStream`,
which handles LP Stream framing and Sphinx packet transport automatically.

## Data Flow

TODO

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

## Connection Lifecycle

TODO
