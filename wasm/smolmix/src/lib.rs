// Copyright 2026 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

//! smolmix-wasm: drop-in browser networking over the Nym mixnet.
//!
//! Exposes three APIs that mirror the browser's native networking surface:
//!
//! - **`mixFetch(url, init)`**: drop-in `fetch()` replacement (HTTP/HTTPS)
//! - **`mixWebSocket(url, protocols, onEvent)`**: drop-in `WebSocket` replacement (WS/WSS)
//! - **`mixDNS(hostname)`**: DNS-only hostname lookup (UDP / IPR path, no TCP/TLS)
//!
//! All three share the same mixnet tunnel (DNS, TCP, TLS), initialised once
//! via `setupMixTunnel(opts)` and torn down with `disconnectMixTunnel()`.

// All modules gated on wasm32 so `cargo check` on the host triple sees an empty crate.
// Cargo features (`dns` / `fetch` / `websocket`) further gate the entry-point modules
// and their heavy deps; see [features] in Cargo.toml.
#[cfg(target_arch = "wasm32")]
mod bridge;
#[cfg(target_arch = "wasm32")]
mod device;
// DNS resolves over DoH now, so it needs the HTTP/TLS stack (`fetch` or
// `websocket`, which both pull it). Every real feature combo includes one; only
// a bare `--no-default-features` build with none of them excludes the resolver.
#[cfg(all(target_arch = "wasm32", any(feature = "fetch", feature = "websocket")))]
mod dns;
#[cfg(target_arch = "wasm32")]
mod error;
#[cfg(all(target_arch = "wasm32", any(feature = "fetch", feature = "websocket")))]
mod fetch;
#[cfg(all(target_arch = "wasm32", feature = "fetch"))]
mod http;
#[cfg(target_arch = "wasm32")]
mod ipr;
#[cfg(all(target_arch = "wasm32", feature = "dns"))]
mod mixdns;
#[cfg(all(target_arch = "wasm32", feature = "fetch"))]
mod mixfetch;
#[cfg(all(target_arch = "wasm32", feature = "websocket"))]
mod mixwebsocket;
#[cfg(target_arch = "wasm32")]
mod reactor;
#[cfg(target_arch = "wasm32")]
mod state;
#[cfg(target_arch = "wasm32")]
mod stream;
#[cfg(all(target_arch = "wasm32", any(feature = "fetch", feature = "websocket")))]
mod tls;
#[cfg(target_arch = "wasm32")]
mod tunnel;
#[cfg(target_arch = "wasm32")]
mod util;

#[cfg(target_arch = "wasm32")]
pub use error::FetchError;
#[cfg(target_arch = "wasm32")]
pub use tunnel::WasmTunnel;
// Re-exported so the `pub` wasm-bindgen `getTunnelState` accessor can name the
// return type, and so tsify emits its `.d.ts` (see state.rs).
#[cfg(target_arch = "wasm32")]
pub use state::{FailureReason, TaskName, TunnelState};

#[cfg(target_arch = "wasm32")]
use serde::Deserialize;
#[cfg(target_arch = "wasm32")]
use std::sync::OnceLock;
#[cfg(target_arch = "wasm32")]
use tsify::Tsify;
#[cfg(target_arch = "wasm32")]
use wasm_bindgen::prelude::*;
#[cfg(target_arch = "wasm32")]
use wasm_bindgen_futures::future_to_promise;

/// Global tunnel singleton, set once by `setupMixTunnel`, stays in the OnceLock after shutdown.
#[cfg(target_arch = "wasm32")]
pub(crate) static TUNNEL: OnceLock<WasmTunnel> = OnceLock::new();

/// Resolve the tunnel and gate on readiness. Used by every JS entry point.
#[cfg(target_arch = "wasm32")]
pub(crate) fn ready_tunnel() -> Result<&'static WasmTunnel, FetchError> {
    let tunnel = TUNNEL.get().ok_or(FetchError::NotConnected)?;
    if !tunnel.is_ready() {
        return Err(FetchError::Tunnel(format!(
            "tunnel not ready: {:?}",
            tunnel.tunnel_state()
        )));
    }
    Ok(tunnel)
}

