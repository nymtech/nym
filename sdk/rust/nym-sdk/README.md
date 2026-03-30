# Nym Rust SDK

This repo contains several components:
- `mixnet`: exposes Nym Client builders and methods. This is useful if you want to interact directly with the Client, or build transport abstractions. The `mixnet::stream` submodule provides `MixnetStream` — a TCP-socket-like abstraction over the mixnet with LP Stream framing, sequence numbering, and `AsyncRead`/`AsyncWrite` support.
- `ipr_wrapper`: provides `IpMixStream`, a high-level interface for tunnelling IP packets through Exit Gateways' IpPacketRouter (IPR) over the mixnet. Built on `MixnetStream`, it handles gateway discovery, connect handshakes, and IP packet send/receive. See the [`smolmix`](../../smolmix) crate for a full tunnel implementation using this module.
- `tcp_proxy`: exposes functionality to set up client/server instances that expose a localhost TcpSocket to read/write to like a 'normal' socket connection. `tcp_proxy/bin/` contains standalone `nym-proxy-client` and `nym-proxy-server` binaries.
- `clientpool`: a configurable pool of ephemeral Nym Clients which can be created as a background process and quickly grabbed.

Documentation can be found [here](https://nym.com/docs/developers/rust).
