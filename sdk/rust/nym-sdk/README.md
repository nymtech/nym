# Nym Rust SDK

This repo contains several components:
- `mixnet`: exposes Nym Client builders and methods. This is useful if you want to interact directly with the Client, or build transport abstractions.
- `tcp_proxy`: exposes functionality to set up client/server instances that expose a localhost TcpSocket to read/write to like a 'normal' socket connection. `tcp_proxy/bin/` contains standalone `nym-proxy-client` and `nym-proxy-server` binaries. *Note: this module is being superceded by the `stream_wrapper` module, and this module will not have features added to it in the future.*
- `clientpool`: a configurable pool of ephemeral Nym Clients which can be created as a background process and quickly grabbed.
- `stream_wrapper`: made up of two parts: a TCP-Socket-like abstraction (`mixnet_stream_wrapper.rs`) for a Nym Client, and an abstraction built on top of this (`mixnet_stream_wrapper_ipr`) which allows for client-side integrations to send IP packets through Exit Gateways' IpPacketRouter, and use the Mixnet as a proxy. For an example of where this is used, see the `mixtcp` crate.

Documentation can be found [here](https://nym.com/docs/developers/rust).
