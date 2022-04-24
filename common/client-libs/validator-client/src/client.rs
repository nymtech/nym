// Copyright 2021 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use crate::{validator_api, ValidatorClientError};
use coconut_interface::{BlindSignRequestBody, BlindedSignatureResponse, VerificationKeyResponse};
use mixnet_contract_common::{GatewayBond, IdentityKeyRef, MixNodeBond};
use url::Url;
use validator_api_requests::models::{
    CoreNodeStatusResponse, MixnodeStatusResponse, RewardEstimationResponse,
    StakeSaturationResponse, UptimeResponse,
};

#[cfg(feature = "nymd-client")]
use network_defaults::DEFAULT_NETWORK;

#[cfg(feature = "nymd-client")]
use crate::nymd::{
    error::NymdError, CosmWasmClient, NymdClient, QueryNymdClient, SigningNymdClient,
};

#[cfg(feature = "nymd-client")]
use mixnet_contract_common::{
    mixnode::DelegationEvent, ContractStateParams, Delegation, IdentityKey, Interval,
    MixnetContractVersion, MixnodeRewardingStatusResponse, RewardedSetNodeStatus,
    RewardedSetUpdateDetails,
};
#[cfg(feature = "nymd-client")]
use std::collections::{HashMap, HashSet};
#[cfg(feature = "nymd-client")]
use std::str::FromStr;

#[cfg(feature = "nymd-client")]
#[must_use]
#[derive(Debug)]
pub struct Config {
    network: network_defaults::all::Network,
    api_url: Url,
    nymd_url: Url,
    mixnet_contract_address: Option<cosmrs::AccountId>,
    vesting_contract_address: Option<cosmrs::AccountId>,
    erc20_bridge_contract_address: Option<cosmrs::AccountId>,

    mixnode_page_limit: Option<u32>,
    gateway_page_limit: Option<u32>,
    mixnode_delegations_page_limit: Option<u32>,
    rewarded_set_page_limit: Option<u32>,
}

#[cfg(feature = "nymd-client")]
impl Config {
    pub fn new(
        network: network_defaults::all::Network,
        nymd_url: Url,
        api_url: Url,
        mixnet_contract_address: Option<cosmrs::AccountId>,
        vesting_contract_address: Option<cosmrs::AccountId>,
        erc20_bridge_contract_address: Option<cosmrs::AccountId>,
    ) -> Self {
        Config {
            network,
            nymd_url,
            mixnet_contract_address,
            vesting_contract_address,
            erc20_bridge_contract_address,
            api_url,
            mixnode_page_limit: None,
            gateway_page_limit: None,
            mixnode_delegations_page_limit: None,
            rewarded_set_page_limit: None,
        }
    }

    pub fn with_mixnode_page_limit(mut self, limit: Option<u32>) -> Config {
        self.mixnode_page_limit = limit;
        self
    }

    pub fn with_gateway_page_limit(mut self, limit: Option<u32>) -> Config {
        self.gateway_page_limit = limit;
        self
    }

    pub fn with_mixnode_delegations_page_limit(mut self, limit: Option<u32>) -> Config {
        self.mixnode_delegations_page_limit = limit;
        self
    }

    pub fn with_rewarded_set_page_limit(mut self, limit: Option<u32>) -> Config {
        self.rewarded_set_page_limit = limit;
        self
    }
}

#[cfg(feature = "nymd-client")]
pub struct Client<C> {
    pub network: network_defaults::all::Network,
    mixnet_contract_address: Option<cosmrs::AccountId>,
    vesting_contract_address: Option<cosmrs::AccountId>,
    erc20_bridge_contract_address: Option<cosmrs::AccountId>,
    mnemonic: Option<bip39::Mnemonic>,

    mixnode_page_limit: Option<u32>,
    gateway_page_limit: Option<u32>,
    mixnode_delegations_page_limit: Option<u32>,
    rewarded_set_page_limit: Option<u32>,

