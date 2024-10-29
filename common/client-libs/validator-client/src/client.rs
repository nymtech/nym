// Copyright 2021-2023 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use crate::nyxd::{self, NyxdClient};
use crate::signing::direct_wallet::DirectSecp256k1HdWallet;
use crate::signing::signer::{NoSigner, OfflineSigner};
use crate::{
    nym_api, DirectSigningReqwestRpcValidatorClient, QueryReqwestRpcValidatorClient,
    ReqwestRpcClient, ValidatorClientError,
};
use nym_api_requests::ecash::models::{
    AggregatedCoinIndicesSignatureResponse, AggregatedExpirationDateSignatureResponse,
    BatchRedeemTicketsBody, EcashBatchTicketRedemptionResponse, EcashTicketVerificationResponse,
    SpentCredentialsResponse, VerifyEcashTicketBody,
};
use nym_api_requests::ecash::{
    BlindSignRequestBody, BlindedSignatureResponse, PartialCoinIndicesSignatureResponse,
    PartialExpirationDateSignatureResponse, VerificationKeyResponse,
};
use nym_api_requests::models::{
    GatewayCoreStatusResponse, MixnodeCoreStatusResponse, MixnodeStatusResponse,
    NymNodeDescription, RewardEstimationResponse, StakeSaturationResponse,
};
use nym_api_requests::models::{LegacyDescribedGateway, MixNodeBondAnnotated};
use nym_api_requests::nym_nodes::SkimmedNode;
use nym_coconut_dkg_common::types::EpochId;
use nym_http_api_client::UserAgent;
use nym_network_defaults::NymNetworkDetails;
use time::Date;
use url::Url;

pub use crate::nym_api::NymApiClientExt;
use nym_mixnet_contract_common::NymNodeDetails;
pub use nym_mixnet_contract_common::{
    mixnode::MixNodeDetails, GatewayBond, IdentityKey, IdentityKeyRef, NodeId,
};
// re-export the type to not break existing imports
pub use crate::coconut::EcashApiClient;

#[cfg(feature = "http-client")]
use crate::rpc::http_client;
#[cfg(feature = "http-client")]
use crate::{DirectSigningHttpRpcValidatorClient, HttpRpcClient, QueryHttpRpcValidatorClient};

#[must_use]
#[derive(Debug, Clone)]
pub struct Config {
    api_url: Url,
    nyxd_url: Url,

    // TODO: until refactored, this is a dead field under some features
    nyxd_config: nyxd::Config,
}

impl TryFrom<NymNetworkDetails> for Config {
    type Error = ValidatorClientError;

    fn try_from(value: NymNetworkDetails) -> Result<Self, Self::Error> {
        Config::try_from_nym_network_details(&value)
    }
}

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

    pub fn with_simulated_gas_multiplier(mut self, gas_multiplier: f32) -> Self {
        self.nyxd_config.simulated_gas_multiplier = gas_multiplier;
        self
    }
}

pub struct Client<C, S = NoSigner> {
    // ideally they would have been read-only, but unfortunately rust doesn't have such features
    // #[deprecated(note = "please use `nym_api_client` instead")]
    pub nym_api: nym_api::Client,
    // pub nym_api_client: NymApiClient,
    pub nyxd: NyxdClient<C, S>,
}

#[cfg(feature = "http-client")]
impl Client<HttpRpcClient, DirectSecp256k1HdWallet> {
    pub fn new_signing(
        config: Config,
        mnemonic: bip39::Mnemonic,
    ) -> Result<DirectSigningHttpRpcValidatorClient, ValidatorClientError> {
        let rpc_client = http_client(config.nyxd_url.as_str())?;
        let prefix = &config.nyxd_config.chain_details.bech32_account_prefix;
        let wallet = DirectSecp256k1HdWallet::from_mnemonic(prefix, mnemonic);

        Ok(Self::new_signing_with_rpc_client(
            config, rpc_client, wallet,
        ))
    }

    pub fn change_nyxd(&mut self, new_endpoint: Url) -> Result<(), ValidatorClientError> {
        self.nyxd.change_endpoint(new_endpoint.as_ref())?;
        Ok(())
    }
}

impl Client<ReqwestRpcClient, DirectSecp256k1HdWallet> {
    pub fn new_reqwest_signing(
        config: Config,
        mnemonic: bip39::Mnemonic,
    ) -> DirectSigningReqwestRpcValidatorClient {
        let rpc_client = ReqwestRpcClient::new(config.nyxd_url.clone());
        let prefix = &config.nyxd_config.chain_details.bech32_account_prefix;
        let wallet = DirectSecp256k1HdWallet::from_mnemonic(prefix, mnemonic);

        Self::new_signing_with_rpc_client(config, rpc_client, wallet)
    }
}

