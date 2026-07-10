# Conformance spike findings (OpenSpec `nym-sdk-dvpn`, task 1)

Throwaway spikes that de-risk the two novel mechanisms before building the real
crates. Both are **self-contained** (no live gateway, credentials, or network) —
they isolate the genuine unknowns rather than re-proving already-proven paths.

## Key architectural finding (affects the design doc)

The `nym-vpn-client` reference two-hop datapath runs on **wireguard-go + gVisor
netstack (Go/FFI via the `nym-wg-go` crate)**, *not* `boringtun`. The inner
IP/UDP framing and double-encapsulation happen **inside Go**. `boringtun` appears
in the reference only in an offline diagnostic ping tool
(`nym-diagnostic/.../wireguard.rs`), never in the tunnel datapath.

Consequence: the design's "confirmed against `nym-vpn-client`" (D4) is accurate
for the **topology, the fixed-port semantics, and the MTU/overhead constants**,
but the **pure-Rust `boringtun` + `smoltcp::wire` nesting is genuinely new code**
with no line-for-line reference. That is precisely what Spike A proves.
Recommend a one-line clarification in `design.md` D4 to say the Rust datapath
*re-implements* (not mirrors) the reference, which is Go/gVisor.

## Reference invariants captured (with citations)

Reference repo: `/Users/mark/data/nym/workspace/nym-vpn-client`.

### Two-hop nesting
- Topology + in-tunnel UDP forwarder: `nym-vpn-lib/.../wireguard/connected_tunnel.rs`
  `run_using_netstack()` (~:319–349, :394); entry tunnel encrypts to entry gw,
  exit-tunnel ciphertext is framed as inner IP/UDP and forwarded through it.
- Fixed exit client port `DEFAULT_EXIT_WG_CLIENT_PORT = 54001`
  (`two_hop_config.rs:17`) — a **fallback** behind a dynamically-bound port
  (`get_dynamic_port`, :49–54), pinned to the exit interface `listen_port`.
- MTU/overhead (`two_hop_config.rs:8–29`): `WG_TUNNEL_OVERHEAD = 80`;
  desktop entry `1500-80=1420` / exit `1500-160=1340`; mobile entry `1280+80=1360`
  / exit `1280`. Matches design D9 exactly.
- boringtun single-packet API (diagnostic): `Tunn::new(static_private,
  peer_public, Option<psk>, Option<keepalive>, index, Option<RateLimiter>)`;
  encapsulate → `WriteToNetwork`; decapsulate drain loop over
  `WriteToNetwork`/`WriteToTunnelV4|V6`/`Done`.

### QUIC bridge
- No `nym_bridges` crate exists anywhere; the inline client under
  `.../tunnel/transports/{mod.rs,certs.rs}` is the sole reference.
- ALPN `hq-29` (`mod.rs`), keepalive 20s, idle 60s, BBR, `open_bi()` once,
  `LengthDelimitedCodec` with 2-byte length (`mod.rs`).
- `IdentityBasedVerifier` (`certs.rs`): SNI ∈ alt-names, CN ∈ alt-names, cert
  SPKI == pinned ed25519 key (`ed25519_dalek::pkcs8`), verify schemes ED25519
  only. **Note:** the reference treats a *SPKI-parse failure* as non-fatal
  (only `trace!`s); the spike tightens this to a hard reject — recommend the
  real crate reject on parse failure too.
- Bridge params `QuicClientOptions { addresses: Vec<SocketAddr>, host:
  Option<String>, id_pubkey: String (base64 ed25519) }`
  (`nym-vpn-api-client/src/response.rs`), sourced from the gateway directory
  `bridges` field; **no** gateway-selection handshake is sent to the bridge.

### Local registration API (for phases 3–4, confirmed)
- `WireguardConfiguration { public_key, psk: Option<PresharedKey>, endpoint,
  private_ipv4, private_ipv6 }` (`common/registration/src/lib.rs:48`).
