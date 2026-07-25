# nym-sdk-session

A provisioning facade over `nym-registration-client`, `nym-bandwidth-controller`
/ `nym-bandwidth-fetcher`, and the credential store. From a caller-supplied
mnemonic it issues and persists zk-nym WireGuard ticketbooks, selects gateways,
and registers them — returning the per-hop `WireguardConfiguration` a datapath
(e.g. [`smoldvpn`](../smoldvpn)) needs.

Shared by both mixnet and dVPN modes.

## Usage

```rust
use nym_sdk_session::{Session, SessionConfig, GatewaySpec};
use tokio_util::sync::CancellationToken;

let config = SessionConfig::new(mnemonic, network, "session-data".into())
    .with_credential_store_path("creds.db");
    // .with_dvpn_directory_url(url)        // gateway monikers + QUIC selection
    // .with_automatic_topups(RestockPolicy::default())  // opt in to background re-issuance
let session = Session::new(config, CancellationToken::new()).await?;

// Issue + persist the WireGuard ticketbooks (deposits NYM if needed). By default the
// session provisions once and never deposits in the background; a live tunnel still
// tops up from already-stored tickets. Opt into background re-issuance with
// `SessionConfig::with_automatic_topups`.
session.ensure_ticketbooks(/* two_hop = */ true).await?;

// Two-hop registration by country codes:
let registration = session
    .register_two_hop(
        &GatewaySpec::Country("CH".into()),
        &GatewaySpec::Country("DE".into()),
    )
    .await?;

// `registration.entry` / `registration.exit` are `HopConfig`s carrying the
// gateway pubkey, negotiated PSK, endpoint, assigned IPs, the client's WG key,
// per-hop gateway metadata (`GatewayInfo`: identity, node id, country, moniker),
// and — for a QUIC entry — the `QuicBridge` params.
```

## Gateway selection

`GatewaySpec::Identity(key)` / `Country("XX")` / `Random`, filtered to
WireGuard-capable nodes (country = the described-node `location`). Single-hop
uses the LP single-gateway `register_dvpn` path; two-hop registers entry + exit.
Two-hop selection excludes the entry gateway from the exit pool, so the two hops
are always distinct gateways (an exit spec that can only match the entry gateway
fails with `SessionError::SameGatewaySelected`).

## dVPN directory: monikers + QUIC entry selection

When `SessionConfig::dvpn_directory_url` is set, the session fetches the dVPN
gateway directory (best-effort — a fetch failure is logged and treated as empty)
to enrich each `GatewayInfo` with the gateway's human **moniker** and to enable
QUIC-bridge entry selection. `register_two_hop_quic(entry, exit)` selects the
entry only among directory gateways that advertise a QUIC bridge (honoring the
`GatewaySpec`), returns the `QuicBridge` params (addresses / SNI host / base64
ed25519 `id_pubkey`) on `registration.entry.bridge`, and fails with
`SessionError::NoQuicGateway` if none match. QUIC fronts the two-hop entry leg
only; `register_single_hop` / `register_two_hop` are unchanged (`bridge = None`).

## Cancellation

Every setup/issuance/registration entry point is driven under the supplied
`CancellationToken`, so a slow provisioning phase can be aborted.

## Design

See the architecture docs in
[`docs/design/sdk/smoldvpn/`](../../../docs/design/sdk/smoldvpn/) and the
[`dvpn-session`](../../../openspec/specs/dvpn-session/spec.md) OpenSpec capability.
