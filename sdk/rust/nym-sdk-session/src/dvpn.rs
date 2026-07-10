// Copyright 2026 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

//! Minimal client for the dVPN gateway directory.
//!
//! QUIC bridge parameters (and human monikers) are not carried by the nym-api
//! described-nodes the session selects from; they live in a separate dVPN
//! directory HTTP endpoint. This module fetches that directory once and indexes
//! it by gateway identity so selection can enrich a gateway's name/country and
//! require a QUIC-bridge-capable entry.

use std::collections::HashMap;
use std::net::SocketAddr;
use std::time::Duration;

use serde::Deserialize;

/// QUIC bridge connection parameters for a gateway, sourced from the dVPN
/// directory. The datapath consumes these to front the WireGuard entry leg with
/// a QUIC bridge.
#[derive(Clone, Debug)]
pub struct QuicBridge {
    /// Candidate bridge socket addresses.
    pub addresses: Vec<SocketAddr>,
    /// SNI host to present to the bridge (if advertised).
    pub sni_host: Option<String>,
    /// Base64-encoded ed25519 identity public key the bridge cert is pinned to.
    pub id_pubkey_base64: String,
}

/// Per-gateway directory metadata indexed by base58 identity.
#[derive(Clone, Debug, Default)]
pub(crate) struct DirEntry {
    pub name: Option<String>,
    pub country: Option<String>,
    pub quic: Option<QuicBridge>,
}

/// The fetched dVPN directory, indexed by base58 gateway identity.
#[derive(Clone, Debug, Default)]
pub(crate) struct DvpnDirectory {
    entries: HashMap<String, DirEntry>,
}

impl DvpnDirectory {
    /// Fetch and index the directory at `url`. Errors are the caller's to treat
    /// as best-effort (an empty directory is a valid fallback).
    pub(crate) async fn fetch(url: &str) -> Result<Self, String> {
        let client = reqwest::Client::builder()
            .timeout(Duration::from_secs(30))
            .build()
            .map_err(|e| e.to_string())?;
        let raw: Vec<RawGateway> = client
            .get(url)
            .send()
            .await
            .map_err(|e| e.to_string())?
            .error_for_status()
            .map_err(|e| e.to_string())?
            .json()
            .await
            .map_err(|e| e.to_string())?;

        let mut entries = HashMap::with_capacity(raw.len());
        for gw in raw {
            entries.insert(
                gw.identity_key.clone(),
                DirEntry {
                    name: gw.name.filter(|n| !n.is_empty() && n != "N/A"),
                    country: gw
                        .location
                        .and_then(|l| l.two_letter_iso_country_code)
                        .filter(|c| !c.is_empty()),
                    quic: gw.bridges.and_then(|b| b.into_quic()),
                },
            );
        }
        Ok(Self { entries })
    }

    /// Directory metadata for a gateway by base58 identity.
    pub(crate) fn entry(&self, identity_base58: &str) -> Option<&DirEntry> {
        self.entries.get(identity_base58)
    }

    /// Whether the gateway advertises a QUIC bridge.
    pub(crate) fn has_quic(&self, identity_base58: &str) -> bool {
        self.entries
            .get(identity_base58)
            .is_some_and(|e| e.quic.is_some())
    }
}

// --- Wire types (only the fields we consume). ---

#[derive(Deserialize)]
struct RawGateway {
    identity_key: String,
    #[serde(default)]
    name: Option<String>,
    #[serde(default)]
    location: Option<RawLocation>,
    #[serde(default)]
    bridges: Option<RawBridges>,
}

#[derive(Deserialize)]
struct RawLocation {
    #[serde(default)]
    two_letter_iso_country_code: Option<String>,
}

#[derive(Deserialize)]
struct RawBridges {
    #[serde(default)]
    transports: Vec<RawTransport>,
}

