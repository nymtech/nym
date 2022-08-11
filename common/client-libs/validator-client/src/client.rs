// Copyright 2021 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use crate::{validator_api, ValidatorClientError};
use mixnet_contract_common::mixnode::MixNodeDetails;
use mixnet_contract_common::NodeId;
use mixnet_contract_common::{GatewayBond, IdentityKeyRef};
use url::Url;
use validator_api_requests::coconut::{
    BlindSignRequestBody, BlindedSignatureResponse, CosmosAddressResponse, VerificationKeyResponse,
    VerifyCredentialBody, VerifyCredentialResponse,
};
use validator_api_requests::models::{
    DeprecatedRewardEstimationResponse, DeprecatedUptimeResponse, GatewayCoreStatusResponse,
    MixnodeCoreStatusResponse, MixnodeStatusResponse, RewardEstimationResponse,
    StakeSaturationResponse,
};
use mixnet_contract_common::pending_events::{PendingEpochEvent, PendingIntervalEvent};
use validator_api_requests::Deprecated;

#[cfg(feature = "nymd-client")]
use crate::nymd::traits::MixnetQueryClient;
#[cfg(feature = "nymd-client")]
use crate::nymd::{self, CosmWasmClient, NymdClient, QueryNymdClient, SigningNymdClient};
#[cfg(feature = "nymd-client")]
use mixnet_contract_common::{mixnode::MixNodeBond, Delegation, RewardedSetNodeStatus};
#[cfg(feature = "nymd-client")]
use network_defaults::NymNetworkDetails;
#[cfg(feature = "nymd-client")]
use validator_api_requests::models::MixNodeBondAnnotated;

#[cfg(feature = "nymd-client")]
#[must_use]
#[derive(Debug, Clone)]
pub struct Config {
    api_url: Url,
    nymd_url: Url,

    nymd_config: nymd::Config,

    mixnode_page_limit: Option<u32>,
    gateway_page_limit: Option<u32>,
    mixnode_delegations_page_limit: Option<u32>,
    rewarded_set_page_limit: Option<u32>,
}

#[cfg(feature = "nymd-client")]
impl Config {
    pub fn try_from_nym_network_details(
        details: &NymNetworkDetails,
    ) -> Result<Self, ValidatorClientError> {
        let mut api_url = details
            .endpoints
            .iter()
            .filter_map(|d| d.api_url.as_ref())
            .map(|url| Url::parse(url))
            .collect::<Result<Vec<_>, _>>()?;

        if api_url.is_empty() {
            return Err(ValidatorClientError::NoAPIUrlAvailable);
        }

        Ok(Config {
            api_url: api_url.pop().unwrap(),
            nymd_url: details.endpoints[0]
                .nymd_url
                .parse()
                .map_err(ValidatorClientError::MalformedUrlProvided)?,
            nymd_config: nymd::Config::try_from_nym_network_details(details)?,
            mixnode_page_limit: None,
            gateway_page_limit: None,
            mixnode_delegations_page_limit: None,
            rewarded_set_page_limit: None,
        })
    }

    // TODO: this method shouldn't really exist as all information should be included immediately
    // via `from_nym_network_details`, but it's here for, you guessed it, legacy compatibility
    pub fn with_urls(mut self, nymd_url: Url, api_url: Url) -> Self {
        self.nymd_url = nymd_url;
        self.api_url = api_url;
        self
    }

    pub fn with_nymd_url(mut self, nymd_url: Url) -> Self {
        self.nymd_url = nymd_url;
        self
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
    // TODO: we really shouldn't be storing a mnemonic here, but removing it would be
    // non-trivial amount of work and it's out of scope of the current branch
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
            config.nymd_config.clone(),
            config.nymd_url.as_str(),
            mnemonic.clone(),
            None,
        )?;

