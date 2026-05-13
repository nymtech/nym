// Copyright 2024 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

//! smolmix-wasm: drop-in browser networking over the Nym mixnet.
//!
//! Exposes three APIs that mirror the browser's native networking surface:
//!
//! - **`mixFetch(url, init)`**: drop-in `fetch()` replacement (HTTP/HTTPS)
//! - **`mixSocket(url, protocols, onEvent)`**: drop-in `WebSocket` replacement (WS/WSS)
//! - **`mixResolve(hostname)`**: DNS-only hostname lookup (UDP / IPR path, no TCP/TLS)
//!
//! All three share the same mixnet tunnel (DNS, TCP, TLS), initialised once
//! via `setupMixTunnel(opts)` and torn down with `disconnectMixTunnel()`.

// All modules gated on wasm32 so `cargo check` on the host triple sees an empty crate.
#[cfg(target_arch = "wasm32")]
mod bridge;
#[cfg(target_arch = "wasm32")]
mod device;
#[cfg(target_arch = "wasm32")]
mod dns;
#[cfg(target_arch = "wasm32")]
mod error;
#[cfg(target_arch = "wasm32")]
mod fetch;
#[cfg(target_arch = "wasm32")]
mod http;
#[cfg(target_arch = "wasm32")]
mod ipr;
#[cfg(target_arch = "wasm32")]
mod mixdns;
#[cfg(target_arch = "wasm32")]
mod mixfetch;
#[cfg(target_arch = "wasm32")]
mod mixsocket;
#[cfg(target_arch = "wasm32")]
mod reactor;
#[cfg(target_arch = "wasm32")]
mod stream;
#[cfg(target_arch = "wasm32")]
mod tls;
#[cfg(target_arch = "wasm32")]
mod tunnel;
#[cfg(target_arch = "wasm32")]
mod util;

#[cfg(target_arch = "wasm32")]
pub use error::FetchError;
#[cfg(target_arch = "wasm32")]
pub use tunnel::WasmTunnel;

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

/// Options accepted by `setupMixTunnel`. Deserialised from the JS object via
/// `serde-wasm-bindgen` + `tsify`, which gives us typed access without manual
/// `Reflect::get` plumbing and emits a matching `.d.ts` for the TS side.
#[derive(Tsify, Deserialize)]
#[tsify(from_wasm_abi)]
#[serde(rename_all = "camelCase")]
#[cfg(target_arch = "wasm32")]
pub struct SetupOpts {
    /// Nym address of the IPR exit node (required).
    pub preferred_ipr: String,
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
}

#[cfg(target_arch = "wasm32")]
fn default_force_tls() -> bool {
    true
}

/// WASM entry point. Installs the panic hook for readable stack traces.
#[wasm_bindgen(start)]
#[cfg(target_arch = "wasm32")]
pub fn main() {
    nym_wasm_utils::set_panic_hook();
}

/// Initialise the mixnet tunnel. See [`SetupOpts`] for the JS-side shape.
#[wasm_bindgen(js_name = "setupMixTunnel")]
#[cfg(target_arch = "wasm32")]
pub fn setup_mix_tunnel(opts: SetupOpts) -> js_sys::Promise {
    future_to_promise(async move {
        let result: Result<JsValue, FetchError> = async move {
            let ipr_address: nym_wasm_client_core::Recipient = opts
                .preferred_ipr
                .parse()
                .map_err(|e| FetchError::Tunnel(format!("invalid IPR address: {e}")))?;

            let defaults = ipr::SurbsConfig::default();
            let surbs = ipr::SurbsConfig {
                open: opts.open_reply_surbs.unwrap_or(defaults.open),
                data: opts.data_reply_surbs.unwrap_or(defaults.data),
            };

            let tunnel_opts = tunnel::TunnelOpts {
                ipr_address,
                client_id: opts.client_id.unwrap_or_else(|| "smolmix-wasm".to_string()),
                force_tls: opts.force_tls,
                disable_poisson_traffic: opts.disable_poisson_traffic,
                disable_cover_traffic: opts.disable_cover_traffic,
                surbs,
            };

            let tun = WasmTunnel::new(tunnel_opts).await?;

            TUNNEL
                .set(tun)
                .map_err(|_| FetchError::Tunnel("tunnel already initialised".into()))?;

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
