// Copyright 2024 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

//! V2 of [`super::NymNetworkDetails`], grouping the api-url configuration into a single
//! [`NetworkingSpecifics`] block and adding room for DNS fallback configuration.
//!
//! Build one of these via `.into()` from an existing [`super::NymNetworkDetails`] (or
//! `super::NymNetworkDetails::new_mainnet().into()`, etc.) rather than constructing it
//! from scratch, so that all of the existing builder methods on the v1 type keep working.

use super::{ApiUrl, ChainDetails, NymContracts, ValidatorDetails};
use crate::sandbox;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

use std::collections::HashMap;
use std::net::IpAddr;

// `#[schema(as = ...)]` gives these a distinct OpenAPI component name from their v1
// namesakes (`nym_network_defaults::NymNetworkDetails` et al.) - without it, utoipa would
// register both under the bare struct name and one would silently clobber the other.
#[derive(Clone, Debug, Deserialize, Eq, Hash, PartialEq, Serialize, JsonSchema)]
#[cfg_attr(feature = "utoipa", derive(utoipa::ToSchema))]
#[cfg_attr(feature = "utoipa", schema(as = network::v2::NymNetworkDetails))]
pub struct NymNetworkDetails {
    pub network_name: String,
    pub chain_details: ChainDetails,
    pub endpoints: Vec<ValidatorDetails>,
    pub contracts: NymContracts,
    pub networking: NetworkingSpecifics,
}

#[derive(Clone, Debug, Hash, Serialize, Deserialize, PartialEq, Eq, JsonSchema)]
#[cfg_attr(feature = "utoipa", derive(utoipa::ToSchema))]
#[cfg_attr(feature = "utoipa", schema(as = network::v2::NetworkingSpecifics))]
pub struct NetworkingSpecifics {
    pub nym_api_urls: Vec<ApiUrl>,
    pub nym_vpn_api_urls: Vec<ApiUrl>,
    pub dns_fallbacks: Vec<DnsFallback>,
    // pub internal_nameservers: std::any::Any,
    // pub covert channels: std::any::Any,
}

#[derive(Clone, Debug, Hash, Serialize, Deserialize, PartialEq, Eq, JsonSchema)]
#[cfg_attr(feature = "utoipa", derive(utoipa::ToSchema))]
#[cfg_attr(feature = "utoipa", schema(as = network::v2::DnsFallback))]
pub struct DnsFallback {
    pub url: String,
    pub addresses: Vec<String>,
}

/// Pins for mainnet's own domains (nym-api, nym-vpn-api) plus the domain-fronting
/// hosts they can hide behind, taken from [`crate::dns::default_static_addrs`] - the
/// same table the DNS resolver falls back to when regular resolution is untrustworthy.
fn dns_fallbacks(raw: HashMap<String, Vec<IpAddr>>) -> Vec<DnsFallback> {
    let mut fallbacks: Vec<DnsFallback> = raw
        .into_iter()
        .map(|(url, addresses)| DnsFallback {
            url,
            addresses: addresses.iter().map(ToString::to_string).collect(),
        })
        .collect();
    fallbacks.sort_by(|a, b| a.url.cmp(&b.url));
    fallbacks
}

/// Reads `dns_fallbacks` from the [`crate::var_names::DNS_FALLBACKS`] env var, as a
/// JSON-encoded `Vec<DnsFallback>`. If unset (or empty), falls back to
/// [`mainnet_dns_fallbacks`] when `network_name` is mainnet's - there are no compiled-in pins
/// for any other network yet - otherwise it's simply empty.
#[cfg(feature = "env")]
fn dns_fallbacks_from_env(network_name: &str) -> Vec<DnsFallback> {
    use crate::var_names;
    use std::env::var;

    match var(var_names::DNS_FALLBACKS) {
        Ok(raw) if !raw.is_empty() => serde_json::from_str(&raw).unwrap_or_else(|e| {
            panic!(
                "{} is set but could not be parsed: {e}",
                var_names::DNS_FALLBACKS
            )
        }),
        _ if network_name == crate::mainnet::NETWORK_NAME => {
            dns_fallbacks(crate::mainnet::dns::default_static_addrs())
        }
        _ if network_name == crate::sandbox::NETWORK_NAME => {
            dns_fallbacks(crate::sandbox::dns::default_static_addrs())
        }
        _ => Vec::new(),
    }
}

