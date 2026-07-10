# nym-sdk-session

A provisioning facade over `nym-registration-client`, `nym-bandwidth-controller`
/ `nym-bandwidth-fetcher`, and the credential store. From a caller-supplied
mnemonic it issues and persists zk-nym WireGuard ticketbooks, selects gateways,
and registers them — returning the per-hop `WireguardConfiguration` a datapath
(e.g. [`nym-smol-dvpn`](../smol-dvpn)) needs.

Shared by both mixnet and dVPN modes (design D3).

## Usage

```rust
use nym_sdk_session::{Session, SessionConfig, GatewaySpec};
use tokio_util::sync::CancellationToken;

let session = Session::new(
    SessionConfig {
        mnemonic,
        network,                         // NymNetworkDetails (e.g. sandbox)
        credential_store_path: Some("creds.db".into()),
        data_path: "session-data".into(),
    },
    CancellationToken::new(),
).await?;

// Issue + persist the WireGuard ticketbooks (deposits NYM if needed).
session.ensure_ticketbooks(/* two_hop = */ true).await?;

// Two-hop registration by country codes:
let registration = session
    .register_two_hop(
        &GatewaySpec::Country("CH".into()),
        &GatewaySpec::Country("DE".into()),
    )
    .await?;

// `registration.entry` / `registration.exit` are `HopConfig`s carrying the
// gateway pubkey, negotiated PSK, endpoint, assigned IPs, and the client's WG
// key — everything a datapath needs.
```

## Gateway selection

`GatewaySpec::Identity(key)` / `Country("XX")` / `Random`, filtered to
WireGuard-capable nodes (country = the described-node `location`). Single-hop
uses the LP single-gateway `register_dvpn` path; two-hop registers entry + exit.

## Cancellation

Every setup/issuance/registration entry point is driven under the supplied
`CancellationToken`, so a slow provisioning phase can be aborted.

## Design

See `sdk/rust/docs/nym-sdk-dvpn/` and the `dvpn-session` OpenSpec capability.