    // ideally they would have been read-only, but unfortunately rust doesn't have such features
    pub validator_api: validator_api::Client,
    pub nymd: NymdClient<C>,
}

#[cfg(feature = "nymd-client")]
impl Client<SigningNymdClient> {
    pub fn new_signing(
        config: Config,
        mnemonic: bip39::Mnemonic,
    ) -> Result<Client<SigningNymdClient>, ValidatorClientError> {
        let validator_api_client = validator_api::Client::new(config.api_url.clone());
        let nymd_client = NymdClient::connect_with_mnemonic(
            config.network,
            config.nymd_url.as_str(),
            config.mixnet_contract_address.clone(),
            config.vesting_contract_address.clone(),
            config.erc20_bridge_contract_address.clone(),
            mnemonic.clone(),
            None,
        )?;

        Ok(Client {
            network: config.network,
            mixnet_contract_address: config.mixnet_contract_address,
            vesting_contract_address: config.vesting_contract_address,
            erc20_bridge_contract_address: config.erc20_bridge_contract_address,
            mnemonic: Some(mnemonic),
            mixnode_page_limit: config.mixnode_page_limit,
            gateway_page_limit: config.gateway_page_limit,
            mixnode_delegations_page_limit: config.mixnode_delegations_page_limit,
            rewarded_set_page_limit: None,
            validator_api: validator_api_client,
            nymd: nymd_client,
        })
    }

    pub fn change_nymd(&mut self, new_endpoint: Url) -> Result<(), ValidatorClientError> {
        self.nymd = NymdClient::connect_with_mnemonic(
            self.network,
            new_endpoint.as_ref(),
            self.mixnet_contract_address.clone(),
            self.vesting_contract_address.clone(),
            self.erc20_bridge_contract_address.clone(),
            self.mnemonic.clone().unwrap(),
            None,
        )?;
        Ok(())
    }

    pub fn set_nymd_simulated_gas_multiplier(&mut self, multiplier: f32) {
        self.nymd.set_simulated_gas_multiplier(multiplier)
    }
}

#[cfg(feature = "nymd-client")]
impl Client<QueryNymdClient> {
    pub fn new_query(config: Config) -> Result<Client<QueryNymdClient>, ValidatorClientError> {
        let validator_api_client = validator_api::Client::new(config.api_url.clone());
        let nymd_client = NymdClient::connect(
            config.nymd_url.as_str(),
            Some(config.mixnet_contract_address.clone().unwrap_or_else(|| {
                cosmrs::AccountId::from_str(DEFAULT_NETWORK.mixnet_contract_address()).unwrap()
            })),
            Some(config.vesting_contract_address.clone().unwrap_or_else(|| {
                cosmrs::AccountId::from_str(DEFAULT_NETWORK.vesting_contract_address()).unwrap()
            })),
            Some(
                config
                    .erc20_bridge_contract_address
                    .clone()
                    .unwrap_or_else(|| {
                        cosmrs::AccountId::from_str(
                            DEFAULT_NETWORK.bandwidth_claim_contract_address(),
                        )
                        .unwrap()
                    }),
            ),
        )?;

        Ok(Client {
            network: config.network,
            mixnet_contract_address: config.mixnet_contract_address,
            vesting_contract_address: config.vesting_contract_address,
            erc20_bridge_contract_address: config.erc20_bridge_contract_address,
            mnemonic: None,
            mixnode_page_limit: config.mixnode_page_limit,
            gateway_page_limit: config.gateway_page_limit,
            mixnode_delegations_page_limit: config.mixnode_delegations_page_limit,
            rewarded_set_page_limit: config.rewarded_set_page_limit,
            validator_api: validator_api_client,
            nymd: nymd_client,
        })
    }

