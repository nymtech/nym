// Copyright 2024 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

//! V2 of [`v1::NymNetworkDetails`], grouping the api-url configuration into a single
//! [`NetworkingSpecifics`] block and adding room for DNS fallback configuration.
//!
//! This is now the canonical representation: `new_empty`/`new_mainnet`/`new_sandbox`/
//! `new_from_env`/`export_to_env` are all implemented here, directly against the
//! `mainnet`/`sandbox` consts and env vars. [`v1::NymNetworkDetails`] (v1) is derived
//! from this one via `.into()` (see the `From` impls below) rather than the other way
//! around, so parsing/exporting only needs to be correct in one place.

use crate::GAS_PRICE_AMOUNT;
use crate::network::{ApiUrl, ChainDetails, DenomDetailsOwned, NymContracts, ValidatorDetails};
use crate::{mainnet, sandbox, v1};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

use std::collections::HashMap;
use std::net::IpAddr;

#[cfg(feature = "env")]
use std::env::{VarError, var};
#[cfg(feature = "env")]
use std::ffi::OsStr;
#[cfg(feature = "env")]
use url::Url;

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

pub(crate) fn dns_fallbacks(raw: HashMap<String, Vec<IpAddr>>) -> Vec<DnsFallback> {
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
/// JSON-encoded `Vec<DnsFallback>`. If unset (or empty), falls back to the compiled-in
/// `mainnet`/`sandbox` pins matching `network_name` - there are none for any other network
/// yet, so it's simply empty in that case.
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
        _ if network_name == mainnet::NETWORK_NAME => {
            dns_fallbacks(mainnet::dns::default_static_addrs())
        }
        _ if network_name == sandbox::NETWORK_NAME => {
            dns_fallbacks(sandbox::dns::default_static_addrs())
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

/// `""` is treated the same as unset - mirrors the legacy env-var convention used
/// throughout `mainnet.rs`/`sandbox.rs` for "this network has no contract deployed yet".
fn parse_optional_str(raw: &str) -> Option<String> {
    (!raw.is_empty()).then(|| raw.to_string())
}

#[cfg(feature = "env")]
fn serialize_api_urls(urls: &[ApiUrl]) -> Option<String> {
    serde_json::to_string(urls)
        .inspect_err(|e| tracing::warn!("failed to serialize api urls for env: {e}"))
        .ok()
}

#[cfg(feature = "env")]
fn try_parse_api_urls(k: impl AsRef<OsStr>) -> Result<Vec<ApiUrl>, serde_json::Error> {
    match var(k) {
        Ok(raw) if !raw.is_empty() => serde_json::from_str(&raw),
        _ => Ok(Vec::new()),
    }
}

#[cfg(feature = "env")]
fn get_optional_env<K: AsRef<OsStr>>(env: K) -> Option<String> {
    match var(env) {
        Ok(var) => (!var.is_empty()).then_some(var),
        Err(VarError::NotPresent) => None,
        err => panic!("Unable to set: {err:?}"),
    }
}

// NYM_APIS was introduced to replace the singular NYM_API; fall back to it so
// setups that only ever configured NYM_API (via setup_env's legacy migration
// or otherwise) keep working.
#[cfg(feature = "env")]
fn parse_legacy_nym_api() -> ApiUrl {
    use crate::var_names;

    let legacy_api = get_optional_env(var_names::NYM_API).unwrap_or_else(|| {
        panic!(
            "neither {} nor legacy {} is set",
            var_names::NYM_APIS,
            var_names::NYM_API
        )
    });
    Url::parse(&legacy_api)
        .unwrap_or_else(|e| panic!("{} is not a valid url: {e}", var_names::NYM_API))
        .into()
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
impl From<v1::NymNetworkDetails> for NymNetworkDetails {
    fn from(v1: v1::NymNetworkDetails) -> Self {
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
impl From<NymNetworkDetails> for v1::NymNetworkDetails {
    fn from(v2: NymNetworkDetails) -> Self {
        let nym_vpn_api_url = v2
            .networking
            .nym_vpn_api_urls
            .first()
            .map(|url| url.url.clone());

        v1::NymNetworkDetails {
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
        NymNetworkDetails {
            network_name: Default::default(),
            chain_details: ChainDetails {
                bech32_account_prefix: Default::default(),
                mix_denom: DenomDetailsOwned {
                    base: Default::default(),
                    display: Default::default(),
                    display_exponent: Default::default(),
                },
                stake_denom: DenomDetailsOwned {
                    base: Default::default(),
                    display: Default::default(),
                    display_exponent: Default::default(),
                },
            },
            endpoints: Default::default(),
            contracts: Default::default(),
            networking: NetworkingSpecifics {
                nym_api_urls: Vec::new(),
                nym_vpn_api_urls: Vec::new(),
                dns_fallbacks: Vec::new(),
            },
        }
    }

    pub fn new_mainnet() -> Self {
        NymNetworkDetails {
            network_name: mainnet::NETWORK_NAME.into(),
            chain_details: ChainDetails::mainnet(),
            endpoints: mainnet::validators(),
            contracts: NymContracts {
                mixnet_contract_address: parse_optional_str(mainnet::MIXNET_CONTRACT_ADDRESS),
                vesting_contract_address: parse_optional_str(mainnet::VESTING_CONTRACT_ADDRESS),
                performance_contract_address: parse_optional_str(
                    mainnet::PERFORMANCE_CONTRACT_ADDRESS,
                ),
                network_monitors_contract_address: parse_optional_str(
                    mainnet::NETWORK_MONITORS_CONTRACT_ADDRESS,
                ),
                node_families_contract_address: parse_optional_str(
                    mainnet::NODE_FAMILIES_CONTRACT_ADDRESS,
                ),
                ecash_contract_address: parse_optional_str(mainnet::ECASH_CONTRACT_ADDRESS),
                group_contract_address: parse_optional_str(mainnet::GROUP_CONTRACT_ADDRESS),
                multisig_contract_address: parse_optional_str(mainnet::MULTISIG_CONTRACT_ADDRESS),
                coconut_dkg_contract_address: parse_optional_str(
                    mainnet::COCONUT_DKG_CONTRACT_ADDRESS,
                ),
            },
            networking: NetworkingSpecifics {
                nym_api_urls: mainnet::NYM_APIS.iter().copied().map(Into::into).collect(),
                nym_vpn_api_urls: mainnet::NYM_VPN_APIS
                    .iter()
                    .copied()
                    .map(Into::into)
                    .collect(),
                dns_fallbacks: dns_fallbacks(mainnet::dns::default_static_addrs()),
            },
        }
    }

    pub fn new_sandbox() -> Self {
        NymNetworkDetails {
            network_name: sandbox::NETWORK_NAME.into(),
            chain_details: ChainDetails {
                bech32_account_prefix: sandbox::BECH32_PREFIX.to_string(),
                mix_denom: sandbox::MIX_DENOM.into(),
                stake_denom: sandbox::STAKE_DENOM.into(),
            },
            endpoints: sandbox::validators(),
            contracts: NymContracts {
                mixnet_contract_address: parse_optional_str(sandbox::MIXNET_CONTRACT_ADDRESS),
                vesting_contract_address: parse_optional_str(sandbox::VESTING_CONTRACT_ADDRESS),
                performance_contract_address: parse_optional_str(
                    sandbox::PERFORMANCE_CONTRACT_ADDRESS,
                ),
                network_monitors_contract_address: parse_optional_str(
                    sandbox::NETWORK_MONITORS_CONTRACT_ADDRESS,
                ),
                node_families_contract_address: parse_optional_str(
                    sandbox::NODE_FAMILIES_CONTRACT_ADDRESS,
                ),
                ecash_contract_address: parse_optional_str(sandbox::ECASH_CONTRACT_ADDRESS),
                group_contract_address: parse_optional_str(sandbox::GROUP_CONTRACT_ADDRESS),
                multisig_contract_address: parse_optional_str(sandbox::MULTISIG_CONTRACT_ADDRESS),
                coconut_dkg_contract_address: parse_optional_str(
                    sandbox::COCONUT_DKG_CONTRACT_ADDRESS,
                ),
            },
            networking: NetworkingSpecifics {
                nym_api_urls: sandbox::NYM_APIS.iter().copied().map(Into::into).collect(),
                nym_vpn_api_urls: sandbox::NYM_VPN_APIS
                    .iter()
                    .copied()
                    .map(Into::into)
                    .collect(),
                dns_fallbacks: dns_fallbacks(sandbox::dns::default_static_addrs()),
            },
        }
    }

    #[cfg(feature = "env")]
    pub fn new_from_env() -> Self {
        use crate::var_names;

        let nym_api_urls = try_parse_api_urls(var_names::NYM_APIS).unwrap_or_else(|e| {
            panic!(
                "{} is set but could not be parsed: {e}",
                var_names::NYM_APIS
            )
        });
        let nym_api_urls = if nym_api_urls.is_empty() {
            vec![parse_legacy_nym_api()]
        } else {
            nym_api_urls
        };
        let nym_api = nym_api_urls
            .first()
            .expect("nym_api_urls is guaranteed non-empty at this point");
        let nym_vpn_api_urls = try_parse_api_urls(var_names::NYM_VPN_APIS).unwrap_or_else(|e| {
            panic!(
                "{} is set but could not be parsed: {e}",
                var_names::NYM_VPN_APIS
            )
        });

        let network_name = var(var_names::NETWORK_NAME).expect("network name not set");
        let dns_fallbacks = dns_fallbacks_from_env(&network_name);

        NymNetworkDetails {
            network_name,
            chain_details: ChainDetails {
                bech32_account_prefix: var(var_names::BECH32_PREFIX)
                    .expect("bech32 prefix not set"),
                mix_denom: DenomDetailsOwned {
                    base: var(var_names::MIX_DENOM).expect("mix denomination base not set"),
                    display: var(var_names::MIX_DENOM_DISPLAY)
                        .expect("mix denomination display not set"),
                    display_exponent: var(var_names::DENOMS_EXPONENT)
                        .expect("denomination exponent not set")
                        .parse()
                        .expect("denomination exponent is not u32"),
                },
                stake_denom: DenomDetailsOwned {
                    base: var(var_names::STAKE_DENOM).expect("stake denomination base not set"),
                    display: var(var_names::STAKE_DENOM_DISPLAY)
                        .expect("stake denomination display not set"),
                    display_exponent: var(var_names::DENOMS_EXPONENT)
                        .expect("denomination exponent not set")
                        .parse()
                        .expect("denomination exponent is not u32"),
                },
            },
            endpoints: vec![ValidatorDetails::new(
                var(var_names::NYXD).expect("nyxd validator not set"),
                Some(nym_api.url.clone()),
                get_optional_env(var_names::NYXD_WEBSOCKET),
            )],
            contracts: NymContracts {
                mixnet_contract_address: get_optional_env(var_names::MIXNET_CONTRACT_ADDRESS),
                vesting_contract_address: get_optional_env(var_names::VESTING_CONTRACT_ADDRESS),
                performance_contract_address: get_optional_env(
                    var_names::PERFORMANCE_CONTRACT_ADDRESS,
                ),
                network_monitors_contract_address: get_optional_env(
                    var_names::NETWORK_MONITORS_CONTRACT_ADDRESS,
                ),
                node_families_contract_address: get_optional_env(
                    var_names::NODE_FAMILIES_CONTRACT_ADDRESS,
                ),
                ecash_contract_address: get_optional_env(var_names::ECASH_CONTRACT_ADDRESS),
                group_contract_address: get_optional_env(var_names::GROUP_CONTRACT_ADDRESS),
                multisig_contract_address: get_optional_env(var_names::MULTISIG_CONTRACT_ADDRESS),
                coconut_dkg_contract_address: get_optional_env(
                    var_names::COCONUT_DKG_CONTRACT_ADDRESS,
                ),
            },
            networking: NetworkingSpecifics {
                nym_api_urls,
                nym_vpn_api_urls,
                dns_fallbacks,
            },
        }
    }

    /// Exports every field to its env var (mirroring [`Self::new_from_env`]'s reads),
    /// including `networking.dns_fallbacks` (as JSON) to
    /// [`crate::var_names::DNS_FALLBACKS`]. Leaves `DNS_FALLBACKS` untouched if
    /// `dns_fallbacks` is empty, same as how optional contract addresses are skipped
    /// when unset.
    #[rustfmt::skip]
    #[cfg(feature = "env")]
    pub fn export_to_env(self) {
        use crate::var_names;
        use std::env::set_var;

        fn set_optional_var(var_name: &str, value: Option<String>) {
            if let Some(value) = value {
                unsafe { set_var(var_name, value) }
            }
        }

        unsafe {
            let nym_api_urls = serialize_api_urls(&self.networking.nym_api_urls);
            let nym_vpn_api_urls = serialize_api_urls(&self.networking.nym_vpn_api_urls);
            let dns_fallbacks = self.networking.dns_fallbacks;

            set_var(var_names::NETWORK_NAME, self.network_name);
            set_var(var_names::BECH32_PREFIX, self.chain_details.bech32_account_prefix);

            set_var(var_names::MIX_DENOM, self.chain_details.mix_denom.base);
            set_var(var_names::MIX_DENOM_DISPLAY, self.chain_details.mix_denom.display);

            set_var(var_names::STAKE_DENOM, self.chain_details.stake_denom.base);
            set_var(var_names::STAKE_DENOM_DISPLAY, self.chain_details.stake_denom.display);

            set_var(var_names::DENOMS_EXPONENT, self.chain_details.mix_denom.display_exponent.to_string());

            if let Some(e) = self.endpoints.first() {
                set_var(var_names::NYXD, e.nyxd_url.clone());
                set_optional_var(var_names::NYM_API, e.api_url.clone());
                set_optional_var(var_names::NYXD_WEBSOCKET, e.websocket_url.clone());
            }

            set_optional_var(var_names::MIXNET_CONTRACT_ADDRESS, self.contracts.mixnet_contract_address);
            set_optional_var(var_names::VESTING_CONTRACT_ADDRESS, self.contracts.vesting_contract_address);
            set_optional_var(var_names::NETWORK_MONITORS_CONTRACT_ADDRESS, self.contracts.network_monitors_contract_address);
            set_optional_var(var_names::NODE_FAMILIES_CONTRACT_ADDRESS, self.contracts.node_families_contract_address);
            set_optional_var(var_names::ECASH_CONTRACT_ADDRESS, self.contracts.ecash_contract_address);
            set_optional_var(var_names::GROUP_CONTRACT_ADDRESS, self.contracts.group_contract_address);
            set_optional_var(var_names::MULTISIG_CONTRACT_ADDRESS, self.contracts.multisig_contract_address);
            set_optional_var(var_names::COCONUT_DKG_CONTRACT_ADDRESS, self.contracts.coconut_dkg_contract_address);

            set_optional_var(var_names::NYM_VPN_APIS, nym_vpn_api_urls);
            set_optional_var(var_names::NYM_APIS, nym_api_urls);

            if !dns_fallbacks.is_empty() {
                set_var(var_names::DNS_FALLBACKS, serialize_dns_fallbacks(&dns_fallbacks));
            }
        }
    }

    #[must_use]
    pub fn with_network_name(mut self, network_name: String) -> Self {
        self.network_name = network_name;
        self
    }

    #[must_use]
    pub fn with_chain_details(mut self, chain_details: ChainDetails) -> Self {
        self.chain_details = chain_details;
        self
    }

    #[must_use]
    pub fn with_bech32_account_prefix<S: Into<String>>(mut self, prefix: S) -> Self {
        self.chain_details.bech32_account_prefix = prefix.into();
        self
    }

    #[must_use]
    pub fn with_mix_denom(mut self, mix_denom: DenomDetailsOwned) -> Self {
        self.chain_details.mix_denom = mix_denom;
        self
    }

    #[must_use]
    pub fn with_stake_denom(mut self, stake_denom: DenomDetailsOwned) -> Self {
        self.chain_details.stake_denom = stake_denom;
        self
    }

    #[must_use]
    pub fn with_base_mix_denom<S: Into<String>>(mut self, base_mix_denom: S) -> Self {
        self.chain_details.mix_denom = DenomDetailsOwned::base_only(base_mix_denom.into());
        self
    }

    #[must_use]
    pub fn with_base_stake_denom<S: Into<String>>(mut self, base_stake_denom: S) -> Self {
        self.chain_details.stake_denom = DenomDetailsOwned::base_only(base_stake_denom.into());
        self
    }

    #[must_use]
    pub fn with_additional_validator_endpoint(mut self, endpoint: ValidatorDetails) -> Self {
        self.endpoints.push(endpoint);
        self
    }

    #[must_use]
    pub fn with_validator_endpoint(mut self, endpoint: ValidatorDetails) -> Self {
        self.endpoints = vec![endpoint];
        self
    }

    #[must_use]
    pub fn with_contracts(mut self, contracts: NymContracts) -> Self {
        self.contracts = contracts;
        self
    }

    #[must_use]
    pub fn with_mixnet_contract<S: Into<String>>(mut self, contract: Option<S>) -> Self {
        self.contracts.mixnet_contract_address = contract.map(Into::into);
        self
    }

    #[must_use]
    pub fn with_vesting_contract<S: Into<String>>(mut self, contract: Option<S>) -> Self {
        self.contracts.vesting_contract_address = contract.map(Into::into);
        self
    }

    #[must_use]
    pub fn with_node_families_contract<S: Into<String>>(mut self, contract: Option<S>) -> Self {
        self.contracts.node_families_contract_address = contract.map(Into::into);
        self
    }

    #[must_use]
    pub fn with_ecash_contract<S: Into<String>>(mut self, contract: Option<S>) -> Self {
        self.contracts.ecash_contract_address = contract.map(Into::into);
        self
    }

    #[must_use]
    pub fn with_group_contract<S: Into<String>>(mut self, contract: Option<S>) -> Self {
        self.contracts.group_contract_address = contract.map(Into::into);
        self
    }

    #[must_use]
    pub fn with_multisig_contract<S: Into<String>>(mut self, contract: Option<S>) -> Self {
        self.contracts.multisig_contract_address = contract.map(Into::into);
        self
    }

    #[must_use]
    pub fn with_coconut_dkg_contract<S: Into<String>>(mut self, contract: Option<S>) -> Self {
        self.contracts.coconut_dkg_contract_address = contract.map(Into::into);
        self
    }

    #[must_use]
    pub fn with_performance_contract<S: Into<String>>(mut self, contract: Option<S>) -> Self {
        self.contracts.performance_contract_address = contract.map(Into::into);
        self
    }

    #[must_use]
    pub fn with_network_monitors_contract<S: Into<String>>(mut self, contract: Option<S>) -> Self {
        self.contracts.network_monitors_contract_address = contract.map(Into::into);
        self
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
        if self.networking.nym_api_urls.is_empty() {
            return self.networking.nym_api_urls.clone();
        }

        self.endpoints
            .iter()
            .filter_map(|e| e.api_url())
            .map(ApiUrl::from)
            .collect()
    }

    pub fn nym_vpn_api_urls(&self) -> Vec<ApiUrl> {
        self.networking.nym_vpn_api_urls.clone()
    }

    pub fn dns_fallbacks(&self) -> Vec<DnsFallback> {
        self.networking.dns_fallbacks.clone()
    }

    pub fn default_gas_price_amount(&self) -> f64 {
        GAS_PRICE_AMOUNT
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
    fn dns_fallbacks_from_env_defaults_to_empty_when_unset_on_unknown_network() {
        with_dns_fallbacks_var_cleared(|| {
            assert_eq!(dns_fallbacks_from_env("some-unknown-network"), Vec::new());
        });
    }

    #[test]
    fn dns_fallbacks_from_env_falls_back_to_mainnet_pins_when_unset() {
        with_dns_fallbacks_var_cleared(|| {
            assert_eq!(
                dns_fallbacks_from_env(mainnet::NETWORK_NAME),
                dns_fallbacks(mainnet::dns::default_static_addrs())
            );
        });
    }

    #[test]
    fn dns_fallbacks_from_env_falls_back_to_sandbox_pins_when_unset() {
        with_dns_fallbacks_var_cleared(|| {
            assert_eq!(
                dns_fallbacks_from_env(sandbox::NETWORK_NAME),
                dns_fallbacks(sandbox::dns::default_static_addrs())
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

            assert_eq!(dns_fallbacks_from_env(mainnet::NETWORK_NAME), fallbacks);
            assert_eq!(
                NymNetworkDetails::new_from_env().networking.dns_fallbacks,
                fallbacks
            );
        });
    }

    #[test]
    fn new_mainnet_dns_fallbacks_are_not_empty() {
        assert!(
            !NymNetworkDetails::new_mainnet()
                .networking
                .dns_fallbacks
                .is_empty()
        );
    }

    // regression test: new_sandbox() used to go through sandbox::network_details().into(),
    // and the v1 -> v2 `From` impl always zeroes dns_fallbacks - so this silently stayed
    // empty even after sandbox::dns pins were added.
    #[test]
    fn new_sandbox_dns_fallbacks_are_not_empty() {
        assert!(
            !NymNetworkDetails::new_sandbox()
                .networking
                .dns_fallbacks
                .is_empty()
        );
    }

    // v1's new_mainnet()/new_sandbox() are now thin `v2::...().into()` wrappers - check the
    // v1-visible fields still come out right despite the construction living here now.
    #[test]
    fn v1_new_mainnet_still_reports_mainnet_details() {
        let v1 = crate::NymNetworkDetails::new_mainnet();
        assert_eq!(v1.network_name, mainnet::NETWORK_NAME);
        assert!(!v1.nym_api_urls().is_empty());
        assert!(v1.contracts.mixnet_contract_address.is_some());
    }

    #[test]
    fn v1_and_v2_mainnet_agree_on_shared_fields() {
        let v1 = crate::NymNetworkDetails::new_mainnet();
        let v2 = NymNetworkDetails::new_mainnet();
        assert_eq!(v1.network_name, v2.network_name);
        assert_eq!(v1.chain_details, v2.chain_details);
        assert_eq!(v1.nym_api_urls(), v2.networking.nym_api_urls);
        assert_eq!(v1.nym_vpn_api_urls(), v2.networking.nym_vpn_api_urls);
    }
}