/// Options accepted by `setupMixTunnel`. Deserialised from the JS object via
/// `serde-wasm-bindgen` + `tsify`, which gives us typed access without manual
/// `Reflect::get` plumbing and emits a matching `.d.ts` for the TS side.
#[derive(Tsify, Deserialize)]
#[tsify(from_wasm_abi)]
#[serde(rename_all = "camelCase")]
#[cfg(target_arch = "wasm32")]
pub struct SetupOpts {
    /// Nym address of the IPR exit node. Omit (or pass `null`) to let
    /// smolmix auto-discover a performance-weighted random IPR via the
    /// Nym API directory.
    #[serde(default)]
    pub preferred_ipr: Option<String>,
    /// Identity key (base58) of the entry gateway to register with. Omit (or
    /// pass `null`) for performance-weighted random selection. Only consulted
    /// on the first registration for a given `clientId`; a client that already
    /// has a stored gateway keeps it, so randomise `clientId` to force a fresh
    /// pick. Pinning this lets a host CSP allowlist a single gateway hostname
    /// instead of `wss://*`.
    #[serde(default)]
    pub preferred_gateway: Option<String>,
    /// Client storage namespace; randomise per session for clean state.
    #[serde(default)]
    pub client_id: Option<String>,
    /// Use `wss://` for gateway connections (default: `true`).
    #[serde(default = "default_force_tls")]
    pub force_tls: bool,
    /// Disable Poisson-distributed dummy traffic (default: `false`).
    #[serde(default)]
    pub disable_poisson_traffic: bool,
    /// Disable cover traffic loop (default: `false`).
    #[serde(default)]
    pub disable_cover_traffic: bool,
    /// SURBs attached to the LP Open frame and the v9 ConnectRequest sent
    /// during the IPR handshake. `None` falls back to [`ipr::SurbsConfig::default`].
    #[serde(default)]
    pub open_reply_surbs: Option<u32>,
    /// SURBs attached to each LP Data frame the bridge sends. Higher values
    /// raise download throughput at the cost of outgoing-packet overhead.
    #[serde(default)]
    pub data_reply_surbs: Option<u32>,
    /// DoH resolver endpoints, tried in order (e.g.
    /// `["https://1.1.1.1/dns-query"]`). Defaults to Cloudflare, Quad9, Google.
    #[serde(default)]
    pub doh_endpoints: Option<Vec<String>>,
    /// Passphrase used to encrypt persistent client storage (identity keys,
    /// gateway details). Omit for plaintext storage. The same passphrase
    /// must be supplied on subsequent page loads to read the same keys.
    #[serde(default)]
    pub storage_passphrase: Option<String>,
    /// IPR connect handshake timeout in milliseconds. Defaults to 60000. Used
    /// for a pinned IPR and as the final budget once auto-discovery rotation is
    /// exhausted.
    #[serde(default)]
    pub connect_timeout_ms: Option<u32>,
    /// Per-version IPR handshake budget in milliseconds (auto-discovery only);
    /// one exit may spend up to two, a v10 probe then a v9 connect, before
    /// rotating. Defaults to 6000.
    #[serde(default)]
    pub ipr_attempt_timeout_ms: Option<u32>,
    /// Maximum IPR candidates to try during auto-discovery rotation. Defaults
    /// to 5.
    #[serde(default)]
    pub ipr_max_attempts: Option<u32>,
    /// DoH query timeout in milliseconds (per endpoint attempt). Defaults to
    /// 8000, sized to cover a cold TLS handshake to the resolver.
    #[serde(default)]
    pub dns_timeout_ms: Option<u32>,
    /// TCP keepalive interval in milliseconds. Defaults to 10000.
    #[serde(default)]
    pub tcp_keepalive_ms: Option<u32>,
    /// Per-TCP-stream RX/TX buffer size in bytes (capped at 65535).
    /// Defaults to 65535.
    #[serde(default)]
    pub tcp_buffer_size: Option<u32>,
    /// Maximum HTTP redirect chain depth before `mixFetch` gives up.
    /// Defaults to 5.
    #[serde(default)]
    pub max_redirects: Option<u8>,
}

#[cfg(target_arch = "wasm32")]
fn default_force_tls() -> bool {
    true
}

/// WASM entry point. Installs the panic hook + state-machine recorder,
/// and flips the runtime debug-log switch on when smolmix's `debug`
/// feature is enabled.
#[wasm_bindgen(start)]
#[cfg(target_arch = "wasm32")]
pub fn main() {
    nym_wasm_utils::set_panic_hook();
    #[cfg(feature = "debug")]
    nym_wasm_utils::set_debug_logging(true);
    state::install_panic_recorder();
}