    pub fn change_nymd(&mut self, new_endpoint: Url) -> Result<(), ValidatorClientError> {
        self.nymd = NymdClient::connect(
            new_endpoint.as_ref(),
            self.mixnet_contract_address.clone(),
            self.vesting_contract_address.clone(),
            self.erc20_bridge_contract_address.clone(),
        )?;
        Ok(())
    }
}

#[cfg(feature = "nymd-client")]
impl<C> Client<C> {
    pub fn change_validator_api(&mut self, new_endpoint: Url) {
        self.validator_api.change_url(new_endpoint)
    }

    // use case: somebody initialised client without a contract in order to upload and initialise one
    // and now they want to actually use it without making new client
    pub fn set_mixnet_contract_address(&mut self, mixnet_contract_address: cosmrs::AccountId) {
        self.mixnet_contract_address = Some(mixnet_contract_address)
    }

    pub fn get_mixnet_contract_address(&self) -> Option<cosmrs::AccountId> {
        self.mixnet_contract_address.clone()
    }

    pub async fn get_cached_mixnodes(&self) -> Result<Vec<MixNodeBond>, ValidatorClientError> {
        Ok(self.validator_api.get_mixnodes().await?)
    }

    pub async fn get_cached_rewarded_mixnodes(
        &self,
    ) -> Result<Vec<MixNodeBond>, ValidatorClientError> {
        Ok(self.validator_api.get_rewarded_mixnodes().await?)
    }

    pub async fn get_cached_active_mixnodes(
        &self,
    ) -> Result<Vec<MixNodeBond>, ValidatorClientError> {
        Ok(self.validator_api.get_active_mixnodes().await?)
    }

    pub async fn get_cached_gateways(&self) -> Result<Vec<GatewayBond>, ValidatorClientError> {
        Ok(self.validator_api.get_gateways().await?)
    }

    pub async fn get_contract_settings(&self) -> Result<ContractStateParams, ValidatorClientError>
    where
        C: CosmWasmClient + Sync,
    {
        Ok(self.nymd.get_contract_settings().await?)
    }

    pub async fn get_operator_rewards(&self, address: String) -> Result<u128, ValidatorClientError>
    where
        C: CosmWasmClient + Sync,
    {
        Ok(self.nymd.get_operator_rewards(address).await?.u128())
    }

    pub async fn get_delegator_rewards(
        &self,
        address: String,
        mix_identity: IdentityKey,
    ) -> Result<u128, ValidatorClientError>
    where
        C: CosmWasmClient + Sync,
    {
        Ok(self
            .nymd
            .get_delegator_rewards(address, mix_identity)
            .await?
            .u128())
    }

    pub async fn get_pending_delegation_events(
        &self,
        owner_address: String,
        proxy_address: Option<String>,
    ) -> Result<Vec<DelegationEvent>, ValidatorClientError>
    where
        C: CosmWasmClient + Sync,
    {
        Ok(self
            .nymd
            .get_pending_delegation_events(owner_address, proxy_address)
            .await?)
    }

    pub async fn get_current_epoch(&self) -> Result<Interval, ValidatorClientError>
    where
        C: CosmWasmClient + Sync,
    {
        Ok(self.nymd.get_current_epoch().await?)
    }

    pub async fn get_mixnet_contract_version(&self) -> Result<MixnetContractVersion, NymdError>
    where
        C: CosmWasmClient + Sync,
    {
        self.nymd.get_mixnet_contract_version().await
    }

    pub async fn get_rewarding_status(
        &self,
        mix_identity: mixnet_contract_common::IdentityKey,
        rewarding_interval_nonce: u32,
    ) -> Result<MixnodeRewardingStatusResponse, ValidatorClientError>
    where
        C: CosmWasmClient + Sync,
    {
        Ok(self
            .nymd
            .get_rewarding_status(mix_identity, rewarding_interval_nonce)
            .await?)
    }

    pub async fn get_reward_pool(&self) -> Result<u128, ValidatorClientError>
    where
        C: CosmWasmClient + Sync,
    {
        Ok(self.nymd.get_reward_pool().await?.u128())
    }