#[cfg(feature = "env")]
fn serialize_dns_fallbacks(fallbacks: &[DnsFallback]) -> String {
    serde_json::to_string(fallbacks)
        .inspect_err(|e| tracing::warn!("failed to serialize dns_fallbacks for env: {e}"))
        .unwrap_or_default()
}

// by default we assume the same defaults as mainnet, i.e. same prefixes and denoms
impl Default for NymNetworkDetails {
    fn default() -> Self {
        NymNetworkDetails::new_mainnet()
    }
}

/// Converts from the existing (v1) network details, deriving `networking` from its
/// `nym_api_urls()` / `nym_vpn_api_urls()` accessors. There's no DNS fallback data on v1,
/// so `dns_fallbacks` always starts out empty.
impl From<super::NymNetworkDetails> for NymNetworkDetails {
    fn from(v1: super::NymNetworkDetails) -> Self {
        NymNetworkDetails {
            networking: NetworkingSpecifics {
                nym_api_urls: v1.nym_api_urls(),
                nym_vpn_api_urls: v1.nym_vpn_api_urls(),
                dns_fallbacks: Vec::new(),
            },
            network_name: v1.network_name,
            chain_details: v1.chain_details,
            endpoints: v1.endpoints,
            contracts: v1.contracts,
        }
    }
}

/// Converts back down to the v1 shape for callers that aren't ready to consume the new
/// fields yet. `dns_fallbacks` is dropped, since v1 has nowhere to put it.
impl From<NymNetworkDetails> for super::NymNetworkDetails {
    fn from(v2: NymNetworkDetails) -> Self {
        let nym_vpn_api_url = v2
            .networking
            .nym_vpn_api_urls
            .first()
            .map(|url| url.url.clone());

        super::NymNetworkDetails {
            network_name: v2.network_name,
            chain_details: v2.chain_details,
            endpoints: v2.endpoints,
            contracts: v2.contracts,
            nym_api_urls: (!v2.networking.nym_api_urls.is_empty())
                .then_some(v2.networking.nym_api_urls),
            nym_vpn_api_urls: (!v2.networking.nym_vpn_api_urls.is_empty())
                .then_some(v2.networking.nym_vpn_api_urls),
            nym_vpn_api_url,
        }
    }
}

impl NymNetworkDetails {
    pub fn new_empty() -> Self {
        super::NymNetworkDetails::new_empty().into()
    }

    pub fn new_mainnet() -> Self {
        let mut base: NymNetworkDetails = super::NymNetworkDetails::new_mainnet().into();
        base.networking.dns_fallbacks = dns_fallbacks(crate::mainnet::dns::default_static_addrs());
        base
    }

    pub fn new_sandbox() -> Self {
        sandbox::network_details().into()
    }

    #[cfg(feature = "env")]
    pub fn new_from_env() -> Self {
        let mut details: NymNetworkDetails = super::NymNetworkDetails::new_from_env().into();
        details.networking.dns_fallbacks = dns_fallbacks_from_env(&details.network_name);
        details
    }

    /// Exports the v1-shared fields via [`super::NymNetworkDetails::export_to_env`], plus
    /// `networking.dns_fallbacks` (as JSON) to [`crate::var_names::DNS_FALLBACKS`] - mirroring
    /// how [`Self::new_from_env`] reads it back. Leaves `DNS_FALLBACKS` untouched if
    /// `dns_fallbacks` is empty, same as how the v1 exporter skips unset optional fields.
    #[cfg(feature = "env")]
    pub fn export_to_env(self) {
        use crate::var_names;
        use std::env::set_var;

        let dns_fallbacks = self.networking.dns_fallbacks.clone();
        let v1: super::NymNetworkDetails = self.into();
        v1.export_to_env();

        if !dns_fallbacks.is_empty() {
            unsafe {
                set_var(
                    var_names::DNS_FALLBACKS,
                    serialize_dns_fallbacks(&dns_fallbacks),
                )
            }
        }
    }

