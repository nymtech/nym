## 1. Conformance spikes (de-risk first, throwaway)

- [x] 1.1 Spike A — two-hop boringtun nesting parity: register entry+exit via `nym-registration-client`, build the exit→IP/UDP→entry encapsulation with `boringtun` + `smoltcp::wire`, push one hand-framed packet, and confirm a reply matches the `nym-vpn-client` reference behavior (fixed exit source port 54001)
- [x] 1.2 Spike B — QUIC bridge one-packet round-trip: connect to a bridge with `quinn` (ALPN `hq-29`, ed25519-SPKI pinning), open one bi-stream, send a 2-byte-length-framed WireGuard packet and confirm the framed reply
- [x] 1.3 Record spike outcomes; confirm the two-hop mechanism and bridge framing before building the real crates

## 2. `common/smol-core` (shared smoltcp stack)

- [x] 2.1 Create `common/smol-core` crate; add to workspace `members`
- [x] 2.2 Extract the transport-agnostic smoltcp stack from `smolmix` (device wiring, `Net`, `TcpStream`/`UdpSocket`) behind an abstract IP-packet transport (`Vec<u8>` in/out)
- [x] 2.3 Add the tunnel-scoped DNS resolver (bound to a stack UDP socket)
- [x] 2.4 Refactor `smolmix` to consume `smol-core`; preserve its public API
- [x] 2.5 Regression tests proving `smolmix`'s public API and behavior are unchanged; unit tests for `smol-core` TCP/UDP/DNS

## 3. `sdk/rust/nym-sdk-session` (provisioning facade)

- [x] 3.1 Create `nym-sdk-session` crate; add to workspace `members`
- [x] 3.2 Mnemonic → nyxd signing client → `NyxdCredentialFetcher` + `BandwidthController` wiring; issue `V1WireguardEntry`/`V1WireguardExit` ticketbooks
- [x] 3.3 Persistent credential store integration; reuse stored tickets on subsequent bring-up
- [x] 3.4 Gateway selection: identity / two-letter country code / random, filtered to WireGuard-capable nodes (country = filter on described-node `location`)
- [x] 3.5 Gateway registration via `nym-registration-client`; return per-hop `WireguardConfiguration` (pubkey + PSK + endpoint + assigned IPs)
- [x] 3.6 `CancellationToken` support to abort the setup/issuance phase
- [x] 3.7 Implement single-hop as a single-gateway registration (`BuilderConfig` mandates entry+exit and the reference is two-hop-only, so use the LP single-gateway `register_dvpn` path rather than forcing `entry == exit`); expose `gateway=` for single-hop and `entry=`/`exit=` for two-hop
- [x] 3.8 Tests for issuance, storage reuse, each selection mode, and setup abort
- [x] 3.9 Optional dVPN gateway-directory client (`SessionConfig.dvpn_directory_url`, best-effort fetch); enrich per-hop `GatewayInfo` with the gateway moniker (`name`) + node id/country/IP, exposed on `HopConfig`
- [x] 3.10 QUIC-bridge entry selection: `register_two_hop_quic` requires a QUIC-capable entry per the directory (honoring the `GatewaySpec`), returns the entry `QuicBridge` params on `HopConfig.bridge`, and fails with `SessionError::NoQuicGateway` when none match; single-hop / non-QUIC two-hop carry no bridge
- [x] 3.11 Two-hop selection excludes the entry gateway from the exit pool so the hops are always distinct (`SessionError::SameGatewaySelected` if the exit spec can only match the entry)

## 4. `sdk/rust/smol-dvpn` datapath (`nym-smol-dvpn`)