    pub async fn get_circulating_supply(&self) -> Result<u128, ValidatorClientError>
    where
        C: CosmWasmClient + Sync,
    {
        Ok(self.nymd.get_circulating_supply().await?.u128())
    }

    pub async fn get_sybil_resistance_percent(&self) -> Result<u8, ValidatorClientError>
    where
        C: CosmWasmClient + Sync,
    {
        Ok(self.nymd.get_sybil_resistance_percent().await?)
    }

    pub async fn get_active_set_work_factor(&self) -> Result<u8, ValidatorClientError>
    where
        C: CosmWasmClient + Sync,
    {
        Ok(self.nymd.get_active_set_work_factor().await?)
    }

    pub async fn get_epochs_in_interval(&self) -> Result<u64, ValidatorClientError>
    where
        C: CosmWasmClient + Sync,
    {
        Ok(self.nymd.get_epochs_in_interval().await?)
    }

    pub async fn get_interval_reward_percent(&self) -> Result<u8, ValidatorClientError>
    where
        C: CosmWasmClient + Sync,
    {
        Ok(self.nymd.get_interval_reward_percent().await?)
    }

    pub async fn get_current_rewarded_set_update_details(
        &self,
    ) -> Result<RewardedSetUpdateDetails, ValidatorClientError>
    where
        C: CosmWasmClient + Sync,
    {
        Ok(self
            .nymd
            .query_current_rewarded_set_update_details()
            .await?)
    }

    // basically handles paging for us
    pub async fn get_all_nymd_rewarded_set_mixnode_identities(
        &self,
    ) -> Result<Vec<(IdentityKey, RewardedSetNodeStatus)>, ValidatorClientError>
    where
        C: CosmWasmClient + Sync,
    {
        let mut identities = Vec::new();
        let mut start_after = None;
        let mut height = None;

        loop {
            let mut paged_response = self
                .nymd
                .get_rewarded_set_identities_paged(
                    start_after.take(),
                    self.rewarded_set_page_limit,
                    height,
                )
                .await?;
            identities.append(&mut paged_response.identities);

            if height.is_none() {
                // keep using the same height (the first query happened at the most recent height)
                height = Some(paged_response.at_height)
            }

            if let Some(start_after_res) = paged_response.start_next_after {
                start_after = Some(start_after_res)
            } else {
                break;
            }
        }

        Ok(identities)
    }

    pub async fn get_nymd_rewarded_and_active_sets(
        &self,
    ) -> Result<Vec<(MixNodeBond, RewardedSetNodeStatus)>, ValidatorClientError>
    where
        C: CosmWasmClient + Sync,
    {
        let all_mixnodes = self.get_all_nymd_mixnodes().await?;
        let rewarded_set_identities = self
            .get_all_nymd_rewarded_set_mixnode_identities()
            .await?
            .into_iter()
            .collect::<HashMap<_, _>>();

        Ok(all_mixnodes
            .into_iter()
            .filter_map(|node| {
                rewarded_set_identities
                    .get(node.identity())
                    .map(|status| (node, *status))
            })
            .collect())
    }

    /// If you need both rewarded and the active set, consider using [Self::get_nymd_rewarded_and_active_sets] instead
    pub async fn get_nymd_rewarded_set(&self) -> Result<Vec<MixNodeBond>, ValidatorClientError>
    where
        C: CosmWasmClient + Sync,
    {
        let all_mixnodes = self.get_all_nymd_mixnodes().await?;
        let rewarded_set_identities = self
            .get_all_nymd_rewarded_set_mixnode_identities()
            .await?
            .into_iter()
            .map(|(identity, _status)| identity)
            .collect::<HashSet<_>>();

        Ok(all_mixnodes
            .into_iter()
            .filter(|node| rewarded_set_identities.contains(node.identity()))
            .collect())
    }