        Ok(Client {
            mnemonic: Some(mnemonic),
            mixnode_page_limit: config.mixnode_page_limit,
            gateway_page_limit: config.gateway_page_limit,
            mixnode_delegations_page_limit: config.mixnode_delegations_page_limit,
            rewarded_set_page_limit: config.rewarded_set_page_limit,
            validator_api: validator_api_client,
            nymd: nymd_client,
        })
    }

    pub fn change_nymd(&mut self, new_endpoint: Url) -> Result<(), ValidatorClientError> {
        self.nymd = NymdClient::connect_with_mnemonic(
            self.nymd.current_config().clone(),
            new_endpoint.as_ref(),
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
        let nymd_client =
            NymdClient::connect(config.nymd_config.clone(), config.nymd_url.as_str())?;

        Ok(Client {
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
        self.nymd = NymdClient::connect(self.nymd.current_config().clone(), new_endpoint.as_ref())?;
        Ok(())
    }
}

// nymd wrappers
#[cfg(feature = "nymd-client")]
impl<C> Client<C> {
    // use case: somebody initialised client without a contract in order to upload and initialise one
    // and now they want to actually use it without making new client
    pub fn set_mixnet_contract_address(&mut self, mixnet_contract_address: cosmrs::AccountId) {
        self.nymd
            .set_mixnet_contract_address(mixnet_contract_address)
    }

    pub fn get_mixnet_contract_address(&self) -> cosmrs::AccountId {
        self.nymd.mixnet_contract_address().clone()
    }

    // basically handles paging for us
    pub async fn get_all_nymd_rewarded_set_mixnodes(
        &self,
    ) -> Result<Vec<(NodeId, RewardedSetNodeStatus)>, ValidatorClientError>
    where
        C: CosmWasmClient + Sync + Send,
    {
        let mut identities = Vec::new();
        let mut start_after = None;

        loop {
            let mut paged_response = self
                .nymd
                .get_rewarded_set_paged(start_after.take(), self.rewarded_set_page_limit)
                .await?;
            identities.append(&mut paged_response.nodes);

            if let Some(start_after_res) = paged_response.start_next_after {
                start_after = Some(start_after_res)
            } else {
                break;
            }
        }

        Ok(identities)
    }

    pub async fn get_all_nymd_mixnode_bonds(&self) -> Result<Vec<MixNodeBond>, ValidatorClientError>
    where
        C: CosmWasmClient + Sync + Send,
    {
        let mut mixnodes = Vec::new();
        let mut start_after = None;
        loop {
            let mut paged_response = self
                .nymd
                .get_mixnode_bonds_paged(self.mixnode_page_limit, start_after.take())
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

    pub async fn get_all_nymd_mixnodes_detailed(
        &self,
    ) -> Result<Vec<MixNodeDetails>, ValidatorClientError>
    where
        C: CosmWasmClient + Sync + Send,
    {
        let mut mixnodes = Vec::new();
        let mut start_after = None;
        loop {
            let mut paged_response = self
                .nymd
                .get_mixnodes_detailed_paged(self.mixnode_page_limit, start_after.take())
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
        C: CosmWasmClient + Sync + Send,
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
        mix_id: NodeId,
    ) -> Result<Vec<Delegation>, ValidatorClientError>
    where
        C: CosmWasmClient + Sync + Send,
    {
        let mut delegations = Vec::new();
        let mut start_after = None;
        loop {
            let mut paged_response = self
                .nymd
                .get_mixnode_delegations_paged(
                    mix_id,
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
        C: CosmWasmClient + Sync + Send,
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

    pub async fn get_all_network_delegations(&self) -> Result<Vec<Delegation>, ValidatorClientError>
    where
        C: CosmWasmClient + Sync + Send,
    {
        let mut delegations = Vec::new();
        let mut start_after = None;
        loop {
            let mut paged_response = self
                .nymd
                .get_all_network_delegations_paged(
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

    pub async fn get_all_nymd_pending_epoch_events(
        &self,
    ) -> Result<Vec<PendingEpochEvent>, ValidatorClientError>
    where
        C: CosmWasmClient + Sync + Send,
    {
        let mut events = Vec::new();
        let mut start_after = None;

        loop {
            let mut paged_response = self
                .nymd
                .get_pending_epoch_events_paged(start_after.take(), self.rewarded_set_page_limit)
                .await?;
            events.append(&mut paged_response.events);

            if let Some(start_after_res) = paged_response.start_next_after {
                start_after = Some(start_after_res)
            } else {
                break;
            }
        }

        Ok(events)
    }

    pub async fn get_all_nymd_pending_interval_events(
        &self,
    ) -> Result<Vec<PendingIntervalEvent>, ValidatorClientError>
    where
        C: CosmWasmClient + Sync + Send,
    {
        let mut events = Vec::new();
        let mut start_after = None;

        loop {
            let mut paged_response = self
                .nymd
                .get_pending_interval_events_paged(start_after.take(), self.rewarded_set_page_limit)
                .await?;
            events.append(&mut paged_response.events);

            if let Some(start_after_res) = paged_response.start_next_after {
                start_after = Some(start_after_res)
            } else {
                break;
            }
        }

        Ok(events)
    }
}

// validator-api wrappers
#[cfg(feature = "nymd-client")]
impl<C> Client<C> {
    pub fn change_validator_api(&mut self, new_endpoint: Url) {
        self.validator_api.change_url(new_endpoint)
    }

    pub async fn get_cached_mixnodes(&self) -> Result<Vec<MixNodeDetails>, ValidatorClientError> {
        Ok(self.validator_api.get_mixnodes().await?)
    }

    pub async fn get_cached_mixnodes_detailed(
        &self,
    ) -> Result<Vec<MixNodeBondAnnotated>, ValidatorClientError> {
        Ok(self.validator_api.get_mixnodes_detailed().await?)
    }

    pub async fn get_cached_rewarded_mixnodes(
        &self,
    ) -> Result<Vec<MixNodeDetails>, ValidatorClientError> {
        Ok(self.validator_api.get_rewarded_mixnodes().await?)
    }

    pub async fn get_cached_rewarded_mixnodes_detailed(
        &self,
    ) -> Result<Vec<MixNodeBondAnnotated>, ValidatorClientError> {
        Ok(self.validator_api.get_rewarded_mixnodes_detailed().await?)
    }

    pub async fn get_cached_active_mixnodes(
        &self,
    ) -> Result<Vec<MixNodeDetails>, ValidatorClientError> {
        Ok(self.validator_api.get_active_mixnodes().await?)
    }

    pub async fn get_cached_active_mixnodes_detailed(
        &self,
    ) -> Result<Vec<MixNodeBondAnnotated>, ValidatorClientError> {
        Ok(self.validator_api.get_active_mixnodes_detailed().await?)
    }

    pub async fn get_cached_gateways(&self) -> Result<Vec<GatewayBond>, ValidatorClientError> {
        Ok(self.validator_api.get_gateways().await?)
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
    ) -> Result<Vec<MixNodeDetails>, ValidatorClientError> {
        Ok(self.validator_api.get_active_mixnodes().await?)
    }

    pub async fn get_cached_rewarded_mixnodes(
        &self,
    ) -> Result<Vec<MixNodeDetails>, ValidatorClientError> {
        Ok(self.validator_api.get_rewarded_mixnodes().await?)
    }

    pub async fn get_cached_mixnodes(&self) -> Result<Vec<MixNodeDetails>, ValidatorClientError> {
        Ok(self.validator_api.get_mixnodes().await?)
    }

    pub async fn get_cached_gateways(&self) -> Result<Vec<GatewayBond>, ValidatorClientError> {
        Ok(self.validator_api.get_gateways().await?)
    }

    pub async fn get_gateway_core_status_count(
        &self,
        identity: IdentityKeyRef<'_>,
        since: Option<i64>,
    ) -> Result<GatewayCoreStatusResponse, ValidatorClientError> {
        Ok(self
            .validator_api
            .get_gateway_core_status_count(identity, since)
            .await?)
    }

    pub async fn get_mixnode_core_status_count(
        &self,
        mix_id: NodeId,
        since: Option<i64>,
    ) -> Result<MixnodeCoreStatusResponse, ValidatorClientError> {
        Ok(self
            .validator_api
            .get_mixnode_core_status_count(mix_id, since)
            .await?)
    }

    pub async fn get_mixnode_status(
        &self,
        mix_id: NodeId,
    ) -> Result<MixnodeStatusResponse, ValidatorClientError> {
        Ok(self.validator_api.get_mixnode_status(mix_id).await?)
    }

    pub async fn get_mixnode_reward_estimation(
        &self,
        mix_id: NodeId,
    ) -> Result<RewardEstimationResponse, ValidatorClientError> {
        Ok(self
            .validator_api
            .get_mixnode_reward_estimation(mix_id)
            .await?)
    }

    pub async fn get_mixnode_stake_saturation(
        &self,
        mix_id: NodeId,
    ) -> Result<StakeSaturationResponse, ValidatorClientError> {
        Ok(self
            .validator_api
            .get_mixnode_stake_saturation(mix_id)
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

    pub async fn get_cosmos_address(&self) -> Result<CosmosAddressResponse, ValidatorClientError> {
        Ok(self.validator_api.get_cosmos_address().await?)
    }

    pub async fn verify_bandwidth_credential(
        &self,
        request_body: &VerifyCredentialBody,
    ) -> Result<VerifyCredentialResponse, ValidatorClientError> {
        Ok(self
            .validator_api
            .verify_bandwidth_credential(request_body)
            .await?)
    }

    // =================================================
    // DEPRECATED ROUTES
    // TO REMOVE ONCE OTHER PARTS OF THE SYSTEM MIGRATED
    // =================================================

    pub async fn deprecated_get_mixnode_core_status_count_by_identity(
        &self,
        identity: IdentityKeyRef<'_>,
        since: Option<i64>,
    ) -> Result<Deprecated<MixnodeCoreStatusResponse>, ValidatorClientError> {
        Ok(self
            .validator_api
            .deprecated_get_mixnode_core_status_count_by_identity(identity, since)
            .await?)
    }

    pub async fn deprecated_get_mixnode_status_by_identity(
        &self,
        identity: IdentityKeyRef<'_>,
    ) -> Result<Deprecated<MixnodeStatusResponse>, ValidatorClientError> {
        Ok(self
            .validator_api
            .deprecated_get_mixnode_status_by_identity(identity)
            .await?)
    }

    pub async fn deprecated_get_mixnode_reward_estimation_by_identity(
        &self,
        identity: IdentityKeyRef<'_>,
    ) -> Result<DeprecatedRewardEstimationResponse, ValidatorClientError> {
        Ok(self
            .validator_api
            .deprecated_get_mixnode_reward_estimation_by_identity(identity)
            .await?)
    }

    pub async fn deprecated_get_mixnode_stake_saturation_by_identity(
        &self,
        identity: IdentityKeyRef<'_>,
    ) -> Result<Deprecated<StakeSaturationResponse>, ValidatorClientError> {
        Ok(self
            .validator_api
            .deprecated_get_mixnode_stake_saturation_by_identity(identity)
            .await?)
    }

    pub async fn deprecated_get_mixnode_avg_uptime_by_identity(
        &self,
        identity: IdentityKeyRef<'_>,
    ) -> Result<DeprecatedUptimeResponse, ValidatorClientError> {
        Ok(self
            .validator_api
            .deprecated_get_mixnode_avg_uptime_by_identity(identity)
            .await?)
    }

    pub async fn deprecated_get_mixnode_avg_uptimes_by_identity(
        &self,
    ) -> Result<Vec<DeprecatedUptimeResponse>, ValidatorClientError> {
        Ok(self
            .validator_api
            .deprecated_get_mixnode_avg_uptimes_by_identity()
            .await?)
    }
}