#[cfg(feature = "http-client")]
impl Client<HttpRpcClient> {
    pub fn new_query(config: Config) -> Result<QueryHttpRpcValidatorClient, ValidatorClientError> {
        let rpc_client = http_client(config.nyxd_url.as_str())?;
        Ok(Self::new_with_rpc_client(config, rpc_client))
    }

    pub fn change_nyxd(&mut self, new_endpoint: Url) -> Result<(), ValidatorClientError> {
        self.nyxd = NyxdClient::connect(self.nyxd.current_config().clone(), new_endpoint.as_ref())?;
        Ok(())
    }
}

impl Client<ReqwestRpcClient> {
    pub fn new_reqwest_query(config: Config) -> QueryReqwestRpcValidatorClient {
        let rpc_client = ReqwestRpcClient::new(config.nyxd_url.clone());
        Self::new_with_rpc_client(config, rpc_client)
    }
}

impl<C> Client<C> {
    pub fn new_with_rpc_client(config: Config, rpc_client: C) -> Self {
        let nym_api_client = nym_api::Client::new(config.api_url.clone(), None);

        Client {
            nym_api: nym_api_client,
            nyxd: NyxdClient::new(config.nyxd_config, rpc_client),
        }
    }
}

impl<C, S> Client<C, S> {
    pub fn new_signing_with_rpc_client(config: Config, rpc_client: C, signer: S) -> Self
    where
        S: OfflineSigner,
    {
        let nym_api_client = nym_api::Client::new(config.api_url.clone(), None);

        Client {
            nym_api: nym_api_client,
            nyxd: NyxdClient::new_signing(config.nyxd_config, rpc_client, signer),
        }
    }
}

// validator-api wrappers
impl<C, S> Client<C, S> {
    pub fn api_url(&self) -> &Url {
        self.nym_api.current_url()
    }

    pub fn change_nym_api(&mut self, new_endpoint: Url) {
        self.nym_api.change_base_url(new_endpoint)
    }

    pub async fn get_cached_mixnodes(&self) -> Result<Vec<MixNodeDetails>, ValidatorClientError> {
        Ok(self.nym_api.get_mixnodes().await?)
    }

    pub async fn get_cached_mixnodes_detailed(
        &self,
    ) -> Result<Vec<MixNodeBondAnnotated>, ValidatorClientError> {
        Ok(self.nym_api.get_mixnodes_detailed().await?)
    }

    pub async fn get_cached_mixnodes_detailed_unfiltered(
        &self,
    ) -> Result<Vec<MixNodeBondAnnotated>, ValidatorClientError> {
        Ok(self.nym_api.get_mixnodes_detailed_unfiltered().await?)
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

    // TODO: combine with NymApiClient...
    pub async fn get_all_cached_described_nodes(
        &self,
    ) -> Result<Vec<NymNodeDescription>, ValidatorClientError> {
        // TODO: deal with paging in macro or some helper function or something, because it's the same pattern everywhere
        let mut page = 0;
        let mut descriptions = Vec::new();

        loop {
            let mut res = self.nym_api.get_nodes_described(Some(page), None).await?;

            descriptions.append(&mut res.data);
            if descriptions.len() < res.pagination.total {
                page += 1
            } else {
                break;
            }
        }

        Ok(descriptions)
    }

    // TODO: combine with NymApiClient...
    pub async fn get_all_cached_bonded_nym_nodes(
        &self,
    ) -> Result<Vec<NymNodeDetails>, ValidatorClientError> {
        // TODO: deal with paging in macro or some helper function or something, because it's the same pattern everywhere
        let mut page = 0;
        let mut bonds = Vec::new();

        loop {
            let mut res = self.nym_api.get_nym_nodes(Some(page), None).await?;

            bonds.append(&mut res.data);
            if bonds.len() < res.pagination.total {
                page += 1
            } else {
                break;
            }
        }

        Ok(bonds)
    }

    pub async fn blind_sign(
        &self,
        request_body: &BlindSignRequestBody,
    ) -> Result<BlindedSignatureResponse, ValidatorClientError> {
        Ok(self.nym_api.blind_sign(request_body).await?)
    }
}

#[derive(Clone)]
pub struct NymApiClient {
    pub nym_api: nym_api::Client,
    // TODO: perhaps if we really need it at some (currently I don't see any reasons for it)
    // we could re-implement the communication with the REST API on port 1317
}

impl NymApiClient {
    pub fn new(api_url: Url) -> Self {
        let nym_api = nym_api::Client::new(api_url, None);

        NymApiClient { nym_api }
    }

