# Nym Rust SDK

This repo contains several components:
- `mixnet`: exposes Nym Client builders and methods. This is useful if you want to interact directly with the Client, or build transport abstractions.
- `tcp_proxy`: exposes functionality to set up client/server instances that expose a localhost TcpSocket to read/write to like a 'normal' socket connection. `tcp_proxy/bin/` contains standalone `nym-proxy-client` and `nym-proxy-server` binaries.
- `clientpool`: a configurable pool of ephemeral Nym Clients which can be created as a background process and quickly grabbed.

Documentation can be found [here](https://nym.com/docs/developers/rust).
