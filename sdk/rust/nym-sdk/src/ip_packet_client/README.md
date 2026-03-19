# ip_packet_client

Minimal fork of helpers from the `nym-vpn-client` monorepo, kept inside `nym-sdk`
to avoid a circular dependency (`nym-ip-packet-client` → `nym-sdk` → circular).

Used exclusively by `ipr_wrapper::IpMixStream` for communicating with IPR exit
gateways over the mixnet.

## What's here

- **discovery.rs** — find a random IPR exit gateway via the Nym API, parse connect responses
- **listener.rs** — `IprListener`: deserialize `IpPacketResponse` bytes, extract IP packets via `MultiIpPacketCodec`
- **helpers.rs** — IPR protocol version check + ICMP packet construction/parsing for integration tests
- **error.rs** — error types for the above

## Where the originals live

All derived from crates in [`nym-vpn-client/nym-vpn-core/crates/`](https://github.com/nymtech/nym-vpn-client/tree/develop/nym-vpn-core/crates):
- `nym-ip-packet-client` (discovery, connect flow)
- `nym-connection-monitor` (ICMP helpers)
- `nym-gateway-probe` (IP packet helpers)
