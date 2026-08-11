use super::*;

// I wanted to use the simpler `NetworkDetails` name, but there's a clash
// with `NetworkDetails` defined in all.rs...
#[derive(Clone, Debug, Deserialize, Eq, Hash, PartialEq, Serialize, JsonSchema)]
#[cfg_attr(feature = "utoipa", derive(utoipa::ToSchema))]
pub struct NymNetworkDetails {
    pub network_name: String,
    pub chain_details: ChainDetails,
    pub endpoints: Vec<ValidatorDetails>,
    pub contracts: NymContracts,
    pub nym_api_urls: Option<Vec<ApiUrl>>,
    pub nym_vpn_api_urls: Option<Vec<ApiUrl>>,
    pub nym_vpn_api_url: Option<String>,
}

// by default we assume the same defaults as mainnet, i.e. same prefixes and denoms
impl Default for NymNetworkDetails {
    fn default() -> Self {
        NymNetworkDetails::new_mainnet()
    }
}

impl NymNetworkDetails {
    /// Delegates to [`v2::NymNetworkDetails::new_empty`] - see the module docs on [`v2`].
    pub fn new_empty() -> Self {
        v2::NymNetworkDetails::new_empty().into()
    }

    /// Delegates to [`v2::NymNetworkDetails::new_from_env`] - see the module docs on [`v2`].
    #[cfg(feature = "env")]
    pub fn new_from_env() -> Self {
        v2::NymNetworkDetails::new_from_env().into()
    }

    /// Delegates to [`v2::NymNetworkDetails::new_mainnet`] - see the module docs on [`v2`].
    pub fn new_mainnet() -> Self {
        v2::NymNetworkDetails::new_mainnet().into()
    }

    /// Upgrades to v2 (picking up `nym_api_urls()`'s endpoints-derived fallback along the
    /// way, same as any other v1 -> v2 conversion) and delegates to
    /// [`v2::NymNetworkDetails::export_to_env`] - see the module docs on [`v2`].
    #[cfg(feature = "env")]
    pub fn export_to_env(self) {
        let v2: v2::NymNetworkDetails = self.into();
        v2.export_to_env();
    }

    pub fn default_gas_price_amount(&self) -> f64 {
        GAS_PRICE_AMOUNT
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

    pub fn set_nym_api_urls<U: Into<ApiUrl>>(&mut self, urls: Vec<U>) {
        let urls: Vec<ApiUrl> = urls.into_iter().map(Into::into).collect();
        self.nym_api_urls = (!urls.is_empty()).then_some(urls);
    }

    #[must_use]
    pub fn with_nym_api_urls<U: Into<ApiUrl>>(mut self, urls: Vec<U>) -> Self {
        self.set_nym_api_urls(urls);
        self
    }

    pub fn set_nym_vpn_api_urls<U: Into<ApiUrl>>(&mut self, urls: Vec<U>) {
        let urls: Vec<ApiUrl> = urls.into_iter().map(Into::into).collect();
        if urls.is_empty() {
            self.nym_vpn_api_urls = None;
            self.nym_vpn_api_url = None
        } else {
            self.nym_vpn_api_url = Some(urls.first().expect("checked non-empty above").url.clone());
            self.nym_vpn_api_urls = Some(urls);
        }
    }

    #[must_use]
    pub fn with_nym_vpn_api_urls<U: Into<ApiUrl>>(mut self, urls: Vec<U>) -> Self {
        self.set_nym_vpn_api_urls(urls);
        self
    }

    /// Returns the configured `nym_api_urls` if any are set, otherwise
    /// falls back to the api urls derived from `endpoints` (the legacy validator list).
    pub fn nym_api_urls(&self) -> Vec<ApiUrl> {
        if let Some(urls) = &self.nym_api_urls
            && !urls.is_empty()
        {
            return urls.clone();
        }

        self.endpoints
            .iter()
            .filter_map(|e| e.api_url())
            .map(ApiUrl::from)
            .collect()
    }

    pub fn nym_vpn_api_urls(&self) -> Vec<ApiUrl> {
        self.nym_vpn_api_urls.clone().unwrap_or_default()
    }
}
