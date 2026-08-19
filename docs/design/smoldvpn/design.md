# `nym-smoldvpn` — design

## 1. Goals & non-goals

**Goals**

- A `sdk/rust` crate that establishes a **userspace WireGuard tunnel** to Nym
  gateways using `boringtun` — **no OS `tun` interface, no elevated privileges**.
- **Single-hop** (one gateway) and **two-hop** (entry → exit) modes. Gateways
  support both. Two-hop takes `entry=…` and `exit=…`; single-hop takes just
  `gateway=…` (either an SDK mode option or a dedicated example CLI).
- Caller specifies each gateway independently as: **identity key**, **two-letter
  country code**, or **randomly chosen**.
- Traffic goes in via ordinary `tokio` primitives — a `TcpStream`, a `UdpSocket`,
  or a raw `AsyncRead + AsyncWrite` — so `tonic`, `hyper`, `reqwest`, and raw
  TCP/UDP work inside the tunnel unchanged.
- Caller supplies a **mnemonic** with access to NYM tokens; the crate issues and
  **persists zk-nym ticketbooks** in a credential store so the tunnel can be
  brought up and down at will.
- Fully `tokio`-async. A `CancellationToken` **aborts the setup phase** (the deposit +
  signer round-trip and gateway selection are slow) — up to the point a registration
  ticket is spent, which then runs to completion so a cancel can't lose a ticket — and
  **tears down a long-lived running tunnel** (see §10).
- **Optional QUIC-bridge transport** for clients that are blocked from using pure
  WireGuard (DPI / UDP blocking).
- **Pure `tokio` + Rust**, no Go and no gVisor "netstack" FFI, so a future WASM
  build (with limitations) stays possible.

**Non-goals**

- Not a replacement for the full NymVPN client. It is a **building block** for
  Nym-built and third-party SDK consumers.
- No OS-level routing, kill-switch, or default-route capture. The caller drives
  the tunnel explicitly through the sockets it hands out.
- The QUIC **bridge server** is out of scope — it is deployed independently. Only
  the client side is built here.

## 2. Terminology note