    #[must_use]
    pub fn with_dns_fallbacks(mut self, dns_fallbacks: Vec<DnsFallback>) -> Self {
        self.networking.dns_fallbacks = dns_fallbacks;
        self
    }

    pub fn set_nym_api_urls<U: Into<ApiUrl>>(&mut self, urls: Vec<U>) {
        self.networking.nym_api_urls = urls.into_iter().map(Into::into).collect();
    }

    #[must_use]
    pub fn with_nym_api_urls<U: Into<ApiUrl>>(mut self, urls: Vec<U>) -> Self {
        self.set_nym_api_urls(urls);
        self
    }

    pub fn set_nym_vpn_api_urls<U: Into<ApiUrl>>(&mut self, urls: Vec<U>) {
        self.networking.nym_vpn_api_urls = urls.into_iter().map(Into::into).collect();
    }

    #[must_use]
    pub fn with_nym_vpn_api_urls<U: Into<ApiUrl>>(mut self, urls: Vec<U>) -> Self {
        self.set_nym_vpn_api_urls(urls);
        self
    }

    pub fn nym_api_urls(&self) -> Vec<ApiUrl> {
        self.networking.nym_api_urls.clone()
    }

    pub fn nym_vpn_api_urls(&self) -> Vec<ApiUrl> {
        self.networking.nym_vpn_api_urls.clone()
    }

    pub fn dns_fallbacks(&self) -> Vec<DnsFallback> {
        self.networking.dns_fallbacks.clone()
    }
}

#[cfg(all(test, feature = "env"))]
mod tests {
    use super::*;
    use crate::var_names;
    use std::sync::Mutex;

    // env vars are process-global and cargo runs tests in this file on separate threads of
    // the same process, so serialize access to `DNS_FALLBACKS` and leave it exactly as each
    // test found it.
    static DNS_FALLBACKS_ENV_LOCK: Mutex<()> = Mutex::new(());

    fn with_dns_fallbacks_var_cleared(test: impl FnOnce()) {
        let _guard = DNS_FALLBACKS_ENV_LOCK.lock().unwrap();

        let previous = std::env::var(var_names::DNS_FALLBACKS).ok();
        unsafe { std::env::remove_var(var_names::DNS_FALLBACKS) };

        test();

        unsafe {
            match &previous {
                Some(value) => std::env::set_var(var_names::DNS_FALLBACKS, value),
                None => std::env::remove_var(var_names::DNS_FALLBACKS),
            }
        }
    }

    #[test]
    fn dns_fallbacks_from_env_defaults_to_empty_when_unset_on_non_mainnet() {
        with_dns_fallbacks_var_cleared(|| {
            assert_eq!(dns_fallbacks_from_env(sandbox::NETWORK_NAME), Vec::new());
        });
    }

    #[test]
    fn dns_fallbacks_from_env_falls_back_to_mainnet_pins_when_unset() {
        with_dns_fallbacks_var_cleared(|| {
            assert_eq!(
                dns_fallbacks_from_env(crate::mainnet::NETWORK_NAME),
                dns_fallbacks(crate::mainnet::dns::default_static_addrs())
            );
        });
    }

    #[test]
    fn dns_fallbacks_round_trip_through_export_and_env() {
        with_dns_fallbacks_var_cleared(|| {
            let fallbacks = vec![
                DnsFallback {
                    url: "example.com".to_string(),
                    addresses: vec!["1.2.3.4".to_string(), "::1".to_string()],
                },
                DnsFallback {
                    url: "other.example.com".to_string(),
                    addresses: vec!["5.6.7.8".to_string()],
                },
            ];

            // new_mainnet() (rather than new_empty()) so the v1-shared fields it exports
            // alongside dns_fallbacks (NYXD, NETWORK_NAME, ...) are non-empty, letting
            // new_from_env() read the whole struct back without panicking on a missing var.
            NymNetworkDetails::new_mainnet()
                .with_dns_fallbacks(fallbacks.clone())
                .export_to_env();

            assert_eq!(
                dns_fallbacks_from_env(crate::mainnet::NETWORK_NAME),
                fallbacks
            );
            assert_eq!(
                NymNetworkDetails::new_from_env().networking.dns_fallbacks,
                fallbacks
            );
        });
    }
}