- [x] 4.1 Create `sdk/rust/smol-dvpn` crate (`nym-smol-dvpn`); add to workspace `members`; declare `boringtun`, `quinn`, `quinn-proto` in its own `Cargo.toml` (not the workspace table)
- [x] 4.2 Define the `WgPacketTransport` seam (one WG packet per send/recv) and the `Direct` UDP implementation
- [x] 4.3 Single-hop datapath: one `boringtun` `Tunn` on `smol-core`, peer configured from registration (pubkey + PSK)
- [x] 4.4 Two-hop datapath: nested `Tunn`s with the exit→IP/UDP→entry encapsulation from Spike A
- [x] 4.5 boringtun timer pump on a dedicated cancellable task, routed through the active transport
- [x] 4.6 Tunnel lifecycle + `CancellationToken` (abort setup / teardown long-lived tunnel; `shutdown()`); tickets retained on teardown
- [x] 4.7 Configurable, runtime-adjustable MTU with reference defaults (overhead 80/hop; desktop 1420/1340; mobile 1360/1280) — `Tunnel::set_mtu()` rebuilds the smol-core interface at the new MTU while preserving the WireGuard session (no re-handshake); verified live (resolve → set_mtu(MOBILE) → resolve). Note: `tokio-smoltcp 0.5` fixes the interface MTU at construction, so the change rebuilds the interface rather than a fully seamless in-place resize.
- [x] 4.8 DNS-in-tunnel default (configurable) via the `smol-core` resolver
- [x] 4.9 Background bandwidth top-up task via the `nym-wireguard-private-metadata` client
- [x] 4.10 Optional `on_socket_open`-style socket-protection callback (Linux/Android)
- [x] 4.11 Integration test: bring up single-hop and two-hop tunnels and pass TCP/UDP traffic

## 5. Traffic surfaces & connectors

- [x] 5.1 Expose `tcp_connect` (`AsyncRead+AsyncWrite`) and UDP socket surfaces
- [x] 5.2 `tonic` connector/channel adapter
- [x] 5.3 `hyper` and `reqwest` connector adapters
- [x] 5.4 Example + test: `tonic` gRPC request through the tunnel (`examples/smol-dvpn-grpc.rs`: single-hop bring-up → `tonic` channel over `tunnel.connector()` → gRPC Health `Check`, using `tonic-health`'s client; compiles, run against a live gRPC health service like the other live examples)

## 6. `dvpn-quic-bridge` (QUIC-tunnelling two-hop)

- [x] 6.1 Inline QUIC client module mirroring `nym-vpn-client` `transports/` (no `nym_bridges` dep): `quinn` setup, ALPN `hq-29`, `keep_alive_interval` + `max_idle_timeout` + BBR
- [x] 6.2 `IdentityBasedVerifier` (ed25519-only, SNI/CN ∈ alt-names, SPKI == pinned key)
- [x] 6.3 2-byte big-endian length framing per WG packet over one `open_bi()` stream; wire as the `QuicBridge` `WgPacketTransport`
- [x] 6.4 Source bridge params (`addresses`, SNI host, base64 ed25519 `id_pubkey`) from the gateway directory; enforce QUIC only on the two-hop entry leg (reject QUIC one-hop)
- [x] 6.5 Keep LP registration Direct-only; bridge the WG data plane only; cancellable connect
- [x] 6.6 Conformance test against a pinned bridge reference (or mock) for framing + pinning

## 7. Example CLIs (`dvpn-tools`)

- [x] 7.1 `smol-dvpn-config --gateway <spec>`: single-hop LP registration → plain WireGuard config export (Interface + Peer with pubkey/PSK/endpoint/allowed-ips)
- [x] 7.2 `smol-dvpn-topup`: spend a stored ticket via the gateway `metadata` endpoint and report updated available bandwidth
- [x] 7.3 `two-hop-ip`: query `ipinfo.io` directly then through the tunnel to show the public IP/location relocate to the exit gateway (verified live: CH → DE)
- [x] 7.4 `two-hop-quic`: same as `two-hop-ip` but the entry leg is carried over a QUIC bridge (selects a QUIC-capable entry from the directory; verified live up to the QUIC dial — sandbox bridge endpoint currently unreachable)
- [x] 7.5 `zcash-sync`: sync a configurable number of Zcash compact blocks (default 10,000, `--blocks <N>`) from `zec.rocks:443` (gRPC-over-TLS, hand-written prost messages + manual tonic client) direct vs. through the tunnel and compare throughput (verified live)
- [x] 7.6 Shared example CLI (`examples/common`): `--one-hop`/`--two-hop`, `--entry`/`--exit`/`--gateway <spec>` (random / ISO country / base58 identity), `--quic`, `--blocks <N>` (zcash-sync), with validation; plus a TLS-over-tunnel connector and an HTTPS `ipinfo.io` fetcher

## 8. Docs, licensing, and finalization

- [x] 8.1 Crate READMEs + usage examples; link the design docs in `docs/design/sdk/smol-dvpn/`
- [x] 8.2 Confirm license notices for `boringtun` (BSD-3-Clause) and `quinn` (MIT/Apache-2.0) are retained; verify workspace license compliance
- [x] 8.3 Ensure no Go / gVisor netstack and no new workspace-table deps introduced; `cargo test`/`clippy` clean across the three crates