    pub fn new_with_user_agent(api_url: Url, user_agent: UserAgent) -> Self {
        let nym_api = nym_api::Client::builder::<_, ValidatorClientError>(api_url)
            .expect("invalid api url")
            .with_user_agent(user_agent)
            .build::<ValidatorClientError>()
            .expect("failed to build nym api client");

        NymApiClient { nym_api }
    }

    pub fn api_url(&self) -> &Url {
        self.nym_api.current_url()
    }

    pub fn change_nym_api(&mut self, new_endpoint: Url) {
        self.nym_api.change_base_url(new_endpoint);
    }

    #[deprecated(note = "use get_basic_active_mixing_assigned_nodes instead")]
    pub async fn get_basic_mixnodes(
        &self,
        semver_compatibility: Option<String>,
    ) -> Result<Vec<SkimmedNode>, ValidatorClientError> {
        Ok(self
            .nym_api
            .get_basic_mixnodes(semver_compatibility)
            .await?
            .nodes)
    }

    #[deprecated(note = "use get_all_basic_entry_assigned_nodes instead")]
    pub async fn get_basic_gateways(
        &self,
        semver_compatibility: Option<String>,
    ) -> Result<Vec<SkimmedNode>, ValidatorClientError> {
        Ok(self
            .nym_api
            .get_basic_gateways(semver_compatibility)
            .await?
            .nodes)
    }

    /// retrieve basic information for nodes are capable of operating as an entry gateway
    /// this includes legacy gateways and nym-nodes
    pub async fn get_all_basic_entry_assigned_nodes(
        &self,
        semver_compatibility: Option<String>,
    ) -> Result<Vec<SkimmedNode>, ValidatorClientError> {
        // TODO: deal with paging in macro or some helper function or something, because it's the same pattern everywhere
        let mut page = 0;
        let mut nodes = Vec::new();

        loop {
            let mut res = self
                .nym_api
                .get_basic_entry_assigned_nodes(
                    semver_compatibility.clone(),
                    false,
                    Some(page),
                    None,
                )
                .await?;

            nodes.append(&mut res.nodes.data);
            if nodes.len() < res.nodes.pagination.total {
                page += 1
            } else {
                break;
            }
        }

        Ok(nodes)
    }

    /// retrieve basic information for nodes that got assigned 'mixing' node in this epoch
    /// this includes legacy mixnodes and nym-nodes
    pub async fn get_basic_active_mixing_assigned_nodes(
        &self,
        semver_compatibility: Option<String>,
    ) -> Result<Vec<SkimmedNode>, ValidatorClientError> {
        // TODO: deal with paging in macro or some helper function or something, because it's the same pattern everywhere
        let mut page = 0;
        let mut nodes = Vec::new();

        loop {
            let mut res = self
                .nym_api
                .get_basic_active_mixing_assigned_nodes(
                    semver_compatibility.clone(),
                    false,
                    Some(page),
                    None,
                )
                .await?;

            nodes.append(&mut res.nodes.data);
            if nodes.len() < res.nodes.pagination.total {
                page += 1
            } else {
                break;
            }
        }

        Ok(nodes)
    }

    pub async fn get_cached_active_mixnodes(
        &self,
    ) -> Result<Vec<MixNodeDetails>, ValidatorClientError> {
        Ok(self.nym_api.get_active_mixnodes().await?)
    }

    pub async fn get_cached_rewarded_mixnodes(
        &self,
    ) -> Result<Vec<MixNodeDetails>, ValidatorClientError> {
        Ok(self.nym_api.get_rewarded_mixnodes().await?)
    }

    pub async fn get_cached_mixnodes(&self) -> Result<Vec<MixNodeDetails>, ValidatorClientError> {
        Ok(self.nym_api.get_mixnodes().await?)
    }

    pub async fn get_cached_gateways(&self) -> Result<Vec<GatewayBond>, ValidatorClientError> {
        Ok(self.nym_api.get_gateways().await?)
    }

    pub async fn get_cached_described_gateways(
        &self,
    ) -> Result<Vec<LegacyDescribedGateway>, ValidatorClientError> {
        Ok(self.nym_api.get_gateways_described().await?)
    }

    pub async fn get_all_described_nodes(
        &self,
    ) -> Result<Vec<NymNodeDescription>, ValidatorClientError> {
        // TODO: deal with paging in macro or some helper function or something, because it's the same pattern everywhere
        let mut page = 0;
        let mut descriptions = Vec::new();

        loop {
            let mut res = self.nym_api.get_nodes_described(Some(page), None).await?;

            descriptions.append(&mut res.data);
            if descriptions.len() < res.pagination.total {
                page += 1
            } else {
                break;
            }
        }

        Ok(descriptions)
    }