- Single-hop: `LpRegistrationClient::register_dvpn(rng, wg_keypair,
  gateway_identity, &dyn BandwidthTicketProvider, ticket_type)`
  (`lp_client/client.rs:539`). Two-hop: `NestedLpSession`
  (`lp_client/nested_session/mod.rs:338`).
- Best end-to-end templates: `nym-gateway-probe/src/common/probe_tests.rs:160`
  and `integration-tests/src/lp_registration.rs` (single + two-hop).
- `BuilderConfig` mandates both `entry_node` + `exit_node` (confirmed) →
  single-hop must call the low-level single-gateway path directly (task 3.7).

## Spike results

Both spikes pass (`cargo run --bin spike_a_nesting` / `--bin spike_b_quic`,
exit 0), fully in-process with no live infra.

### Spike A — two-hop boringtun nesting: **PASS**
- Single-hop: 71 B app IP packet round-trips through one `Tunn` (103 B on wire).
- Two-hop: exit `Tunn` → `smoltcp::wire` IPv4/UDP frame (src `10.2.0.2:54001`,
  dst exit endpoint) → entry `Tunn` → entry-gw decap → parse → exit-gw decap
  recovers the original 71 B packet byte-for-byte. Inner source port `54001`
  and exit endpoint preserved across the nesting.
- **Confirmed:** the pure-Rust `boringtun` + `smoltcp::wire` nesting works and is
  the correct model for task 4.4. The reference-only unknown (parity of the
  hand-built inner frame vs. what gVisor emits) is resolved for the encap/decap
  path; the remaining live variable is real-gateway acceptance, deferred to a
  sandbox integration test.

### Spike B — QUIC bridge framing + pinning: **PASS**
- Positive: 1200 B WG packet round-trips over one `open_bi()` stream with 2-byte
  BE length framing, ALPN `hq-29`, ed25519-SPKI pin, no selection handshake.
- Negative: a corrupted pin is rejected during the TLS handshake
  (`SPKI does not match pinned id_pubkey`).
- **Confirmed:** the inline `quinn` client model (tasks 6.1–6.3) is sound; the
  `IdentityBasedVerifier` and framing reproduce the reference behavior.

## Live validation (sandbox, task 4.11)

The full stack was validated end-to-end against **live sandbox gateways** via
`sdk/rust/smol-dvpn/tests/live_bringup.rs` (funded mnemonic → on-chain deposit →
zk-nym issuance → LP registration → boringtun datapath → traffic):

- **Single-hop:** PASS — resolved `nymtech.net` (→ 76.76.21.21) through the tunnel.
- **Two-hop (nested):** PASS — resolved through the full entry→exit path.

Two real datapath bugs were found and fixed during live bring-up (not visible in
the in-process spike, which owned both server `Tunn`s):
1. **Inner-frame source IP.** The exit→entry carrier frame must be sourced from
   the **entry**-assigned tunnel IP (not the exit's) — the entry gateway's
   cryptokey routing (allowed-ips) drops any other source.
2. **Return-path filter.** Inbound inner frames arrive `src=exit_endpoint,
   dst=our_tunnel_ip`; validate on `src == exit_endpoint`, not `dst` (an earlier
   `dst`-based check silently dropped every exit reply, so the exit handshake
   never completed).

Known follow-up: teardown of a live two-hop tunnel can be slow (the live test
force-exits after asserting traffic); worth investigating the reactor/datapath
shutdown interaction.

### Follow-ups for the real crates
1. `design.md` D4: note the Rust datapath *re-implements* the reference (Go/gVisor),
   it does not mirror boringtun code (there is none in the ref datapath).
2. Real crate should **reject** on SPKI parse failure (ref only `trace!`s).
3. Fixed exit port 54001 is a fallback in the ref; the real datapath should bind
   a dynamic port and fall back to 54001, matching `two_hop_config.rs`.
4. `smoltcp` needs a coherent feature set incl. a socket feature (mirror
   `smolmix/core`) or its own lib won't compile.