    /// If you need both rewarded and the active set, consider using [Self::get_nymd_rewarded_and_active_sets] instead
    pub async fn get_nymd_active_set(&self) -> Result<Vec<MixNodeBond>, ValidatorClientError>
    where
        C: CosmWasmClient + Sync,
    {
        let all_mixnodes = self.get_all_nymd_mixnodes().await?;
        let active_set_identities = self
            .get_all_nymd_rewarded_set_mixnode_identities()
            .await?
            .into_iter()
            .filter_map(|(identity, status)| {
                if status.is_active() {
                    Some(identity)
                } else {
                    None
                }
            })
            .collect::<HashSet<_>>();

        Ok(all_mixnodes
            .into_iter()
            .filter(|node| active_set_identities.contains(node.identity()))
            .collect())
    }

    pub async fn get_all_nymd_mixnodes(&self) -> Result<Vec<MixNodeBond>, ValidatorClientError>
    where
        C: CosmWasmClient + Sync,
    {
        let mut mixnodes = Vec::new();
        let mut start_after = None;
        loop {
            let mut paged_response = self
                .nymd
                .get_mixnodes_paged(start_after.take(), self.mixnode_page_limit)
                .await?;
            mixnodes.append(&mut paged_response.nodes);

            if let Some(start_after_res) = paged_response.start_next_after {
                start_after = Some(start_after_res)
            } else {
                break;
            }
        }

        Ok(mixnodes)
    }

    pub async fn get_all_nymd_gateways(&self) -> Result<Vec<GatewayBond>, ValidatorClientError>
    where
        C: CosmWasmClient + Sync,
    {
        let mut gateways = Vec::new();
        let mut start_after = None;
        loop {
            let mut paged_response = self
                .nymd
                .get_gateways_paged(start_after.take(), self.gateway_page_limit)
                .await?;
            gateways.append(&mut paged_response.nodes);

            if let Some(start_after_res) = paged_response.start_next_after {
                start_after = Some(start_after_res)
            } else {
                break;
            }
        }

        Ok(gateways)
    }

    pub async fn get_all_nymd_single_mixnode_delegations(
        &self,
        identity: IdentityKey,
    ) -> Result<Vec<Delegation>, ValidatorClientError>
    where
        C: CosmWasmClient + Sync,
    {
        let mut delegations = Vec::new();
        let mut start_after = None;
        loop {
            let mut paged_response = self
                .nymd
                .get_mix_delegations_paged(
                    identity.clone(),
                    start_after.take(),
                    self.mixnode_delegations_page_limit,
                )
                .await?;
            delegations.append(&mut paged_response.delegations);

            if let Some(start_after_res) = paged_response.start_next_after {
                start_after = Some(start_after_res)
            } else {
                break;
            }
        }

        Ok(delegations)
    }

    pub async fn get_all_delegator_delegations(
        &self,
        delegation_owner: &cosmrs::AccountId,
    ) -> Result<Vec<Delegation>, ValidatorClientError>
    where
        C: CosmWasmClient + Sync,
    {
        let mut delegations = Vec::new();
        let mut start_after = None;
        loop {
            let mut paged_response = self
                .nymd
                .get_delegator_delegations_paged(
                    delegation_owner.to_string(),
                    start_after.take(),
                    self.mixnode_delegations_page_limit,
                )
                .await?;
            delegations.append(&mut paged_response.delegations);

            if let Some(start_after_res) = paged_response.start_next_after {
                start_after = Some(start_after_res)
            } else {
                break;
            }
        }

        Ok(delegations)
    }

    pub async fn get_mixnode_avg_uptimes(
        &self,
    ) -> Result<Vec<UptimeResponse>, ValidatorClientError> {
        Ok(self.validator_api.get_mixnode_avg_uptimes().await?)
    }

    pub async fn blind_sign(
        &self,
        request_body: &BlindSignRequestBody,
    ) -> Result<BlindedSignatureResponse, ValidatorClientError> {
        Ok(self.validator_api.blind_sign(request_body).await?)
    }

