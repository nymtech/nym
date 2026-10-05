// Copyright 2026 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

//! Minimal client for the dVPN gateway directory.
//!
//! Bridge parameters (and human monikers) are not carried by the nym-api
//! described-nodes the session selects from; they live in a separate dVPN
//! directory HTTP endpoint. This module fetches that directory once and indexes
//! it by gateway identity so selection can enrich a gateway's name/country and
//! require a bridge-capable entry.
//!
//! Bridge parameters are exposed as `nym_bridges_types::ClientConfig` — the
//! same shared type `nym_bridges`'s `BridgeConn` dials and nym-node-status-api
//! serves — rather than a locally duplicated struct, so this crate doesn't own
//! any bridge-transport-specific fields and picks up new transport kinds
//! automatically as `nym_bridges` implements them.

use std::collections::HashMap;
use std::time::Duration;

use nym_bridges_types::{ClientConfig, PersistedClientConfig};
use serde::Deserialize;

/// Per-gateway directory metadata indexed by base58 identity.
#[derive(Clone, Debug, Default)]
pub(crate) struct DirEntry {
    pub name: Option<String>,
    pub country: Option<String>,
    pub bridge: Option<ClientConfig>,
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
                    bridge: gw.bridges.and_then(|v| {
                        serde_json::from_value::<PersistedClientConfig>(v)
                            .ok()
                            .and_then(first_usable_transport)
                    }),
                },
            );
        }
        Ok(Self { entries })
    }

    /// Directory metadata for a gateway by base58 identity.
    pub(crate) fn entry(&self, identity_base58: &str) -> Option<&DirEntry> {
        self.entries.get(identity_base58)
    }

    /// Whether the gateway advertises a usable bridge transport (any kind).
    pub(crate) fn has_bridge(&self, identity_base58: &str) -> bool {
        self.entries
            .get(identity_base58)
            .is_some_and(|e| e.bridge.is_some())
    }
}

/// The first transport `nym_bridges_types::ClientConfig::is_usable` accepts,
/// regardless of transport kind — a malformed earlier entry (no routable
/// address, or no identity pin) must not shadow a valid later one, and this
/// crate doesn't need to know which transport kinds exist to pick one:
/// `nym_bridges`'s `BridgeConn` is the thing that dispatches on the variant.
fn first_usable_transport(cfg: PersistedClientConfig) -> Option<ClientConfig> {
    cfg.usable_transports().next().cloned()
}

// --- Wire types (only the fields we consume; `bridges` is the shared
// `nym_bridges_types::PersistedClientConfig` shape, parsed leniently so one
// gateway's malformed bridge info can't fail the whole directory fetch). ---

#[derive(Deserialize)]
struct RawGateway {
    identity_key: String,
    #[serde(default)]
    name: Option<String>,
    #[serde(default)]
    location: Option<RawLocation>,
    #[serde(default)]
    bridges: Option<serde_json::Value>,
}

#[derive(Deserialize)]
struct RawLocation {
    #[serde(default)]
    two_letter_iso_country_code: Option<String>,
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Parse a `PersistedClientConfig` JSON body (as served by the dVPN
    /// directory) straight into a [`ClientConfig`], the same path `fetch` uses.
    fn bridge_from_json(json: &str) -> Option<ClientConfig> {
        serde_json::from_str::<PersistedClientConfig>(json)
            .ok()
            .and_then(first_usable_transport)
    }

    #[test]
    fn skips_broken_first_transport() {
        // The first quic_plain has a blank id_pubkey (unusable); a valid later transport must still
        // be selected rather than shadowed.
        let bridge = bridge_from_json(
            r#"{"version":"0","transports":[
                {"transport_type":"quic_plain","args":{"addresses":["1.2.3.4:443"],"host":"a","id_pubkey":""}},
                {"transport_type":"quic_plain","args":{"addresses":["5.6.7.8:443"],"host":"b","id_pubkey":"PINKEY"}}
            ]}"#,
        )
        .expect("a usable bridge transport exists");
        let ClientConfig::QuicPlain(opts) = bridge else {
            panic!("expected quic_plain");
        };
        assert_eq!(opts.id_pubkey, "PINKEY");
        assert_eq!(opts.addresses, vec!["5.6.7.8:443".parse().unwrap()]);
    }

    #[test]
    fn a_padded_value_is_still_usable() {
        // Trimming-for-connect is `nym_bridges`'s job (it decodes/verifies the pin and SNI at
        // dial time); this crate only needs to recognise the transport as usable, unmodified.
        let bridge = bridge_from_json(
            r#"{"version":"0","transports":[
                {"transport_type":"quic_plain","args":{"addresses":["1.2.3.4:443"],"host":"  sni.example  ","id_pubkey":"  PADDED  "}}
            ]}"#,
        )
        .expect("a usable bridge transport exists");
        let ClientConfig::QuicPlain(opts) = bridge else {
            panic!("expected quic_plain");
        };
        assert_eq!(opts.id_pubkey, "  PADDED  ");
        assert_eq!(opts.host.as_deref(), Some("  sni.example  "));
    }

    #[test]
    fn none_when_all_unusable() {
        // No routable address, then a whitespace-only pin — neither is usable.
        assert!(bridge_from_json(
            r#"{"version":"0","transports":[
                {"transport_type":"quic_plain","args":{"addresses":[],"id_pubkey":"X"}},
                {"transport_type":"quic_plain","args":{"addresses":["1.2.3.4:443"],"id_pubkey":"   "}}
            ]}"#,
        )
        .is_none());
    }

    #[test]
    fn any_usable_transport_kind_is_accepted() {
        // A `tls_plain` transport is picked up too — this crate doesn't special-case `quic_plain`;
        // whichever transport `nym_bridges`'s `BridgeConn` can dial is fine.
        let bridge = bridge_from_json(
            r#"{"version":"0","transports":[{"transport_type":"tls_plain","args":{"addresses":["1.2.3.4:443"],"id_pubkey":"K"}}]}"#,
        )
        .expect("a usable bridge transport exists");
        assert!(matches!(bridge, ClientConfig::TlsPlain(_)));
    }

    #[test]
    fn unrecognised_transport_type_makes_the_whole_config_unparseable() {
        // `nym_bridges_types::ClientConfig` is a closed enum (`quic_plain` | `tls_plain`): unlike
        // an ad hoc parser, an entirely unknown transport type fails deserialization of the
        // *whole* config, not just that one entry. `fetch` treats that failure the same as "no
        // bridge info" for the gateway — accepted trade-off for sharing the canonical wire type
        // instead of hand-rolling a lenient one.
        assert!(bridge_from_json(
            r#"{"version":"0","transports":[{"transport_type":"obfs4","args":{"addresses":["1.2.3.4:443"],"id_pubkey":"K"}}]}"#,
        )
        .is_none());
    }
}
