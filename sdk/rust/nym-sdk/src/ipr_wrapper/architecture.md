# IpMixStream Architecture

## Overview

`IpMixStream` tunnels IP packets through the Nym mixnet to an IP Packet Router
(IPR) exit gateway. It provides a high-level API over a single `MixnetStream`,
which handles LP Stream framing and Sphinx packet transport automatically.

## Data Flow

```text
Client                              IPR (exit gateway)
------                              ------------------
IpMixStream.send_ip_packet(bytes)
  IpPacketRequest.to_bytes()
    MixnetStream.write()
      LP Stream frame
        Sphinx packets
          mixnet ──────────────────> on_reconstructed_message()
                                      detect LpFrameKind::Stream
                                        strip LP header
                                          parse IpPacketRequest
                                            write IP packet to TUN
                                              ──> internet

                                    internet response arrives on TUN
                                      ConnectedClientHandler
                                        wrap in IpPacketResponse
                                        wrap in LP Stream frame
          mixnet <──────────────────      send via Sphinx/SURBs

      stream router dispatches
        by stream_id
    MixnetStream.read()
  IprListener parses response
IpMixStream.handle_incoming()
  returns Vec<ip_packet_bytes>
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

## Connection Lifecycle

1. `IpMixStream::new(env)` -- discover IPR, connect MixnetClient, open MixnetStream
2. `connect_tunnel()` -- send connect request, receive allocated IPs
3. `send_ip_packet()` / `handle_incoming()` -- steady-state data transfer
4. `disconnect_stream()` -- tear down MixnetClient

## Key Design Decisions

- **MixnetStream over MixnetClient**: One stream per IPR tunnel. LP framing is
  handled by MixnetStream internally, no manual frame construction needed.

- **Multiplexing at IP layer**: Different remote hosts are addressed by IP
  packet destination headers, not by opening multiple streams.

- **stream_id threading**: The IPR stores stream_id in each client's
  `ConnectedClientHandler` so async TUN responses are wrapped in matching LP
  Stream frames for correct dispatch at the client.