Throughout, **"netstack" refers to the Go/gVisor userspace stack** used elsewhere
(e.g. `nym-gateway-probe`'s `wgPing` FFI). We deliberately do **not** use it. Our
userspace TCP/IP stack is the pure-Rust **smoltcp** stack (the same one `smolmix`
already uses), referred to here as "the smoltcp stack".

## 3. Crate family & layering

```
  common/smol-core            smoltcp stack: channels<IP packet Vec<u8>> → TcpStream / UdpSocket / DNS.
                              Pure tokio + Rust. WASM-capable. No opinion on transport.
      ├── smolmix             + IPR / mixnet bridge          (5-hop)   [exists → refactor onto nym-smol-core]
      └── smoldvpn/  crate nym-smoldvpn
          (nym-smoldvpn)     + boringtun WG datapath        (1-/2-hop) [NEW]

  sdk/rust/nym-sdk-session    ticketbooks (bandwidth-controller) + gateway registration
                              (registration-client). Shared by mixnet AND dvpn. [NEW]
```

- **`nym-smol-core`** is `smolmix` with the mixnet-specific bridge removed: the
  transport-agnostic core that turns a bidirectional stream of IP-packet
  `Vec<u8>` into `tcp_connect() -> TcpStream`, `udp_socket() -> UdpSocket`, and a
  DNS resolver. `smolmix` (5-hop) and `nym-smoldvpn` (2-hop) both sit on it.
- **`nym-sdk-session`** wraps `nym-registration-client` +
  `nym-bandwidth-controller` + the credential store. It answers "acquire paid
  access to these gateways" for **both** mixnet and dvpn modes, so it lives beside
  the mixnet client rather than inside `nym-smoldvpn`.
- **`nym-smoldvpn`** owns only the WireGuard datapath (boringtun) and the
  transport strategy.
- **Dependency isolation (required):** every *new* dependency `nym-smoldvpn` needs
  — `boringtun`, `quinn`, `quinn-proto`, `nym_bridges` (git, pinned), and anything
  else not already used elsewhere — is declared **directly in
  `smoldvpn/Cargo.toml`**, *not* promoted to the workspace-root
  `[workspace.dependencies]` table. Deps already in the workspace table (e.g.
  `smoltcp`, `tokio`, `tokio-smoltcp`, the `nym-*` crates) continue to use
  `workspace = true`. This keeps the WG/QUIC dependency surface contained to this
  one crate.

## 4. The layered data path

### 4.0 The three data-plane modes

Data flows in exactly **three** ways. They are `TunnelMode` × transport, but not a
full matrix — QUIC bridging only applies to the two-hop path (the bridge is bound
1:1 to the entry gateway):

| # | Mode | Tunnels | Transport to entry gateway |
|---|------|---------|-----------------------------|
| 1 | **one-hop** | single `Tunn(client ↔ gateway)` | `Direct` — UDP |
| 2 | **two-hop** | nested `Tunn`s (entry + exit) | `Direct` — UDP |
| 3 | **QUIC-tunnelling two-hop** | nested `Tunn`s (entry + exit) | `QuicBridge` — QUIC to a bridge that relays to the entry gateway |

The exit hop is always reached *through* the entry tunnel, so the QUIC bridge only
ever fronts the **entry-gateway leg**. There is no "QUIC one-hop" mode.

```
   tonic · hyper · reqwest · raw TCP · raw UDP           ← varied caller surfaces
        │   (connector returns AsyncRead+AsyncWrite / datagram)
        ▼
 ┌───────────────────────────────────────────────────────────┐
 │  smoltcp stack (nym-smol-core)                                  │  pure Rust ✓  WASM ✓
 │    tcp_connect() → TcpStream   udp_socket() → UdpSocket     │
 │    DNS resolver bound to the tunnel (default on)            │
 └───────────────────────────────────────────────────────────┘
        │  app IP packet (Vec<u8> over an mpsc-style seam)
        ▼
 ┌───────────────────────────────────────────────────────────┐
 │  boringtun INNER   Tunn(client ↔ EXIT)      [two-hop only]  │  pure Rust ✓  WASM ✓
 │    .encapsulate(app_ip_pkt) → inner WG payload             │  boringtun lives ONLY here
 │        │                                                    │
 │        ▼  frame inner payload as IP/UDP → exit.endpoint     │  built with smoltcp::wire
 │           (NO new dependency — smoltcp already present)     │
 │        │                                                    │
 │        ▼                                                    │
 │  boringtun OUTER   Tunn(client ↔ ENTRY)                     │
 │    .encapsulate(inner_ip_udp_pkt) → outer WG payload        │
 └───────────────────────────────────────────────────────────┘
        │  WG payload (a UDP datagram body)
        ▼
 ┌───────────────────────────────────────────────────────────┐
 │  trait WgPacketTransport { send(Vec<u8>); recv()->Vec }     │  ← the ONLY non-WASM seam
 │    Direct:      tokio::net::UdpSocket → entry.endpoint      │  (carries one WG packet per call)
 │    QuicBridge:  quinn bi-stream, 2-byte len-framed → bridge │
 │    wasm(future): WebTransport (which IS QUIC)               │
 └───────────────────────────────────────────────────────────┘
        ▼
   ENTRY GW ──(decrypts outer, forwards inner to exit.endpoint)──▶ EXIT GW ──▶ internet
```

> **Engine choice — pure Rust, no Go.** The reference client
> (`nym-vpn-client`) runs its datapath on **wireguard-go via FFI** with a
> **gVisor netstack** — the exact Go/netstack dependency we are avoiding.
> `nym-smoldvpn` **must be pure Rust: `boringtun` for the WG engine + `smoltcp`
> for the stack.** We reuse the reference's *architecture and wire protocols*
> (registration, two-hop nesting, QUIC bridge framing) but **not its engine**.
> (`nym-vpn-client` uses `boringtun` only in a throwaway diagnostic probe, never
> for the tunnel — so this datapath is genuinely new pure-Rust code.)

**Single-hop** collapses the INNER tunnel: the app IP packet goes straight into
the single `Tunn(client ↔ gateway)` and out over the transport. No middle framing.

### 4.1 Two-hop nesting detail

**Mechanism confirmed** against the reference client's userspace ("netstack") path
— see §14. The reference does true userspace double-encapsulation: the exit tunnel
encrypts to a **loopback** endpoint, and an **in-tunnel UDP proxy** re-injects that
ciphertext into the entry tunnel addressed to the exit gateway's real endpoint. We
reproduce the same effect with boringtun + smoltcp, with no loopback socket needed
since we own both `Tunn`s directly:

```
outgoing:
  app_ip  = <IP packet from the exit smoltcp stack, dst = internet host>
  inner   = exit_tunn.encapsulate(app_ip)                     // WG ciphertext for EXIT gateway
  mid_ip  = IP(src = entry-assigned IP, dst = exit.endpoint.ip)
            / UDP(src = client_port (fixed, ref default 54001), dst = exit.endpoint.port)
            / inner                                            // built with smoltcp::wire
  outer   = entry_tunn.encapsulate(mid_ip)                    // WG ciphertext for ENTRY gateway
  transport.send(outer)                                        // → entry.endpoint (Direct or QuicBridge)

incoming: exact reverse (entry_tunn.decapsulate → parse mid_ip/UDP → exit_tunn.decapsulate → app_ip)
```

Key points established from the reference:

- The **exit peer `endpoint` from registration is the exit gateway's real public
  address**; the entry tunnel routes it (in the reference's OS-tun mode via the
  entry peer's `allowed_ips = [exit_endpoint_ip, metadata_endpoint_ip]`; in the
  userspace mode via the UDP proxy).
- **Fixed exit-tunnel source port** (`client_port`, reference default **54001**).
- **MTU stepping (actual reference values):** `WG_TUNNEL_OVERHEAD = 80` B/hop, over
  a base of `ETHERNET_V2_MTU = 1500` (desktop) or `MIN_IPV6_MTU = 1280` (iOS/Android).
  - desktop: `ENTRY_MTU = 1500 − 80 = 1420`, `EXIT_MTU = 1500 − 160 = 1340`
  - mobile:  `ENTRY_MTU = 1280 + 80 = 1360`, `EXIT_MTU = 1280`

  So the MTU the application's sockets see (the exit stack) is **~1340 on desktop /
  1280 on mobile**. `nym-smoldvpn` sizes the smoltcp interface accordingly. **MTU
  must be configurable** via the SDK config and **changeable dynamically while the
  tunnel is up** (a config channel that re-sizes the smoltcp interface + re-derives
  the per-hop MTUs at runtime), defaulting to the values above.
- The one hand-built packet (`mid_ip`) uses `smoltcp::wire::{Ipv4Packet, UdpPacket,
  …}` — **no dependency beyond `boringtun` + `smoltcp`**, both WASM-capable.

`boringtun` also emits timer-driven traffic (keepalives, handshake initiation,
rekey). A periodic tick (`Tunn::update_timers`) must be pumped and its output sent
through the same transport.

## 5. Control plane vs. data plane seams

The two seams are handled **separately in time**, and — importantly —
**LP registration cannot be bridged over QUIC yet**. So today:

1. **Register directly** (control plane, always `Direct`): LP handshake over TCP
   to the gateway's LP port. This runs first and is *not* bridgeable at present.
2. **Then bring up the WG tunnel**, and *only the WG data plane* may ride the QUIC
   bridge (`Direct` or `QuicBridge`).

```
                       DIRECT                            QUIC BRIDGE
 control plane   ┌─ LpTransportChannel ─┐          (not bridgeable yet — always Direct)
 (LP register)   │   TcpStream → GW:lp  │          ┌─ LpTransportChannel ─┐
   [always]      └──────────────────────┘          │   TcpStream → GW:lp  │  ← future
                                                    └──────────────────────┘
 data plane      ┌─ WgPacketTransport ─┐          ┌─ WgPacketTransport ───┐
 (WG packets)    │   UdpSocket → GW:wg  │          │ QUIC bi-stream→ BRIDGE │──relay──▶ GW:wg
                 └──────────────────────┘          └───────────────────────┘

 to a DPI/censor:  raw WireGuard UDP    ──vs──   WG data plane is QUIC on :443 (decoy SNI);
                                                 registration is still direct TCP
```

- **Control plane always `Direct` for now.** LP registration is TCP to the gateway
  LP port. The `nym-bridges` forwarder is a fixed-target **UDP** relay to the
  gateway's WG port, so it does not carry the LP/TCP handshake. A censored client
  that can still reach the LP port over TCP registers directly, then bridges only
  the WG UDP data plane.
- **Future:** `nym-lp`'s `LpTransportChannel` is already a trait over
  `AsyncRead + AsyncWrite` (impl'd for `TcpStream`), so an LP-over-QUIC-stream
  wrapper is a natural extension once a bridge that relays the LP port exists. Until
  then, `GatewayTransport::QuicBridge` affects the **data plane only**.
- The **data-plane seam is `WgPacketTransport`** (new, small): one WG packet per
  `send`/`recv`. `Direct` is a real UDP datagram; `QuicBridge` length-frames each
  packet onto a reliable QUIC stream (§6).

## 6. Transport strategies & the QUIC bridge

The bridge protocol is now known (server repo: `nymtech/nym-bridges`; client usage:
`nym-vpn-client`). It is a **transparent, fixed-target forwarder**, not a
client-steerable relay: **each bridge is bound 1:1 to one gateway server-side**, so
the client sends **no gateway-selection handshake** — it opens one QUIC bi-stream
and pipes length-framed WireGuard packets. Because of the 1:1 binding, the bridge
sits on the **entry-gateway leg** (the outer tunnel's transport to the entry
gateway); the exit hop is always reached *through* the entry tunnel regardless.

```rust
enum GatewayTransport {
    /// Reach gateways directly over UDP. Default.
    Direct,
    /// Tunnel the entry leg through a QUIC bridge. For clients blocked from pure
    /// WireGuard/UDP. Params come from the gateway directory / VPN API per gateway,
    /// NOT hand-specified — mirror `nym_bridges::transport::quic::ClientOptions`:
    QuicBridge {
        addresses: Vec<SocketAddr>, // bridge server addr(s), from the directory
        host: Option<String>,       // SNI presented on the wire (a plausible-looking hostname to a
                                    // DPI observer), but see the note below: it must still be an
                                    // accepted cert alt-name, so it is not a free-form decoy
        id_pubkey: String,          // base64 ed25519 identity key, for cert pinning
    },
}
```

The bridge protocol (see §14 for sources):

- **`quinn` 0.11 + `quinn-proto` 0.11, `rustls` 0.23 (ring provider).** The ring
  default crypto provider is installed at startup. `quinn` is a **crate-local
  dependency of `nym-smoldvpn`**, like `boringtun`.
- **ALPN = `hq-29`** (legacy HTTP/QUIC id — *not* `h3`; there is no real HTTP/3).
- **WireGuard packets ride a single reliable QUIC bi-stream, length-framed with a
  2-byte big-endian prefix** (`tokio_util::LengthDelimitedCodec`,
  `length_field_length(2)`) — not RFC 9221 datagrams. Per outbound WG packet: write
  `u16be(len) || packet`; per inbound: read `u16be` then that many bytes.
- **Server-identity cert pinning (ed25519).** The bridge self-signs an X.509 cert
  whose CN/SAN is the base58 of its ed25519 identity key and whose SPKI *is* that
  key. A custom `rustls::ServerCertVerifier` (`IdentityBasedVerifier`) requires: SNI
  ∈ alt-names, CN ∈ alt-names, cert SPKI == pinned `id_pubkey`, valid self-signature,
  and `ED25519`-only verify schemes. The client is anonymous
  (`with_no_client_auth()`).
- **SNI vs. pinning.** The presented SNI looks like an innocuous hostname to a
  passive DPI observer, but it is *not* a free-form decoy: the verifier rejects any
  SNI that is not one of the certificate's accepted alt-names. In practice the
  directory supplies an SNI (`host`) consistent with the bridge's identity cert, so
  the "decoy" property is about how ordinary it *looks*, not about presenting an
  arbitrary unrelated hostname.
- **Delegate the QUIC connection to the `nym-bridges` client; add only the datapath
  framing on top.** `nym-smoldvpn` depends on the `nym-bridges` crate (published to
  crates.io) and uses its `transport::quic::{transport_conn, ClientOptions}` to
  establish the cert-pinned, ALPN-`hq-29` connection, so the client can never drift
  from the bridge server. On top of that connection this crate adds the WireGuard
  datapath: one reliable `open_bi()` stream carrying 2-byte-length-framed WG packets.
- **Session liveness.** `nym-bridges` does not set QUIC keep-alive or a BBR
  congestion controller; WireGuard's own persistent-keepalive keeps the long-lived
  session (and its NAT mapping) alive.
- **Socket-protection hook.** An optional `SocketProtector` callback (Linux/Android)
  lets a VPN app protect the underlying UDP socket from routing loops.
- **The bridge path is the better WASM story, not worse.** Browsers cannot open raw
  UDP, but **WebTransport is HTTP/3 / QUIC and is browser-native** — so the future
  WASM `WgPacketTransport` is essentially "the bridge over WebTransport". The
  censorship feature and the WASM ambition converge on the same abstraction.
- **Amnezia-WG obfuscation** (junk-packet padding at the WG layer) exists in the
  reference as a separate, complementary obfuscation to the QUIC bridge — a possible
  future option, out of scope for v1.

## 7. Credentials & zk-nym ticketbooks

Handled by **`nym-sdk-session`**, reusing existing crates end-to-end:

1. Caller provides a **mnemonic**. →
   `DirectSigningHttpRpcNyxdClient::connect_with_mnemonic(...)`.
2. `NyxdCredentialFetcher` **deposits NYM** on-chain
   (`EcashSigningClient::make_ticketbook_deposit`) and **contacts the Nym API
   signers** (`obtain_aggregate_wallet` over the epoch's ecash API clients) to
   receive an issued ticketbook.
3. `BandwidthController` is the **single writer** to `nym-credential-storage`
   (sqlite). It persists issued ticketbooks and runs an **auto-refill loop** that
   restocks when a ticket type runs low or nears expiry.
4. dVPN needs the wireguard ticket types: **`V1WireguardEntry`** (entry hop) and
   **`V1WireguardExit`** (exit hop). Single-hop needs only the relevant one.

Because ticketbooks are persisted, a second bring-up finds them already stocked
and skips the deposit → **"up and down at will" falls out for free**.

The initial registration (§8) spends the wireguard tickets. Keeping a long-lived
tunnel alive requires **ongoing top-up** to the gateways via their `metadata`
endpoint — see §10.

## 8. Gateway selection

Reuses `nym-client-core` `init/helpers.rs` over `nym-topology` `RoutingNode`s.

```rust
enum GatewaySpec {
    Identity(ed25519::PublicKey),   // get_specified_gateway(..)   — exists
    Country(String),                // two-letter ISO code         — thin new filter
    Random,                         // uniformly_random_gateway(..) — exists
}
```

- **Identity** and **Random** exist directly.
- **Country** is a thin new filter: described nodes carry
  `auxiliary_details.location: Option<celes::Country>` (ISO 3166 alpha-2). Filter
  the candidate set on it, then pick one (random within country).
- **Role filtering:** entry candidates must advertise the `entry` role; exit
  candidates must expose `authenticator` + `ip_packet_router` and
  `can_operate_exit_gateway()`. WireGuard support is indicated by a `wireguard`
  service entry (tunnel/metadata ports + public key) on the node description.

The chosen entry/exit identities become `NymNodeWithKeys` inputs to
`nym-registration-client`'s two-hop registration, which returns two
`WireguardConfiguration`s:

```rust
struct WireguardConfiguration {
    public_key: x25519::PublicKey,   // the gateway's WG pubkey — set in our peer config
    psk: Option<PresharedKey>,       // negotiated by LP registration — set in our peer config
    endpoint: SocketAddr,
    private_ipv4: Ipv4Addr,
    private_ipv6: Ipv6Addr,
}
```

The WireGuard peer config (both for the boringtun datapath and for the exported
plain-WG config in §11) sets the **gateway `public_key`** and the
**LP-negotiated `psk`** from this struct.

## 9. DNS inside the tunnel

- **Default: DNS resolves inside the tunnel** (through the exit), so name
  resolution is not leaked to the host resolver. Configurable (opt-out, or
  point at a specific resolver).
- Implemented by binding a resolver (e.g. `hickory-resolver`) to a `nym-smol-core`
  `UdpSocket` — the same pattern `smolmix` uses (`mixdns`).
- `tonic` / `hyper` / `reqwest` are given a custom connector/resolver that routes
  both the DNS query and the resulting connection through the tunnel.

## 10. Lifecycle & cancellation

```
   Builder ── build() ─▶ Provisioning ──▶ Registering ──▶ Connected(tunnel) ──▶ (torn down)

   CancellationToken:
     • during Provisioning + gateway selection + LP handshake → ABORT setup (slow, no ticket spent)
     • once a registration ticket-spend has begun → runs to completion (never dropped mid-spend,
       so a cancel can't lose a ticket); cancellation takes effect after it returns
     • during Connected                  → TEAR DOWN the long-lived tunnel
```

- **Registering always runs `Direct`** (LP over TCP); the `GatewayTransport`
  choice (`Direct` vs `QuicBridge`) applies only when the WG data plane is brought
  up in the Connected phase. LP-over-QUIC is not available yet (§5).
- The tunnel is **long-lived** and is **explicitly** closed / torn down via the
  cancellation token (or an explicit `shutdown()`).
- While connected, `nym-smoldvpn` runs background tasks: the boringtun timer pump,
  the transport receive loop, the DNS resolver, and a **bandwidth top-up task**.
- **Two distinct top-up layers.** These are separate and independently controlled:
  1. *Gateway-side top-up* (extending a live tunnel): fresh tickets are *pushed to
     the gateways* via the `metadata` endpoint to raise `available_bandwidth`. This
     spends **already-stored** tickets, so it costs nothing new and is **on by
     default** for session-built tunnels — the tunnel stays alive without the caller
     writing plumbing. It can be disabled or run in monitor-only mode.
  2. *Chain-side restock* (buying **new** ticketbooks by depositing NYM): handled by
     the `BandwidthController` and **opt-in** via `SessionConfig::automatic_topups`,
     because a ticketbook is large (≈37.5 GB) and over-requesting costs the
     implementer NYM. Scoping lives in `BandwidthControllerConfig::managed_ticket_types`
     (the set of types the controller proactively restocks): the session sets it to the
     WireGuard types when opted in, and leaves it **empty** by default — an empty set
     means no proactive restock, so the controller only spends existing stock while
     still provisioning on demand. (There is no separate `auto_restock` flag; the
     credential fetcher is unchanged and mixnet types are simply never in the managed
     set, so a dVPN session never deposits for mixnet bandwidth.)
- **The metadata client dials through the tunnel.** The metadata endpoint IP is in
  the entry peer's `allowed_ips` and is served in-tunnel; `nym-smoldvpn` reaches it with
  a hyper HTTP/1 client over the tunnel's own `TunnelConnector`, never the host
  network — so top-up traffic cannot leak the client's real IP.
- **Top-up must fire with headroom.** Because the metadata request itself travels
  in-tunnel, it needs spendable bandwidth to get through — so top-up polls at a
  low-water threshold well above zero (default 100 MiB remaining), not at exhaustion.
  That headroom leaves room for the (small) metadata round-trip before the last bytes
  are consumed; an `Exhausted` event means top-up should already have been attempted.
  Integrators wanting a hard guarantee can raise the threshold / poll cadence.
- **Bandwidth events.** Monitoring is decoupled from acting: the tunnel emits
  `BandwidthEvent`s (`Low`/`ToppedUp`/`TopupFailed`/`Exhausted`) on a broadcast
  channel (`Tunnel::bandwidth_events()`) whenever a metadata endpoint is known, even
  with automatic top-up disabled — so an implementer can prompt the user to obtain
  more ticketbooks and drive top-up themselves.

## 11. Public API

> The shape below is illustrative; see the crate README and rustdoc for the
> current public surface.

```rust
// ── nym-sdk-session (shared provisioning facade) ────────────────────────────
let session = SessionBuilder::new(mnemonic, network)
    .credential_store(store_path)          // persistent → up/down at will
    .build()?;

// ── nym-smoldvpn ─────────────────────────────────────────────────────────────
let tunnel = DvpnTunnelBuilder::new(session)
    // two-hop: specify entry AND exit
    .mode(TunnelMode::TwoHop {
        entry: GatewaySpec::Country("CH".into()),
        exit:  GatewaySpec::Random,
    })
    // single-hop alternative: specify just one gateway
    // .mode(TunnelMode::SingleHop { gateway: GatewaySpec::Identity(key) })
    .transport(GatewayTransport::Direct)    // or QuicBridge { .. }
    .dns_in_tunnel(true)                    // default
    .cancellation(token.clone())
    .connect()                              // provisions → registers → brings up datapath
    .await?;

// varied traffic surfaces:
let stream  = tunnel.tcp_connect("10.64.0.1:443".parse()?).await?; // AsyncRead+AsyncWrite
let udp     = tunnel.udp_socket().await?;                          // datagrams
let channel = tunnel.tonic_channel("https://exit.internal:443").await?; // tonic adapter
let client  = tunnel.reqwest_client()?;                            // reqwest w/ tunnel connector

token.cancel();  // tears the tunnel down; issued tickets remain in the store
```

Traffic surfaces to expose (all reuse `nym-smol-core`):

- `tcp_connect(addr) -> TcpStream` — raw TCP, and the base for everything below.
- `udp_socket() / udp_socket_on(port) -> UdpSocket` — raw UDP.
- A **connector** (tower `Service<Uri>` returning `AsyncRead + AsyncWrite`) that
  `tonic`, `hyper`, and `reqwest` accept directly.
- Thin `tonic_channel(..)` / `reqwest_client(..)` conveniences over that connector.
- `allocated_ips()`, `available_bandwidth()`, `shutdown()`.

## 12. Example CLIs (in `smoldvpn/examples`)

- **`smoldvpn-config --gateway <spec>`** — performs an LP registration against a
  **single gateway** and prints a **plain WireGuard config** (`[Interface]
  PrivateKey/Address`, `[Peer] PublicKey/PresharedKey/Endpoint/AllowedIPs`) usable
  with stock `wg`/`wg-quick`. This is a **single-hop** config; **two-hop requires
  chaining wg clients**, which stock `wg-quick` cannot express in one interface.
  Single-hop is a first-class supported VPN mode (gateways support it). The config
  is valid only for the duration of the registered zk-nym bandwidth and
  **disconnects when the bandwidth is used up** — stock kernel WireGuard knows
  nothing about ecash top-up.
- **`smoldvpn-topup`** — spends a stored ticket to **top up bandwidth via the
  gateway `metadata` endpoint** (`wireguard-private-metadata` client), extending an
  existing registration/config so it survives past its initial bandwidth.

## 13. Reuse map (concrete crates & paths)

| Concern | Crate / path |
|---|---|
| Signing chain client (mnemonic) | `nym-validator-client` — `DirectSigningHttpRpcNyxdClient::connect_with_mnemonic` |
| Deposit + signer issuance | `nym-bandwidth-fetcher` — `common/bandwidth-fetcher/src/credentials.rs` (`NyxdCredentialFetcher`) |
| Controller + auto-refill | `nym-bandwidth-controller` — `common/bandwidth-controller/src/controller.rs` |
| Credential store (sqlite) | `nym-credential-storage` — `Storage` trait, `initialise_persistent_storage` |
| Ticket types | `nym-credentials-interface` — `TicketType::{V1WireguardEntry, V1WireguardExit}` |
| Two-hop WG registration | `nym-registration-client` — `RegistrationClientBuilder`, `RegistrationMode::Wireguard`, `clients/lp.rs` |
| WG config / assigned IPs | `nym-registration-common` — `common/registration/src/lib.rs` (`WireguardConfiguration`) |
| LP transport (control-plane seam) | `nym-lp` — `common/nym-lp/src/transport/traits.rs` (`LpTransportChannel`) |
| Gateway directory + selection | `nym-validator-client`, `nym-client-core` — `init/helpers.rs`, `init/types.rs` |
| Country field on nodes | `nym-api-requests` — described-node `auxiliary_details.location: Option<celes::Country>` |
| smoltcp stack pattern | `smolmix` — `smolmix/core/src/{tunnel,device}.rs` → generalise into `nym-smol-core` |
| Metadata top-up | `nym-wireguard-private-metadata` (client) — `topup_bandwidth`, `available_bandwidth` |
| WG datapath (new) | `boringtun` (`Tunn`) + `smoltcp::wire` for the middle IP/UDP frame |
| QUIC bridge (new) | `quinn` 0.11 via the `nym-bridges` client (git-pinned, `transport::quic`): ALPN `hq-29`, ed25519-SPKI pinning; this crate adds the 2-byte len-framed WG packets over one bi-stream |

## 14. Reference implementations studied

Two external Nym repos were read to validate the two-hop and bridge protocols.
`nym-smoldvpn` reuses their **architecture and wire protocols** but **not their
engine** (they are Go; we are pure Rust).

**`nym-vpn-client`** (`nym-vpn-core` workspace) — the production VPN client:
- Datapath engine: **wireguard-go via FFI** (`nym-wg-go` crate) with a **gVisor
  netstack** — the Go/netstack dependency we explicitly avoid. `boringtun` is used
  only in `nym-diagnostic` for a throwaway handshake probe, never for the tunnel.
- Two-hop: `wg_config.rs` maps `WireguardConfiguration` → `PeerConfig` (UAPI text).
  `connected_tunnel.rs` has both a desktop OS-tun mode (`run_using_tun_tun`,
  Mullvad-style allowed-ips routing) and a **userspace mode** (`run_using_netstack`
  + `two_hop_config.rs`) that nests via a **loopback UDP proxy** — the model §4.1
  reproduces with boringtun. Fixed exit `client_port` (default 54001); WG overhead
  80 B/hop.
- QUIC (canonical reference to mirror):
  **`nym-vpn-core/crates/nym-vpn-lib/src/tunnel_state_machine/tunnel/transports/`**
  (`mod.rs` + `certs.rs`). `quinn` 0.11 declared directly (no `nym_bridges` dep).
  Key items: `BridgeConn::try_connect` (`conn.open_bi()` under
  `run_until_cancelled`), `UdpForwarder::launch` (local-UDP shim), `process_udp`
  (the 2-byte `LengthDelimitedCodec` relay loop), `transport_conn` (sets ALPN
  `hq-29`, `keep_alive_interval`, `max_idle_timeout`, BBR), `ClientOptions` +
  `TryFrom<&QuicClientOptions>`, `ALPN_QUIC_HTTP = [b"hq-29"]`. `certs.rs` =
  `IdentityBasedVerifier` (ed25519-SPKI pinning). Optional `on_socket_open:
  FnOnce(RawFd)` socket-protection hook (Linux/Android). Bridge params
  (`QuicClientOptions { addresses, host, id_pubkey }`) come from the VPN API per
  gateway.

**`nym-bridges`** (`github.com/nymtech/nym-bridges`) — the bridge server, and the
QUIC client `nym-smoldvpn` builds on. `nym-smoldvpn` depends on the crate
(git-pinned to a commit, since the protocol is versioned `"0"` and unstable) and
uses its client transport rather than reimplementing the connection.
- Modules used: `transport::quic::{ClientOptions, transport_conn}` (configured
  `quinn::Connection`) and `transport::tls::certs::IdentityBasedVerifier` (pinning).
  This crate layers the 2-byte-length WG framing on top of the connection.
- **Transparent fixed-target forwarder:** the target gateway is set server-side
  (`[forward] address`); **no client-sent gateway-selection handshake**. Bridge is
  1:1 with a gateway. Default listen `[::]:4443` (`:443` in the wild). No client
  auth. Protocol `version = "0"`, explicitly unstable — **pin to a commit**.
  `id_pubkey` is base64 of the ed25519 **public** key (don't confuse with the
  server's secret-key config field).