impl RawBridges {
    /// The first *usable* `quic_plain` transport, parsed into a [`QuicBridge`].
    fn into_quic(self) -> Option<QuicBridge> {
        // Consider every `quic_plain` transport and return the first that yields a usable bridge — a
        // malformed earlier entry (no routable address, or no identity pin) must not shadow a valid
        // later one.
        self.transports
            .into_iter()
            .filter(|t| t.transport_type == "quic_plain")
            .find_map(|t| {
                let args = t.args?;
                let addresses = args
                    .addresses
                    .iter()
                    .filter_map(|a| a.parse::<SocketAddr>().ok())
                    .collect::<Vec<_>>();
                if addresses.is_empty() {
                    return None;
                }
                // The identity pin is what the bridge's certificate is verified against, so a bridge
                // without one is unusable — don't advertise it as QUIC-capable. Trim surrounding
                // whitespace so a padded directory value still base64-decodes at connect time.
                let id_pubkey_base64 = args.id_pubkey.trim().to_string();
                if id_pubkey_base64.is_empty() {
                    return None;
                }
                let sni_host = args
                    .host
                    .map(|h| h.trim().to_string())
                    .filter(|h| !h.is_empty());
                Some(QuicBridge {
                    addresses,
                    sni_host,
                    id_pubkey_base64,
                })
            })
    }
}

#[derive(Deserialize)]
struct RawTransport {
    transport_type: String,
    #[serde(default)]
    args: Option<RawQuicArgs>,
}

#[derive(Deserialize)]
struct RawQuicArgs {
    #[serde(default)]
    addresses: Vec<String>,
    #[serde(default)]
    host: Option<String>,
    id_pubkey: String,
}

#[cfg(test)]
mod tests {
    use super::*;

    fn bridges(json: &str) -> RawBridges {
        serde_json::from_str(json).expect("valid RawBridges json")
    }

    #[test]
    fn into_quic_skips_broken_first_transport() {
        // The first quic_plain has a blank id_pubkey (unusable); a valid later transport must still
        // be selected rather than shadowed.
        let quic = bridges(
            r#"{"transports":[
                {"transport_type":"quic_plain","args":{"addresses":["1.2.3.4:443"],"host":"a","id_pubkey":""}},
                {"transport_type":"quic_plain","args":{"addresses":["5.6.7.8:443"],"host":"b","id_pubkey":"PINKEY"}}
            ]}"#,
        )
        .into_quic()
        .expect("a usable quic bridge exists");
        assert_eq!(quic.id_pubkey_base64, "PINKEY");
        assert_eq!(quic.addresses, vec!["5.6.7.8:443".parse().unwrap()]);
    }

    #[test]
    fn into_quic_trims_id_pubkey_and_host() {
        let quic = bridges(
            r#"{"transports":[
                {"transport_type":"quic_plain","args":{"addresses":["1.2.3.4:443"],"host":"  sni.example  ","id_pubkey":"  PADDED  "}}
            ]}"#,
        )
        .into_quic()
        .expect("a usable quic bridge exists");
        assert_eq!(quic.id_pubkey_base64, "PADDED");
        assert_eq!(quic.sni_host.as_deref(), Some("sni.example"));
    }

    #[test]
    fn into_quic_none_when_all_unusable() {
        // No routable address, then a whitespace-only pin — neither is usable.
        assert!(bridges(
            r#"{"transports":[
                {"transport_type":"quic_plain","args":{"addresses":[],"id_pubkey":"X"}},
                {"transport_type":"quic_plain","args":{"addresses":["1.2.3.4:443"],"id_pubkey":"   "}}
            ]}"#,
        )
        .into_quic()
        .is_none());
    }

    #[test]
    fn into_quic_ignores_non_quic_transports() {
        assert!(bridges(
            r#"{"transports":[{"transport_type":"other","args":{"addresses":["1.2.3.4:443"],"id_pubkey":"K"}}]}"#,
        )
        .into_quic()
        .is_none());
    }
}
