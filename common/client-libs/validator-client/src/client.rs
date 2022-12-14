// Copyright 2021-2022 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use crate::{nym_api, ValidatorClientError};
use coconut_dkg_common::types::NodeIndex;
use coconut_interface::VerificationKey;
use mixnet_contract_common::mixnode::MixNodeDetails;
use mixnet_contract_common::MixId;
use mixnet_contract_common::{GatewayBond, IdentityKeyRef};
use nym_api_requests::coconut::{
    BlindSignRequestBody, BlindedSignatureResponse, VerifyCredentialBody, VerifyCredentialResponse,
};
use nym_api_requests::models::{
    GatewayCoreStatusResponse, MixnodeCoreStatusResponse, MixnodeStatusResponse,
    RewardEstimationResponse, StakeSaturationResponse,
};

#[cfg(feature = "nyxd-client")]
use crate::nyxd::traits::{DkgQueryClient, MixnetQueryClient, MultisigQueryClient};
#[cfg(feature = "nyxd-client")]
use crate::nyxd::{self, CosmWasmClient, NyxdClient, QueryNyxdClient, SigningNyxdClient};
#[cfg(feature = "nyxd-client")]
use coconut_dkg_common::{
    dealer::ContractDealing, types::DealerDetails, verification_key::ContractVKShare,
};
#[cfg(feature = "nyxd-client")]
use coconut_interface::Base58;
#[cfg(feature = "nyxd-client")]
use cw3::ProposalResponse;
#[cfg(feature = "nyxd-client")]
use mixnet_contract_common::{
    families::{Family, FamilyHead},
    mixnode::MixNodeBond,
    pending_events::{PendingEpochEvent, PendingIntervalEvent},
    Delegation, IdentityKey, RewardedSetNodeStatus, UnbondedMixnode,
};
#[cfg(feature = "nyxd-client")]
use network_defaults::NymNetworkDetails;
#[cfg(feature = "nyxd-client")]
use nym_api_requests::models::MixNodeBondAnnotated;
#[cfg(feature = "nyxd-client")]
use std::str::FromStr;
use url::Url;

#[cfg(feature = "nyxd-client")]
#[must_use]
#[derive(Debug, Clone)]
pub struct Config {
    api_url: Url,
    nyxd_url: Url,

    nyxd_config: nyxd::Config,

    mixnode_page_limit: Option<u32>,
    gateway_page_limit: Option<u32>,
    mixnode_delegations_page_limit: Option<u32>,
    rewarded_set_page_limit: Option<u32>,
    dealers_page_limit: Option<u32>,
    verification_key_page_limit: Option<u32>,
    proposals_page_limit: Option<u32>,
}