    pub async fn get_all_bonded_nym_nodes(
        &self,
    ) -> Result<Vec<NymNodeDetails>, ValidatorClientError> {
        // TODO: deal with paging in macro or some helper function or something, because it's the same pattern everywhere
        let mut page = 0;
        let mut bonds = Vec::new();

        loop {
            let mut res = self.nym_api.get_nym_nodes(Some(page), None).await?;

            bonds.append(&mut res.data);
            if bonds.len() < res.pagination.total {
                page += 1
            } else {
                break;
            }
        }

        Ok(bonds)
    }

    pub async fn get_gateway_core_status_count(
        &self,
        identity: IdentityKeyRef<'_>,
        since: Option<i64>,
    ) -> Result<GatewayCoreStatusResponse, ValidatorClientError> {
        Ok(self
            .nym_api
            .get_gateway_core_status_count(identity, since)
            .await?)
    }

    pub async fn get_mixnode_core_status_count(
        &self,
        mix_id: NodeId,
        since: Option<i64>,
    ) -> Result<MixnodeCoreStatusResponse, ValidatorClientError> {
        Ok(self
            .nym_api
            .get_mixnode_core_status_count(mix_id, since)
            .await?)
    }

    pub async fn get_mixnode_status(
        &self,
        mix_id: NodeId,
    ) -> Result<MixnodeStatusResponse, ValidatorClientError> {
        Ok(self.nym_api.get_mixnode_status(mix_id).await?)
    }

    pub async fn get_mixnode_reward_estimation(
        &self,
        mix_id: NodeId,
    ) -> Result<RewardEstimationResponse, ValidatorClientError> {
        Ok(self.nym_api.get_mixnode_reward_estimation(mix_id).await?)
    }

    pub async fn get_mixnode_stake_saturation(
        &self,
        mix_id: NodeId,
    ) -> Result<StakeSaturationResponse, ValidatorClientError> {
        Ok(self.nym_api.get_mixnode_stake_saturation(mix_id).await?)
    }

    pub async fn blind_sign(
        &self,
        request_body: &BlindSignRequestBody,
    ) -> Result<BlindedSignatureResponse, ValidatorClientError> {
        Ok(self.nym_api.blind_sign(request_body).await?)
    }

    pub async fn verify_ecash_ticket(
        &self,
        request_body: &VerifyEcashTicketBody,
    ) -> Result<EcashTicketVerificationResponse, ValidatorClientError> {
        Ok(self.nym_api.verify_ecash_ticket(request_body).await?)
    }

    pub async fn batch_redeem_ecash_tickets(
        &self,
        request_body: &BatchRedeemTicketsBody,
    ) -> Result<EcashBatchTicketRedemptionResponse, ValidatorClientError> {
        Ok(self
            .nym_api
            .batch_redeem_ecash_tickets(request_body)
            .await?)
    }

    pub async fn spent_credentials_filter(
        &self,
    ) -> Result<SpentCredentialsResponse, ValidatorClientError> {
        Ok(self.nym_api.double_spending_filter_v1().await?)
    }

    pub async fn partial_expiration_date_signatures(
        &self,
        expiration_date: Option<Date>,
    ) -> Result<PartialExpirationDateSignatureResponse, ValidatorClientError> {
        Ok(self
            .nym_api
            .partial_expiration_date_signatures(expiration_date)
            .await?)
    }

    pub async fn partial_coin_indices_signatures(
        &self,
        epoch_id: Option<EpochId>,
    ) -> Result<PartialCoinIndicesSignatureResponse, ValidatorClientError> {
        Ok(self
            .nym_api
            .partial_coin_indices_signatures(epoch_id)
            .await?)
    }

    pub async fn global_expiration_date_signatures(
        &self,
        expiration_date: Option<Date>,
    ) -> Result<AggregatedExpirationDateSignatureResponse, ValidatorClientError> {
        Ok(self
            .nym_api
            .global_expiration_date_signatures(expiration_date)
            .await?)
    }

    pub async fn global_coin_indices_signatures(
        &self,
        epoch_id: Option<EpochId>,
    ) -> Result<AggregatedCoinIndicesSignatureResponse, ValidatorClientError> {
        Ok(self
            .nym_api
            .global_coin_indices_signatures(epoch_id)
            .await?)
    }

    pub async fn master_verification_key(
        &self,
        epoch_id: Option<EpochId>,
    ) -> Result<VerificationKeyResponse, ValidatorClientError> {
        Ok(self.nym_api.master_verification_key(epoch_id).await?)
    }
}
