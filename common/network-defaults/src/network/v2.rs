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
        super::NymNetworkDetails::new_mainnet().into()
    }

    pub fn new_sandbox() -> Self {
        sandbox::network_details().into()
    }

    #[cfg(feature = "env")]
    pub fn new_from_env() -> Self {
        super::NymNetworkDetails::new_from_env().into()
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