#[cfg(feature = "nyxd-client")]
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
            nyxd_url: details.endpoints[0]
                .nyxd_url
                .parse()
                .map_err(ValidatorClientError::MalformedUrlProvided)?,
            nyxd_config: nyxd::Config::try_from_nym_network_details(details)?,
            mixnode_page_limit: None,
            gateway_page_limit: None,
            mixnode_delegations_page_limit: None,
            rewarded_set_page_limit: None,
            dealers_page_limit: None,
            verification_key_page_limit: None,
            proposals_page_limit: None,
        })
    }

    // TODO: this method shouldn't really exist as all information should be included immediately
    // via `from_nym_network_details`, but it's here for, you guessed it, legacy compatibility
    pub fn with_urls(mut self, nyxd_url: Url, api_url: Url) -> Self {
        self.nyxd_url = nyxd_url;
        self.api_url = api_url;
        self
    }

    pub fn with_nyxd_url(mut self, nyxd_url: Url) -> Self {
        self.nyxd_url = nyxd_url;
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

#[cfg(feature = "nyxd-client")]
pub struct Client<C> {
    // TODO: we really shouldn't be storing a mnemonic here, but removing it would be
    // non-trivial amount of work and it's out of scope of the current branch
    mnemonic: Option<bip39::Mnemonic>,

    mixnode_page_limit: Option<u32>,
    gateway_page_limit: Option<u32>,
    mixnode_delegations_page_limit: Option<u32>,
    rewarded_set_page_limit: Option<u32>,
    dealers_page_limit: Option<u32>,
    verification_key_page_limit: Option<u32>,
    proposals_page_limit: Option<u32>,

    // ideally they would have been read-only, but unfortunately rust doesn't have such features
    pub nym_api: nym_api::Client,
    pub nyxd: NyxdClient<C>,
}

#[cfg(feature = "nyxd-client")]
impl Client<SigningNyxdClient> {
    pub fn new_signing(
        config: Config,
        mnemonic: bip39::Mnemonic,
    ) -> Result<Client<SigningNyxdClient>, ValidatorClientError> {
        let nym_api_client = nym_api::Client::new(config.api_url.clone());
        let nyxd_client = NyxdClient::connect_with_mnemonic(
            config.nyxd_config.clone(),
            config.nyxd_url.as_str(),
            mnemonic.clone(),
            None,
        )?;

        Ok(Client {
            mnemonic: Some(mnemonic),
            mixnode_page_limit: config.mixnode_page_limit,
            gateway_page_limit: config.gateway_page_limit,
            mixnode_delegations_page_limit: config.mixnode_delegations_page_limit,
            rewarded_set_page_limit: config.rewarded_set_page_limit,
            dealers_page_limit: config.dealers_page_limit,
            verification_key_page_limit: config.verification_key_page_limit,
            proposals_page_limit: config.proposals_page_limit,
            nym_api: nym_api_client,
            nyxd: nyxd_client,
        })
    }

    pub fn change_nyxd(&mut self, new_endpoint: Url) -> Result<(), ValidatorClientError> {
        self.nyxd = NyxdClient::connect_with_mnemonic(
            self.nyxd.current_config().clone(),
            new_endpoint.as_ref(),
            self.mnemonic.clone().unwrap(),
            None,
        )?;
        Ok(())
    }

    pub fn set_nyxd_simulated_gas_multiplier(&mut self, multiplier: f32) {
        self.nyxd.set_simulated_gas_multiplier(multiplier)
    }
}

#[cfg(feature = "nyxd-client")]
impl Client<QueryNyxdClient> {
    pub fn new_query(config: Config) -> Result<Client<QueryNyxdClient>, ValidatorClientError> {
        let nym_api_client = nym_api::Client::new(config.api_url.clone());
        let nyxd_client =
            NyxdClient::connect(config.nyxd_config.clone(), config.nyxd_url.as_str())?;

        Ok(Client {
            mnemonic: None,
            mixnode_page_limit: config.mixnode_page_limit,
            gateway_page_limit: config.gateway_page_limit,
            mixnode_delegations_page_limit: config.mixnode_delegations_page_limit,
            rewarded_set_page_limit: config.rewarded_set_page_limit,
            dealers_page_limit: config.dealers_page_limit,
            verification_key_page_limit: config.verification_key_page_limit,
            proposals_page_limit: config.proposals_page_limit,
            nym_api: nym_api_client,
            nyxd: nyxd_client,
        })
    }

    pub fn change_nyxd(&mut self, new_endpoint: Url) -> Result<(), ValidatorClientError> {
        self.nyxd = NyxdClient::connect(self.nyxd.current_config().clone(), new_endpoint.as_ref())?;
        Ok(())
    }
}

// nyxd wrappers
#[cfg(feature = "nyxd-client")]
impl<C> Client<C> {
    // use case: somebody initialised client without a contract in order to upload and initialise one
    // and now they want to actually use it without making new client

    pub fn set_mixnet_contract_address(&mut self, mixnet_contract_address: cosmrs::AccountId) {
        self.nyxd
            .set_mixnet_contract_address(mixnet_contract_address)
    }

    pub fn get_mixnet_contract_address(&self) -> cosmrs::AccountId {
        self.nyxd.mixnet_contract_address().clone()
    }

    pub async fn get_all_node_families(&self) -> Result<Vec<Family>, ValidatorClientError>
    where
        C: CosmWasmClient + Sync + Send,
    {
        let mut families = Vec::new();
        let mut start_after = None;

        loop {
            let paged_response = self
                .nyxd
                .get_all_node_families_paged(start_after.take(), None)
                .await?;
            families.extend(paged_response.families);

            if let Some(start_after_res) = paged_response.start_next_after {
                start_after = Some(start_after_res)
            } else {
                break;
            }
        }

        Ok(families)
    }

    pub async fn get_all_family_members(
        &self,
    ) -> Result<Vec<(IdentityKey, FamilyHead)>, ValidatorClientError>
    where
        C: CosmWasmClient + Sync + Send,
    {
        let mut members = Vec::new();
        let mut start_after = None;

        loop {
            let paged_response = self
                .nyxd
                .get_all_family_members_paged(start_after.take(), None)
                .await?;
            members.extend(paged_response.members);

            if let Some(start_after_res) = paged_response.start_next_after {
                start_after = Some(start_after_res)
            } else {
                break;
            }
        }

        Ok(members)
    }

    // basically handles paging for us
    pub async fn get_all_nyxd_rewarded_set_mixnodes(
        &self,
    ) -> Result<Vec<(MixId, RewardedSetNodeStatus)>, ValidatorClientError>
    where
        C: CosmWasmClient + Sync + Send,
    {
        let mut identities = Vec::new();
        let mut start_after = None;

        loop {
            let mut paged_response = self
                .nyxd
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

    pub async fn get_all_nyxd_mixnode_bonds(&self) -> Result<Vec<MixNodeBond>, ValidatorClientError>
    where
        C: CosmWasmClient + Sync + Send,
    {
        let mut mixnodes = Vec::new();
        let mut start_after = None;
        loop {
            let mut paged_response = self
                .nyxd
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

    pub async fn get_all_nyxd_mixnodes_detailed(
        &self,
    ) -> Result<Vec<MixNodeDetails>, ValidatorClientError>
    where
        C: CosmWasmClient + Sync + Send,
    {
        let mut mixnodes = Vec::new();
        let mut start_after = None;
        loop {
            let mut paged_response = self
                .nyxd
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

    pub async fn get_all_nyxd_unbonded_mixnodes(
        &self,
    ) -> Result<Vec<(MixId, UnbondedMixnode)>, ValidatorClientError>
    where
        C: CosmWasmClient + Sync + Send,
    {
        let mut mixnodes = Vec::new();
        let mut start_after = None;
        loop {
            let mut paged_response = self
                .nyxd
                .get_unbonded_paged(self.mixnode_page_limit, start_after.take())
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

    pub async fn get_all_nyxd_unbonded_mixnodes_by_owner(
        &self,
        owner: &cosmrs::AccountId,
    ) -> Result<Vec<(MixId, UnbondedMixnode)>, ValidatorClientError>
    where
        C: CosmWasmClient + Sync + Send,
    {
        let mut mixnodes = Vec::new();
        let mut start_after = None;
        loop {
            let mut paged_response = self
                .nyxd
                .get_unbonded_by_owner_paged(owner, self.mixnode_page_limit, start_after.take())
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

    pub async fn get_all_nyxd_unbonded_mixnodes_by_identity(
        &self,
        identity_key: String,
    ) -> Result<Vec<(MixId, UnbondedMixnode)>, ValidatorClientError>
    where
        C: CosmWasmClient + Sync + Send,
    {
        let mut mixnodes = Vec::new();
        let mut start_after = None;
        loop {
            let mut paged_response = self
                .nyxd
                .get_unbonded_by_identity_paged(
                    identity_key.clone(),
                    self.mixnode_page_limit,
                    start_after.take(),
                )
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

    pub async fn get_all_nyxd_gateways(&self) -> Result<Vec<GatewayBond>, ValidatorClientError>
    where
        C: CosmWasmClient + Sync + Send,
    {
        let mut gateways = Vec::new();
        let mut start_after = None;
        loop {
            let mut paged_response = self
                .nyxd
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

    pub async fn get_all_nyxd_single_mixnode_delegations(
        &self,
        mix_id: MixId,
    ) -> Result<Vec<Delegation>, ValidatorClientError>
    where
        C: CosmWasmClient + Sync + Send,
    {
        let mut delegations = Vec::new();
        let mut start_after = None;
        loop {
            let mut paged_response = self
                .nyxd
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
                .nyxd
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
                .nyxd
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

    pub async fn get_all_nyxd_pending_epoch_events(
        &self,
    ) -> Result<Vec<PendingEpochEvent>, ValidatorClientError>
    where
        C: CosmWasmClient + Sync + Send,
    {
        let mut events = Vec::new();
        let mut start_after = None;

        loop {
            let mut paged_response = self
                .nyxd
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

    pub async fn get_all_nyxd_pending_interval_events(
        &self,
    ) -> Result<Vec<PendingIntervalEvent>, ValidatorClientError>
    where
        C: CosmWasmClient + Sync + Send,
    {
        let mut events = Vec::new();
        let mut start_after = None;

        loop {
            let mut paged_response = self
                .nyxd
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

    pub async fn get_all_nyxd_current_dealers(
        &self,
    ) -> Result<Vec<DealerDetails>, ValidatorClientError>
    where
        C: CosmWasmClient + Sync + Send,
    {
        let mut dealers = Vec::new();
        let mut start_after = None;
        loop {
            let mut paged_response = self
                .nyxd
                .get_current_dealers_paged(start_after.take(), self.dealers_page_limit)
                .await?;
            dealers.append(&mut paged_response.dealers);

            if let Some(start_after_res) = paged_response.start_next_after {
                start_after = Some(start_after_res.into_string())
            } else {
                break;
            }
        }

        Ok(dealers)
    }

    pub async fn get_all_nyxd_past_dealers(
        &self,
    ) -> Result<Vec<DealerDetails>, ValidatorClientError>
    where
        C: CosmWasmClient + Sync + Send,
    {
        let mut dealers = Vec::new();
        let mut start_after = None;
        loop {
            let mut paged_response = self
                .nyxd
                .get_past_dealers_paged(start_after.take(), self.dealers_page_limit)
                .await?;
            dealers.append(&mut paged_response.dealers);

            if let Some(start_after_res) = paged_response.start_next_after {
                start_after = Some(start_after_res.into_string())
            } else {
                break;
            }
        }

        Ok(dealers)
    }

    pub async fn get_all_nyxd_epoch_dealings(
        &self,
        idx: usize,
    ) -> Result<Vec<ContractDealing>, ValidatorClientError>
    where
        C: CosmWasmClient + Sync + Send,
    {
        let mut dealings = Vec::new();
        let mut start_after = None;
        loop {
            let mut paged_response = self
                .nyxd
                .get_dealings_paged(idx, start_after.take(), self.dealers_page_limit)
                .await?;
            dealings.append(&mut paged_response.dealings);

            if let Some(start_after_res) = paged_response.start_next_after {
                start_after = Some(start_after_res.into_string())
            } else {
                break;
            }
        }

        Ok(dealings)
    }

    pub async fn get_all_nyxd_verification_key_shares(
        &self,
    ) -> Result<Vec<ContractVKShare>, ValidatorClientError>
    where
        C: CosmWasmClient + Sync + Send,
    {
        let mut shares = Vec::new();
        let mut start_after = None;
        loop {
            let mut paged_response = self
                .nyxd
                .get_vk_shares_paged(start_after.take(), self.verification_key_page_limit)
                .await?;
            shares.append(&mut paged_response.shares);

            if let Some(start_after_res) = paged_response.start_next_after {
                start_after = Some(start_after_res.into_string())
            } else {
                break;
            }
        }

        Ok(shares)
    }

    pub async fn get_all_nyxd_proposals(
        &self,
    ) -> Result<Vec<ProposalResponse>, ValidatorClientError>
    where
        C: CosmWasmClient + Sync + Send,
    {
        let mut proposals = Vec::new();
        let mut start_after = None;

        loop {
            let mut paged_response = self
                .nyxd
                .list_proposals(start_after.take(), self.proposals_page_limit)
                .await?;

            let last_id = paged_response.proposals.last().map(|prop| prop.id);
            proposals.append(&mut paged_response.proposals);

            if let Some(start_after_res) = last_id {
                start_after = Some(start_after_res)
            } else {
                break;
            }
        }

        Ok(proposals)
    }
}

// validator-api wrappers
#[cfg(feature = "nyxd-client")]
impl<C> Client<C> {
    pub fn change_nym_api(&mut self, new_endpoint: Url) {
        self.nym_api.change_url(new_endpoint)
    }

    pub async fn get_cached_mixnodes(&self) -> Result<Vec<MixNodeDetails>, ValidatorClientError> {
        Ok(self.nym_api.get_mixnodes().await?)
    }

    pub async fn get_cached_mixnodes_detailed(
        &self,
    ) -> Result<Vec<MixNodeBondAnnotated>, ValidatorClientError> {
        Ok(self.nym_api.get_mixnodes_detailed().await?)
    }

    pub async fn get_cached_rewarded_mixnodes(
        &self,
    ) -> Result<Vec<MixNodeDetails>, ValidatorClientError> {
        Ok(self.nym_api.get_rewarded_mixnodes().await?)
    }

    pub async fn get_cached_rewarded_mixnodes_detailed(
        &self,
    ) -> Result<Vec<MixNodeBondAnnotated>, ValidatorClientError> {
        Ok(self.nym_api.get_rewarded_mixnodes_detailed().await?)
    }

    pub async fn get_cached_active_mixnodes(
        &self,
    ) -> Result<Vec<MixNodeDetails>, ValidatorClientError> {
        Ok(self.nym_api.get_active_mixnodes().await?)
    }

    pub async fn get_cached_active_mixnodes_detailed(
        &self,
    ) -> Result<Vec<MixNodeBondAnnotated>, ValidatorClientError> {
        Ok(self.nym_api.get_active_mixnodes_detailed().await?)
    }

    pub async fn get_cached_gateways(&self) -> Result<Vec<GatewayBond>, ValidatorClientError> {
        Ok(self.nym_api.get_gateways().await?)
    }

    pub async fn blind_sign(
        &self,
        request_body: &BlindSignRequestBody,
    ) -> Result<BlindedSignatureResponse, ValidatorClientError> {
        Ok(self.nym_api.blind_sign(request_body).await?)
    }
}

#[derive(Clone)]
pub struct CoconutApiClient {
    pub api_client: NymApiClient,
    pub verification_key: VerificationKey,
    pub node_id: NodeIndex,
    #[cfg(feature = "nyxd-client")]
    pub cosmos_address: cosmrs::AccountId,
}

#[cfg(feature = "nyxd-client")]
impl CoconutApiClient {
    pub async fn all_coconut_api_clients<C>(
        nyxd_client: &Client<C>,
    ) -> Result<Vec<Self>, ValidatorClientError>
    where
        C: CosmWasmClient + Sync + Send,
    {
        Ok(nyxd_client
            .get_all_nyxd_verification_key_shares()
            .await?
            .into_iter()
            .filter_map(Self::try_from)
            .collect())
    }

    fn try_from(share: ContractVKShare) -> Option<Self> {
        if share.verified {
            if let Ok(url_address) = Url::parse(&share.announce_address) {
                if let Ok(verification_key) = VerificationKey::try_from_bs58(&share.share) {
                    if let Ok(cosmos_address) = cosmrs::AccountId::from_str(share.owner.as_str()) {
                        return Some(CoconutApiClient {
                            api_client: NymApiClient::new(url_address),
                            verification_key,
                            node_id: share.node_index,
                            cosmos_address,
                        });
                    }
                }
            }
        }
        None
    }
}

#[derive(Clone)]
pub struct NymApiClient {
    pub nym_api_client: nym_api::Client,
    // TODO: perhaps if we really need it at some (currently I don't see any reasons for it)
    // we could re-implement the communication with the REST API on port 1317
}

impl NymApiClient {
    pub fn new(api_url: Url) -> Self {
        let nym_api_client = nym_api::Client::new(api_url);

        NymApiClient { nym_api_client }
    }

    pub fn change_nym_api(&mut self, new_endpoint: Url) {
        self.nym_api_client.change_url(new_endpoint);
    }

    pub async fn get_cached_active_mixnodes(
        &self,
    ) -> Result<Vec<MixNodeDetails>, ValidatorClientError> {
        Ok(self.nym_api_client.get_active_mixnodes().await?)
    }

    pub async fn get_cached_rewarded_mixnodes(
        &self,
    ) -> Result<Vec<MixNodeDetails>, ValidatorClientError> {
        Ok(self.nym_api_client.get_rewarded_mixnodes().await?)
    }

    pub async fn get_cached_mixnodes(&self) -> Result<Vec<MixNodeDetails>, ValidatorClientError> {
        Ok(self.nym_api_client.get_mixnodes().await?)
    }

    pub async fn get_cached_gateways(&self) -> Result<Vec<GatewayBond>, ValidatorClientError> {
        Ok(self.nym_api_client.get_gateways().await?)
    }

    pub async fn get_gateway_core_status_count(
        &self,
        identity: IdentityKeyRef<'_>,
        since: Option<i64>,
    ) -> Result<GatewayCoreStatusResponse, ValidatorClientError> {
        Ok(self
            .nym_api_client
            .get_gateway_core_status_count(identity, since)
            .await?)
    }

    pub async fn get_mixnode_core_status_count(
        &self,
        mix_id: MixId,
        since: Option<i64>,
    ) -> Result<MixnodeCoreStatusResponse, ValidatorClientError> {
        Ok(self
            .nym_api_client
            .get_mixnode_core_status_count(mix_id, since)
            .await?)
    }

    pub async fn get_mixnode_status(
        &self,
        mix_id: MixId,
    ) -> Result<MixnodeStatusResponse, ValidatorClientError> {
        Ok(self.nym_api_client.get_mixnode_status(mix_id).await?)
    }

    pub async fn get_mixnode_reward_estimation(
        &self,
        mix_id: MixId,
    ) -> Result<RewardEstimationResponse, ValidatorClientError> {
        Ok(self
            .nym_api_client
            .get_mixnode_reward_estimation(mix_id)
            .await?)
    }

    pub async fn get_mixnode_stake_saturation(
        &self,
        mix_id: MixId,
    ) -> Result<StakeSaturationResponse, ValidatorClientError> {
        Ok(self
            .nym_api_client
            .get_mixnode_stake_saturation(mix_id)
            .await?)
    }

    pub async fn blind_sign(
        &self,
        request_body: &BlindSignRequestBody,
    ) -> Result<BlindedSignatureResponse, ValidatorClientError> {
        Ok(self.nym_api_client.blind_sign(request_body).await?)
    }

    pub async fn partial_bandwidth_credential(
        &self,
        request_body: &str,
    ) -> Result<BlindedSignatureResponse, ValidatorClientError> {
        Ok(self
            .nym_api_client
            .partial_bandwidth_credential(request_body)
            .await?)
    }

    pub async fn verify_bandwidth_credential(
        &self,
        request_body: &VerifyCredentialBody,
    ) -> Result<VerifyCredentialResponse, ValidatorClientError> {
        Ok(self
            .nym_api_client
            .verify_bandwidth_credential(request_body)
            .await?)
    }
}