    pub async fn get_coconut_verification_key(
        &self,
    ) -> Result<VerificationKeyResponse, ValidatorClientError> {
        Ok(self.validator_api.get_coconut_verification_key().await?)
    }
}

pub struct ApiClient {
    pub validator_api: validator_api::Client,
    // TODO: perhaps if we really need it at some (currently I don't see any reasons for it)
    // we could re-implement the communication with the REST API on port 1317
}

impl ApiClient {
    pub fn new(api_url: Url) -> Self {
        let validator_api_client = validator_api::Client::new(api_url);

        ApiClient {
            validator_api: validator_api_client,
        }
    }

    pub fn change_validator_api(&mut self, new_endpoint: Url) {
        self.validator_api.change_url(new_endpoint);
    }

    pub async fn get_cached_active_mixnodes(
        &self,
    ) -> Result<Vec<MixNodeBond>, ValidatorClientError> {
        Ok(self.validator_api.get_active_mixnodes().await?)
    }

    pub async fn get_cached_rewarded_mixnodes(
        &self,
    ) -> Result<Vec<MixNodeBond>, ValidatorClientError> {
        Ok(self.validator_api.get_rewarded_mixnodes().await?)
    }

    pub async fn get_cached_mixnodes(&self) -> Result<Vec<MixNodeBond>, ValidatorClientError> {
        Ok(self.validator_api.get_mixnodes().await?)
    }

    pub async fn get_cached_gateways(&self) -> Result<Vec<GatewayBond>, ValidatorClientError> {
        Ok(self.validator_api.get_gateways().await?)
    }

    pub async fn get_gateway_core_status_count(
        &self,
        identity: IdentityKeyRef<'_>,
        since: Option<i64>,
    ) -> Result<CoreNodeStatusResponse, ValidatorClientError> {
        Ok(self
            .validator_api
            .get_gateway_core_status_count(identity, since)
            .await?)
    }

    pub async fn get_mixnode_core_status_count(
        &self,
        identity: IdentityKeyRef<'_>,
        since: Option<i64>,
    ) -> Result<CoreNodeStatusResponse, ValidatorClientError> {
        Ok(self
            .validator_api
            .get_mixnode_core_status_count(identity, since)
            .await?)
    }

    pub async fn get_mixnode_status(
        &self,
        identity: IdentityKeyRef<'_>,
    ) -> Result<MixnodeStatusResponse, ValidatorClientError> {
        Ok(self.validator_api.get_mixnode_status(identity).await?)
    }

    pub async fn get_mixnode_reward_estimation(
        &self,
        identity: IdentityKeyRef<'_>,
    ) -> Result<RewardEstimationResponse, ValidatorClientError> {
        Ok(self
            .validator_api
            .get_mixnode_reward_estimation(identity)
            .await?)
    }

    pub async fn get_mixnode_stake_saturation(
        &self,
        identity: IdentityKeyRef<'_>,
    ) -> Result<StakeSaturationResponse, ValidatorClientError> {
        Ok(self
            .validator_api
            .get_mixnode_stake_saturation(identity)
            .await?)
    }

    pub async fn blind_sign(
        &self,
        request_body: &BlindSignRequestBody,
    ) -> Result<BlindedSignatureResponse, ValidatorClientError> {
        Ok(self.validator_api.blind_sign(request_body).await?)
    }

    pub async fn partial_bandwidth_credential(
        &self,
        request_body: &str,
    ) -> Result<BlindedSignatureResponse, ValidatorClientError> {
        Ok(self
            .validator_api
            .partial_bandwidth_credential(request_body)
            .await?)
    }

    pub async fn get_coconut_verification_key(
        &self,
    ) -> Result<VerificationKeyResponse, ValidatorClientError> {
        Ok(self.validator_api.get_coconut_verification_key().await?)
    }
}