/// Initialise the mixnet tunnel. See [`SetupOpts`] for the JS-side shape.
#[wasm_bindgen(js_name = "setupMixTunnel")]
#[cfg(target_arch = "wasm32")]
pub fn setup_mix_tunnel(opts: SetupOpts) -> js_sys::Promise {
    future_to_promise(async move {
        let result: Result<JsValue, FetchError> = async move {
            // One-shot: `TUNNEL` is a `OnceLock` so consumers can hold
            // `&'static WasmTunnel` refs without lifetime gymnastics.
            if TUNNEL.get().is_some() {
                return Err(FetchError::Tunnel(
                    "tunnel already initialised; setupMixTunnel can only be called \
                     once per WASM module instance"
                        .into(),
                ));
            }

            let ipr_address: Option<nym_wasm_client_core::Recipient> = opts
                .preferred_ipr
                .map(|s| {
                    s.parse::<nym_wasm_client_core::Recipient>()
                        .map_err(|e| FetchError::Tunnel(format!("invalid IPR address: {e}")))
                })
                .transpose()?;

            let defaults = ipr::SurbsConfig::default();
            let surbs = ipr::SurbsConfig {
                open: opts.open_reply_surbs.unwrap_or(defaults.open),
                data: opts.data_reply_surbs.unwrap_or(defaults.data),
            };

            let doh_endpoints = opts
                .doh_endpoints
                .map(|list| {
                    list.into_iter()
                        .map(|s| {
                            url::Url::parse(&s).map_err(|e| {
                                FetchError::Tunnel(format!("invalid DoH endpoint '{s}': {e}"))
                            })
                        })
                        .collect::<Result<Vec<_>, _>>()
                })
                .transpose()?;

            let mut builder = tunnel::TunnelOpts::builder()
                .client_id(opts.client_id.unwrap_or_else(|| "smolmix-wasm".to_string()))
                .force_tls(opts.force_tls)
                .disable_poisson_traffic(opts.disable_poisson_traffic)
                .disable_cover_traffic(opts.disable_cover_traffic)
                .surbs(surbs);

            if let Some(ipr) = ipr_address {
                builder = builder.ipr_address(ipr);
            }
            if let Some(gw) = opts.preferred_gateway {
                builder = builder.preferred_gateway(gw);
            }
            if let Some(endpoints) = doh_endpoints {
                builder = builder.doh_endpoints(endpoints);
            }
            if let Some(p) = opts.storage_passphrase {
                builder = builder.storage_passphrase(p);
            }
            if let Some(ms) = opts.connect_timeout_ms {
                builder = builder.connect_timeout(std::time::Duration::from_millis(ms as u64));
            }
            if let Some(ms) = opts.ipr_attempt_timeout_ms {
                builder = builder.ipr_attempt_timeout(std::time::Duration::from_millis(ms as u64));
            }
            if let Some(n) = opts.ipr_max_attempts {
                builder = builder.ipr_max_attempts(n as usize);
            }
            if let Some(ms) = opts.dns_timeout_ms {
                builder = builder.dns_timeout(std::time::Duration::from_millis(ms as u64));
            }
            if let Some(ms) = opts.tcp_keepalive_ms {
                builder =
                    builder.tcp_keepalive_interval(std::time::Duration::from_millis(ms as u64));
            }
            if let Some(n) = opts.tcp_buffer_size {
                builder = builder.tcp_buffer_size(n as usize);
            }
            if let Some(n) = opts.max_redirects {
                builder = builder.max_redirects(n);
            }

            let tunnel_opts = builder.build();

            let tun = WasmTunnel::new(tunnel_opts).await?;

            TUNNEL.set(tun).map_err(|_| {
                FetchError::Tunnel(
                    "tunnel already initialised by a concurrent setupMixTunnel call".into(),
                )
            })?;

            Ok(JsValue::UNDEFINED)
        }
        .await;
        result.map_err(Into::into)
    })
}

/// Disconnect from the mixnet. The tunnel becomes unusable until page reload.
#[wasm_bindgen(js_name = "disconnectMixTunnel")]
#[cfg(target_arch = "wasm32")]
pub fn disconnect_mix_tunnel() -> js_sys::Promise {
    future_to_promise(async {
        if let Some(tunnel) = TUNNEL.get() {
            tunnel.shutdown().await;
        }
        Ok(JsValue::UNDEFINED)
    })
}

/// Returns the current tunnel state. The `TunnelState` type is generated from the
/// Rust serde by tsify (see state.rs), so the `.d.ts` and the SDK type stay in
/// step with the runtime. Pre-`setupMixTunnel` reads as `connecting`.
#[wasm_bindgen(js_name = "getTunnelState")]
#[cfg(target_arch = "wasm32")]
pub fn get_tunnel_state() -> TunnelState {
    match TUNNEL.get() {
        Some(tunnel) => tunnel.tunnel_state(),
        None => state::TunnelState::Connecting,
    }
}
