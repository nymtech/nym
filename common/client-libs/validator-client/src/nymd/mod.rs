// Copyright 2021 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use crate::nymd::cosmwasm_client::signing_client;
use crate::nymd::cosmwasm_client::types::{
    Account, ChangeAdminResult, ContractCodeId, ExecuteResult, InstantiateOptions,
    InstantiateResult, MigrateResult, SequenceResponse, SimulateResponse, UploadResult,
};
use crate::nymd::error::NymdError;
use crate::nymd::fee::DEFAULT_SIMULATED_GAS_MULTIPLIER;
use crate::nymd::wallet::DirectSecp256k1HdWallet;
use cosmrs::cosmwasm;
use cosmrs::rpc::Error as TendermintRpcError;
use cosmrs::rpc::HttpClientUrl;
use cosmrs::tx::Msg;
use cosmwasm_std::Uint128;
use execute::execute;
use mixnet_contract_common::mixnode::DelegationEvent;
use mixnet_contract_common::{
    ContractStateParams, Delegation, ExecuteMsg, Gateway, GatewayBond, GatewayBondResponse,
    GatewayOwnershipResponse, IdentityKey, Interval, LayerDistribution, MixNode, MixNodeBond,
    MixOwnershipResponse, MixnetContractVersion, MixnodeDetailsResponse,
    MixnodeRewardingStatusResponse, PagedDelegatorDelegationsResponse, PagedGatewayResponse,
    PagedMixDelegationsResponse, PagedMixnodeBondsResponse, PagedRewardedSetResponse, QueryMsg,
    RewardedSetUpdateDetails,
};
use serde::Serialize;
use std::convert::TryInto;
use std::time::SystemTime;
use vesting_contract_common::ExecuteMsg as VestingExecuteMsg;
use vesting_contract_common::QueryMsg as VestingQueryMsg;

pub use crate::nymd::cosmwasm_client::client::CosmWasmClient;
pub use crate::nymd::cosmwasm_client::signing_client::SigningCosmWasmClient;
pub use crate::nymd::fee::Fee;
pub use coin::Coin;
pub use cosmrs::bank::MsgSend;
pub use cosmrs::rpc::endpoint::tx::Response as TxResponse;
pub use cosmrs::rpc::endpoint::validators::Response as ValidatorResponse;
pub use cosmrs::rpc::HttpClient as QueryNymdClient;
pub use cosmrs::rpc::Paging;
pub use cosmrs::tendermint::abci::responses::{DeliverTx, Event};
pub use cosmrs::tendermint::abci::tag::Tag;
pub use cosmrs::tendermint::block::Height;
pub use cosmrs::tendermint::hash;
pub use cosmrs::tendermint::validator::Info as TendermintValidatorInfo;
pub use cosmrs::tendermint::Time as TendermintTime;
pub use cosmrs::tx::{self, Gas};
pub use cosmrs::Coin as CosmosCoin;
pub use cosmrs::{bip32, AccountId, Decimal, Denom};
pub use cosmwasm_std::Coin as CosmWasmCoin;
pub use fee::{gas_price::GasPrice, GasAdjustable, GasAdjustment};
use network_defaults::{ChainDetails, NymNetworkDetails};
pub use signing_client::Client as SigningNymdClient;
pub use traits::{VestingQueryClient, VestingSigningClient};

pub mod coin;
pub mod cosmwasm_client;
pub mod error;
pub mod fee;
pub mod traits;
pub mod wallet;

#[derive(Debug, Clone)]
pub struct Config {
    pub(crate) chain_details: ChainDetails,

    // I'd love to have used `NymContracts` struct directly here instead,
    // however, I'd really prefer to use something more strongly typed (i.e. AccountId vs String)
    pub(crate) mixnet_contract_address: Option<AccountId>,
    pub(crate) vesting_contract_address: Option<AccountId>,
    pub(crate) bandwidth_claim_contract_address: Option<AccountId>,
    pub(crate) coconut_bandwidth_contract_address: Option<AccountId>,
    pub(crate) multisig_contract_address: Option<AccountId>,
    // TODO: add this in later commits
    // pub(crate) gas_price: GasPrice,
}

impl Config {
    fn parse_optional_account(
        raw: Option<&String>,
        expected_prefix: &str,
    ) -> Result<Option<AccountId>, NymdError> {
        if let Some(address) = raw {
            let parsed: AccountId = address
                .parse()
                .map_err(|_| NymdError::MalformedAccountAddress(address.clone()))?;
            if parsed.prefix() != expected_prefix {
                Err(NymdError::UnexpectedBech32Prefix {
                    got: parsed.prefix().into(),
                    expected: expected_prefix.into(),
                })
            } else {
                Ok(Some(parsed))
            }
        } else {
            Ok(None)
        }
    }

    pub fn try_from_nym_network_details(details: &NymNetworkDetails) -> Result<Self, NymdError> {
        let prefix = &details.chain_details.bech32_account_prefix;
        Ok(Config {
            chain_details: details.chain_details.clone(),
            mixnet_contract_address: Self::parse_optional_account(
                details.contracts.mixnet_contract_address.as_ref(),
                prefix,
            )?,
            vesting_contract_address: Self::parse_optional_account(
                details.contracts.vesting_contract_address.as_ref(),
                prefix,
            )?,
            bandwidth_claim_contract_address: Self::parse_optional_account(
                details.contracts.bandwidth_claim_contract_address.as_ref(),
                prefix,
            )?,
            coconut_bandwidth_contract_address: Self::parse_optional_account(
                details
                    .contracts
                    .coconut_bandwidth_contract_address
                    .as_ref(),
                prefix,
            )?,
            multisig_contract_address: Self::parse_optional_account(
                details.contracts.multisig_contract_address.as_ref(),
                prefix,
            )?,
        })
    }
}

#[derive(Debug)]
pub struct NymdClient<C> {
    client: C,
    config: Config,
    client_address: Option<Vec<AccountId>>,
    simulated_gas_multiplier: f32,
}

impl NymdClient<QueryNymdClient> {
    pub fn connect<U>(config: Config, endpoint: U) -> Result<NymdClient<QueryNymdClient>, NymdError>
    where
        U: TryInto<HttpClientUrl, Error = TendermintRpcError>,
    {
        Ok(NymdClient {
            client: QueryNymdClient::new(endpoint)?,
            config,
            client_address: None,
            simulated_gas_multiplier: DEFAULT_SIMULATED_GAS_MULTIPLIER,
        })
    }
}

impl NymdClient<SigningNymdClient> {
    // maybe the wallet could be made into a generic, but for now, let's just have this one implementation
    pub fn connect_with_signer<U: Clone>(
        config: Config,
        network: config::defaults::all::Network,
        endpoint: U,
        signer: DirectSecp256k1HdWallet,
        gas_price: Option<GasPrice>,
    ) -> Result<NymdClient<SigningNymdClient>, NymdError>
    where
        U: TryInto<HttpClientUrl, Error = TendermintRpcError>,
    {
        let denom = network.base_mix_denom();
        let client_address = signer
            .try_derive_accounts()?
            .into_iter()
            .map(|account| account.address)
            .collect();
        let gas_price = gas_price.unwrap_or(GasPrice::new_with_default_price(&denom)?);

        Ok(NymdClient {
            client: SigningNymdClient::connect_with_signer(endpoint, signer, gas_price)?,
            config,
            client_address: Some(client_address),
            simulated_gas_multiplier: DEFAULT_SIMULATED_GAS_MULTIPLIER,
        })
    }

    pub fn connect_with_mnemonic<U: Clone>(
        config: Config,
        endpoint: U,
        mnemonic: bip39::Mnemonic,
        gas_price: Option<GasPrice>,
    ) -> Result<NymdClient<SigningNymdClient>, NymdError>
    where
        U: TryInto<HttpClientUrl, Error = TendermintRpcError>,
    {
        let prefix = &config.chain_details.bech32_account_prefix;
        let denom = &config.chain_details.mix_denom.base;
        let wallet = DirectSecp256k1HdWallet::from_mnemonic(prefix, mnemonic)?;
        let client_address = wallet
            .try_derive_accounts()?
            .into_iter()
            .map(|account| account.address)
            .collect();
        let gas_price = gas_price.unwrap_or(GasPrice::new_with_default_price(denom)?);

        Ok(NymdClient {
            client: SigningNymdClient::connect_with_signer(endpoint, wallet, gas_price)?,
            config,
            client_address: Some(client_address),
            simulated_gas_multiplier: DEFAULT_SIMULATED_GAS_MULTIPLIER,
        })
    }
}

impl<C> NymdClient<C> {
    pub fn current_config(&self) -> &Config {
        &self.config
    }

    pub fn set_mixnet_contract_address(&mut self, address: AccountId) {
        self.config.mixnet_contract_address = Some(address);
    }

    pub fn set_vesting_contract_address(&mut self, address: AccountId) {
        self.config.vesting_contract_address = Some(address);
    }

    pub fn set_bandwidth_claim_contract_address(&mut self, address: AccountId) {
        self.config.bandwidth_claim_contract_address = Some(address);
    }

    pub fn set_coconut_bandwidth_contract_address(&mut self, address: AccountId) {
        self.config.coconut_bandwidth_contract_address = Some(address);
    }

    pub fn set_multisig_contract_address(&mut self, address: AccountId) {
        self.config.multisig_contract_address = Some(address);
    }

    // TODO: this should get changed into Result<&AccountId, NymdError> (or Option<&AccountId> in future commits
    // note: what unwrap is doing here is just moving a failure that would have normally
    // occurred in `connect` when attempting to parse an empty address,
    // so it's not introducing new source of failure (just moves it)
    pub fn mixnet_contract_address(&self) -> &AccountId {
        self.config.mixnet_contract_address.as_ref().unwrap()
    }

    // TODO: this should get changed into Result<&AccountId, NymdError> (or Option<&AccountId> in future commits
    // note: what unwrap is doing here is just moving a failure that would have normally
    // occurred in `connect` when attempting to parse an empty address,
    // so it's not introducing new source of failure (just moves it)
    pub fn vesting_contract_address(&self) -> &AccountId {
        self.config.vesting_contract_address.as_ref().unwrap()
    }

    // TODO: this should get changed into Result<&AccountId, NymdError> (or Option<&AccountId> in future commits
    // note: what unwrap is doing here is just moving a failure that would have normally
    // occurred in `connect` when attempting to parse an empty address,
    // so it's not introducing new source of failure (just moves it)
    pub fn bandwidth_claim_contract_address(&self) -> &AccountId {
        self.config
            .bandwidth_claim_contract_address
            .as_ref()
            .unwrap()
    }

    // TODO: this should get changed into Result<&AccountId, NymdError> (or Option<&AccountId> in future commits
    // note: what unwrap is doing here is just moving a failure that would have normally
    // occurred in `connect` when attempting to parse an empty address,
    // so it's not introducing new source of failure (just moves it)
    pub fn coconut_bandwidth_contract_address(&self) -> &AccountId {
        self.config
            .coconut_bandwidth_contract_address
            .as_ref()
            .unwrap()
    }

    // TODO: this should get changed into Result<&AccountId, NymdError> (or Option<&AccountId> in future commits
    // note: what unwrap is doing here is just moving a failure that would have normally
    // occurred in `connect` when attempting to parse an empty address,
    // so it's not introducing new source of failure (just moves it)
    pub fn multisig_contract_address(&self) -> &AccountId {
        self.config.multisig_contract_address.as_ref().unwrap()
    }

    pub fn set_simulated_gas_multiplier(&mut self, multiplier: f32) {
        self.simulated_gas_multiplier = multiplier;
    }

    pub fn wrap_contract_execute_message<M>(
        &self,
        contract_address: &AccountId,
        msg: &M,
        funds: Vec<Coin>,
    ) -> Result<cosmwasm::MsgExecuteContract, NymdError>
    where
        C: SigningCosmWasmClient,
        M: ?Sized + Serialize,
    {
        Ok(cosmwasm::MsgExecuteContract {
            sender: self.address().clone(),
            contract: contract_address.clone(),
            msg: serde_json::to_vec(msg)?,
            funds: funds.into_iter().map(Into::into).collect(),
        })
    }

    pub fn address(&self) -> &AccountId
    where
        C: SigningCosmWasmClient,
    {
        // if this is a signing client (as required by the trait bound), it must have the address set
        &self.client_address.as_ref().unwrap()[0]
    }

    pub fn gas_price(&self) -> &GasPrice
    where
        C: SigningCosmWasmClient,
    {
        self.client.gas_price()
    }

    pub fn gas_adjustment(&self) -> GasAdjustment {
        self.simulated_gas_multiplier
    }

    pub async fn account_sequence(&self) -> Result<SequenceResponse, NymdError>
    where
        C: SigningCosmWasmClient + Sync,
    {
        self.client.get_sequence(self.address()).await
    }

    pub async fn get_account_details(
        &self,
        address: &AccountId,
    ) -> Result<Option<Account>, NymdError>
    where
        C: SigningCosmWasmClient + Sync,
    {
        self.client.get_account(address).await
    }

    pub async fn get_current_block_timestamp(&self) -> Result<TendermintTime, NymdError>
    where
        C: CosmWasmClient + Sync,
    {
        self.get_block_timestamp(None).await
    }

    pub async fn get_block_timestamp(
        &self,
        height: Option<u32>,
    ) -> Result<TendermintTime, NymdError>
    where
        C: CosmWasmClient + Sync,
    {
        Ok(self.client.get_block(height).await?.block.header.time)
    }

    pub async fn get_current_block_height(&self) -> Result<Height, NymdError>
    where
        C: CosmWasmClient + Sync,
    {
        self.client.get_height().await
    }

    /// Obtains the hash of a block specified by the provided height.
    ///
    /// # Arguments
    ///
    /// * `height`: height of the block for which we want to obtain the hash.
    pub async fn get_block_hash(&self, height: u32) -> Result<hash::Hash, NymdError>
    where
        C: CosmWasmClient + Sync,
    {
        self.client
            .get_block(Some(height))
            .await
            .map(|block| block.block_id.hash)
    }

    pub async fn get_validators(
        &self,
        height: u64,
        paging: Paging,
    ) -> Result<ValidatorResponse, NymdError>
    where
        C: CosmWasmClient + Sync,
    {
        Ok(self.client.validators(height as u32, paging).await?)
    }

    pub async fn get_balance(
        &self,
        address: &AccountId,
        denom: String,
    ) -> Result<Option<Coin>, NymdError>
    where
        C: CosmWasmClient + Sync,
    {
        self.client.get_balance(address, denom).await
    }

    pub async fn get_tx(&self, id: tx::Hash) -> Result<TxResponse, NymdError>
    where
        C: CosmWasmClient + Sync,
    {
        self.client.get_tx(id).await
    }

    pub async fn get_total_supply(&self) -> Result<Vec<Coin>, NymdError>
    where
        C: CosmWasmClient + Sync,
    {
        self.client.get_total_supply().await
    }

    pub async fn get_contract_settings(&self) -> Result<ContractStateParams, NymdError>
    where
        C: CosmWasmClient + Sync,
    {
        let request = QueryMsg::StateParams {};
        self.client
            .query_contract_smart(self.mixnet_contract_address(), &request)
            .await
    }

    pub async fn get_operator_rewards(&self, address: String) -> Result<Uint128, NymdError>
    where
        C: CosmWasmClient + Sync,
    {
        let request = QueryMsg::QueryOperatorReward { address };
        self.client
            .query_contract_smart(self.mixnet_contract_address(), &request)
            .await
    }

    pub async fn vesting_get_locked_pledge_cap(&self) -> Result<Uint128, NymdError>
    where
        C: CosmWasmClient + Sync,
    {
        let request = VestingQueryMsg::GetLockedPledgeCap {};
        self.client
            .query_contract_smart(self.vesting_contract_address(), &request)
            .await
    }

    pub async fn get_delegator_rewards(
        &self,
        address: String,
        mix_identity: IdentityKey,
        proxy: Option<String>,
    ) -> Result<Uint128, NymdError>
    where
        C: CosmWasmClient + Sync,
    {
        let request = QueryMsg::QueryDelegatorReward {
            address,
            mix_identity,
            proxy,
        };
        self.client
            .query_contract_smart(self.mixnet_contract_address(), &request)
            .await
    }

    pub async fn get_pending_delegation_events(
        &self,
        owner_address: String,
        proxy_address: Option<String>,
    ) -> Result<Vec<DelegationEvent>, NymdError>
    where
        C: CosmWasmClient + Sync,
    {
        let request = QueryMsg::GetPendingDelegationEvents {
            owner_address,
            proxy_address,
        };
        self.client
            .query_contract_smart(self.mixnet_contract_address(), &request)
            .await
    }

    pub async fn get_current_epoch(&self) -> Result<Interval, NymdError>
    where
        C: CosmWasmClient + Sync,
    {
        let request = QueryMsg::GetCurrentEpoch {};
        self.client
            .query_contract_smart(self.mixnet_contract_address(), &request)
            .await
    }

    pub async fn get_current_operator_cost(&self) -> Result<u64, NymdError>
    where
        C: CosmWasmClient + Sync,
    {
        let request = QueryMsg::GetCurrentOperatorCost {};
        self.client
            .query_contract_smart(self.mixnet_contract_address(), &request)
            .await
    }

    pub async fn get_mixnet_contract_version(&self) -> Result<MixnetContractVersion, NymdError>
    where
        C: CosmWasmClient + Sync,
    {
        let request = QueryMsg::GetContractVersion {};
        self.client
            .query_contract_smart(self.mixnet_contract_address(), &request)
            .await
    }

    pub async fn get_rewarding_status(
        &self,
        mix_identity: mixnet_contract_common::IdentityKey,
        interval_id: u32,
    ) -> Result<MixnodeRewardingStatusResponse, NymdError>
    where
        C: CosmWasmClient + Sync,
    {
        let request = QueryMsg::GetRewardingStatus {
            mix_identity,
            interval_id,
        };
        self.client
            .query_contract_smart(self.mixnet_contract_address(), &request)
            .await
    }

    pub async fn query_current_rewarded_set_height(&self) -> Result<u64, NymdError>
    where
        C: CosmWasmClient + Sync,
    {
        let request = QueryMsg::GetCurrentRewardedSetHeight {};
        self.client
            .query_contract_smart(self.mixnet_contract_address(), &request)
            .await
    }

    pub async fn query_current_rewarded_set_update_details(
        &self,
    ) -> Result<RewardedSetUpdateDetails, NymdError>
    where
        C: CosmWasmClient + Sync,
    {
        let request = QueryMsg::GetRewardedSetUpdateDetails {};
        self.client
            .query_contract_smart(self.mixnet_contract_address(), &request)
            .await
    }

    pub async fn get_rewarded_set_identities_paged(
        &self,
        start_after: Option<IdentityKey>,
        page_limit: Option<u32>,
        height: Option<u64>,
    ) -> Result<PagedRewardedSetResponse, NymdError>
    where
        C: CosmWasmClient + Sync,
    {
        let request = QueryMsg::GetRewardedSet {
            height,
            start_after,
            limit: page_limit,
        };

        self.client
            .query_contract_smart(self.mixnet_contract_address(), &request)
            .await
    }

    pub async fn get_layer_distribution(&self) -> Result<LayerDistribution, NymdError>
    where
        C: CosmWasmClient + Sync,
    {
        let request = QueryMsg::LayerDistribution {};
        self.client
            .query_contract_smart(self.mixnet_contract_address(), &request)
            .await
    }

    pub async fn get_reward_pool(&self) -> Result<Uint128, NymdError>
    where
        C: CosmWasmClient + Sync,
    {
        let request = QueryMsg::GetRewardPool {};
        self.client
            .query_contract_smart(self.mixnet_contract_address(), &request)
            .await
    }

    pub async fn get_circulating_supply(&self) -> Result<Uint128, NymdError>
    where
        C: CosmWasmClient + Sync,
    {
        let request = QueryMsg::GetCirculatingSupply {};
        self.client
            .query_contract_smart(self.mixnet_contract_address(), &request)
            .await
    }

    pub async fn get_sybil_resistance_percent(&self) -> Result<u8, NymdError>
    where
        C: CosmWasmClient + Sync,
    {
        let request = QueryMsg::GetSybilResistancePercent {};
        self.client
            .query_contract_smart(self.mixnet_contract_address(), &request)
            .await
    }

    pub async fn get_active_set_work_factor(&self) -> Result<u8, NymdError>
    where
        C: CosmWasmClient + Sync,
    {
        let request = QueryMsg::GetActiveSetWorkFactor {};
        self.client
            .query_contract_smart(self.mixnet_contract_address(), &request)
            .await
    }

    pub async fn get_interval_reward_percent(&self) -> Result<u8, NymdError>
    where
        C: CosmWasmClient + Sync,
    {
        let request = QueryMsg::GetIntervalRewardPercent {};
        self.client
            .query_contract_smart(self.mixnet_contract_address(), &request)
            .await
    }

    pub async fn get_epochs_in_interval(&self) -> Result<u64, NymdError>
    where
        C: CosmWasmClient + Sync,
    {
        let request = QueryMsg::GetEpochsInInterval {};
        self.client
            .query_contract_smart(self.mixnet_contract_address(), &request)
            .await
    }

    /// Checks whether there is a bonded mixnode associated with the provided client's address
    pub async fn owns_mixnode(&self, address: &AccountId) -> Result<Option<MixNodeBond>, NymdError>
    where
        C: CosmWasmClient + Sync,
    {
        todo!()
        // let request = QueryMsg::OwnsMixnode {
        //     address: address.to_string(),
        // };
        // let response: MixOwnershipResponse = self
        //     .client
        //     .query_contract_smart(self.mixnet_contract_address(), &request)
        //     .await?;
        // Ok(response.mixnode)
    }

    /// Checks whether there is a bonded gateway associated with the provided client's address
    pub async fn owns_gateway(&self, address: &AccountId) -> Result<Option<GatewayBond>, NymdError>
    where
        C: CosmWasmClient + Sync,
    {
        let request = QueryMsg::OwnsGateway {
            address: address.to_string(),
        };
        let response: GatewayOwnershipResponse = self
            .client
            .query_contract_smart(self.mixnet_contract_address(), &request)
            .await?;
        Ok(response.gateway)
    }

    /// Checks whether there is a bonded mixnode associated with the provided identity key
    pub async fn get_mixnode_bond(
        &self,
        identity: IdentityKey,
    ) -> Result<Option<MixNodeBond>, NymdError>
    where
        C: CosmWasmClient + Sync,
    {
        todo!()
        // let request = QueryMsg::GetMixnodeBond { identity };
        // let response: MixnodeBondResponse = self
        //     .client
        //     .query_contract_smart(self.mixnet_contract_address(), &request)
        //     .await?;
        // Ok(response.mixnode)
    }

    /// Checks whether there is a bonded gateway associated with the provided identity key
    pub async fn get_gateway_bond(
        &self,
        identity: IdentityKey,
    ) -> Result<Option<GatewayBond>, NymdError>
    where
        C: CosmWasmClient + Sync,
    {
        let request = QueryMsg::GetGatewayBond { identity };
        let response: GatewayBondResponse = self
            .client
            .query_contract_smart(self.mixnet_contract_address(), &request)
            .await?;
        Ok(response.gateway)
    }

    pub async fn get_mixnodes_paged(
        &self,
        start_after: Option<IdentityKey>,
        page_limit: Option<u32>,
    ) -> Result<PagedMixnodeBondsResponse, NymdError>
    where
        C: CosmWasmClient + Sync,
    {
        let request = QueryMsg::GetMixNodes {
            start_after,
            limit: page_limit,
        };
        self.client
            .query_contract_smart(self.mixnet_contract_address(), &request)
            .await
    }

    pub async fn get_gateways_paged(
        &self,
        start_after: Option<IdentityKey>,
        page_limit: Option<u32>,
    ) -> Result<PagedGatewayResponse, NymdError>
    where
        C: CosmWasmClient + Sync,
    {
        let request = QueryMsg::GetGateways {
            start_after,
            limit: page_limit,
        };
        self.client
            .query_contract_smart(self.mixnet_contract_address(), &request)
            .await
    }

    /// Gets list of all delegations towards particular mixnode on particular page.
    pub async fn get_mix_delegations_paged(
        &self,
        mix_identity: IdentityKey,
        start_after: Option<(String, u64)>,
        page_limit: Option<u32>,
    ) -> Result<PagedMixDelegationsResponse, NymdError>
    where
        C: CosmWasmClient + Sync,
    {
        let request = QueryMsg::GetMixnodeDelegations {
            mix_identity: mix_identity.to_owned(),
            start_after,
            limit: page_limit,
        };
        self.client
            .query_contract_smart(self.mixnet_contract_address(), &request)
            .await
    }

    /// Gets list of all the mixnodes on which a particular address delegated.
    pub async fn get_delegator_delegations_paged(
        &self,
        delegator: String,
        start_after: Option<IdentityKey>,
        page_limit: Option<u32>,
    ) -> Result<PagedDelegatorDelegationsResponse, NymdError>
    where
        C: CosmWasmClient + Sync,
    {
        let request = QueryMsg::GetDelegatorDelegations {
            delegator,
            start_after,
            limit: page_limit,
        };
        self.client
            .query_contract_smart(self.mixnet_contract_address(), &request)
            .await
    }

    /// Checks value of delegation of given client towards particular mixnode.
    pub async fn get_delegation_details(
        &self,
        mix_identity: IdentityKey,
        delegator: &AccountId,
        proxy: Option<String>,
    ) -> Result<Delegation, NymdError>
    where
        C: CosmWasmClient + Sync,
    {
        let request = QueryMsg::GetDelegationDetails {
            mix_identity,
            delegator: delegator.to_string(),
            proxy,
        };
        self.client
            .query_contract_smart(self.mixnet_contract_address(), &request)
            .await
    }

    pub async fn simulate<I, M>(&self, messages: I) -> Result<SimulateResponse, NymdError>
    where
        C: SigningCosmWasmClient + Sync,
        I: IntoIterator<Item = M> + Send,
        M: Msg,
    {
        self.client
            .simulate(
                self.address(),
                messages
                    .into_iter()
                    .map(|msg| msg.into_any())
                    .collect::<Result<Vec<_>, _>>()
                    .map_err(|_| {
                        NymdError::SerializationError("custom simulate messages".to_owned())
                    })?,
                "simulating execution of transactions",
            )
            .await
    }

    /// Send funds from one address to another
    pub async fn send(
        &self,
        recipient: &AccountId,
        amount: Vec<Coin>,
        memo: impl Into<String> + Send + 'static,
        fee: Option<Fee>,
    ) -> Result<TxResponse, NymdError>
    where
        C: SigningCosmWasmClient + Sync,
    {
        let fee = fee.unwrap_or(Fee::Auto(Some(self.simulated_gas_multiplier)));
        self.client
            .send_tokens(self.address(), recipient, amount, fee, memo)
            .await
    }

    /// Send funds from one address to multiple others
    pub async fn send_multiple(
        &self,
        msgs: Vec<(AccountId, Vec<Coin>)>,
        memo: impl Into<String> + Send + 'static,
        fee: Option<Fee>,
    ) -> Result<TxResponse, NymdError>
    where
        C: SigningCosmWasmClient + Sync,
    {
        let fee = fee.unwrap_or(Fee::Auto(Some(self.simulated_gas_multiplier)));
        self.client
            .send_tokens_multiple(self.address(), msgs, fee, memo)
            .await
    }

    /// Grant a fee allowance from one address to another
    pub async fn grant_allowance(
        &self,
        grantee: &AccountId,
        spend_limit: Vec<Coin>,
        expiration: Option<SystemTime>,
        allowed_messages: Vec<String>,
        memo: impl Into<String> + Send + 'static,
        fee: Option<Fee>,
    ) -> Result<TxResponse, NymdError>
    where
        C: SigningCosmWasmClient + Sync,
    {
        let fee = fee.unwrap_or(Fee::Auto(Some(self.simulated_gas_multiplier)));
        self.client
            .grant_allowance(
                self.address(),
                grantee,
                spend_limit,
                expiration,
                allowed_messages,
                fee,
                memo,
            )
            .await
    }

    /// Revoke a fee allowance from one address to another
    pub async fn revoke_allowance(
        &self,
        grantee: &AccountId,
        memo: impl Into<String> + Send + 'static,
        fee: Option<Fee>,
    ) -> Result<TxResponse, NymdError>
    where
        C: SigningCosmWasmClient + Sync,
    {
        let fee = fee.unwrap_or(Fee::Auto(Some(self.simulated_gas_multiplier)));
        self.client
            .revoke_allowance(self.address(), grantee, fee, memo)
            .await
    }

    pub async fn execute<M>(
        &self,
        contract_address: &AccountId,
        msg: &M,
        fee: Fee,
        memo: impl Into<String> + Send + 'static,
        funds: Vec<Coin>,
    ) -> Result<ExecuteResult, NymdError>
    where
        C: SigningCosmWasmClient + Sync,
        M: ?Sized + Serialize + Sync,
    {
        self.client
            .execute(self.address(), contract_address, msg, fee, memo, funds)
            .await
    }

    pub async fn execute_multiple<I, M>(
        &self,
        contract_address: &AccountId,
        msgs: I,
        fee: Fee,
        memo: impl Into<String> + Send + 'static,
    ) -> Result<ExecuteResult, NymdError>
    where
        C: SigningCosmWasmClient + Sync,
        I: IntoIterator<Item = (M, Vec<Coin>)> + Send,
        M: Serialize,
    {
        self.client
            .execute_multiple(self.address(), contract_address, msgs, fee, memo)
            .await
    }

    pub async fn upload(
        &self,
        wasm_code: Vec<u8>,
        memo: impl Into<String> + Send + 'static,
        fee: Option<Fee>,
    ) -> Result<UploadResult, NymdError>
    where
        C: SigningCosmWasmClient + Sync,
    {
        let fee = fee.unwrap_or(Fee::Auto(Some(self.simulated_gas_multiplier)));
        self.client
            .upload(self.address(), wasm_code, fee, memo)
            .await
    }

    pub async fn instantiate<M>(
        &self,
        code_id: ContractCodeId,
        msg: &M,
        label: String,
        memo: impl Into<String> + Send + 'static,
        options: Option<InstantiateOptions>,
        fee: Option<Fee>,
    ) -> Result<InstantiateResult, NymdError>
    where
        C: SigningCosmWasmClient + Sync,
        M: ?Sized + Serialize + Sync,
    {
        let fee = fee.unwrap_or(Fee::Auto(Some(self.simulated_gas_multiplier)));
        self.client
            .instantiate(self.address(), code_id, msg, label, fee, memo, options)
            .await
    }

    pub async fn update_admin(
        &self,
        contract_address: &AccountId,
        new_admin: &AccountId,
        memo: impl Into<String> + Send + 'static,
        fee: Option<Fee>,
    ) -> Result<ChangeAdminResult, NymdError>
    where
        C: SigningCosmWasmClient + Sync,
    {
        let fee = fee.unwrap_or(Fee::Auto(Some(self.simulated_gas_multiplier)));
        self.client
            .update_admin(self.address(), contract_address, new_admin, fee, memo)
            .await
    }

    pub async fn clear_admin(
        &self,
        contract_address: &AccountId,
        memo: impl Into<String> + Send + 'static,
        fee: Option<Fee>,
    ) -> Result<ChangeAdminResult, NymdError>
    where
        C: SigningCosmWasmClient + Sync,
    {
        let fee = fee.unwrap_or(Fee::Auto(Some(self.simulated_gas_multiplier)));
        self.client
            .clear_admin(self.address(), contract_address, fee, memo)
            .await
    }

    pub async fn migrate<M>(
        &self,
        contract_address: &AccountId,
        code_id: ContractCodeId,
        msg: &M,
        memo: impl Into<String> + Send + 'static,
        fee: Option<Fee>,
    ) -> Result<MigrateResult, NymdError>
    where
        C: SigningCosmWasmClient + Sync,
        M: ?Sized + Serialize + Sync,
    {
        let fee = fee.unwrap_or(Fee::Auto(Some(self.simulated_gas_multiplier)));
        self.client
            .migrate(self.address(), contract_address, code_id, fee, msg, memo)
            .await
    }

    #[execute("mixnet")]
    fn _compound_reward(
        &self,
        operator: Option<String>,
        delegator: Option<String>,
        mix_identity: Option<IdentityKey>,
        proxy: Option<String>,
        fee: Option<Fee>,
    ) -> (ExecuteMsg, Option<Fee>)
    where
        C: SigningCosmWasmClient + Sync,
    {
        (
            ExecuteMsg::CompoundReward {
                operator,
                delegator,
                mix_identity,
                proxy,
            },
            fee,
        )
    }

    #[execute("mixnet")]
    fn _compound_operator_reward(&self, fee: Option<Fee>) -> (ExecuteMsg, Option<Fee>)
    where
        C: SigningCosmWasmClient + Sync,
    {
        todo!()
        // (ExecuteMsg::CompoundOperatorReward {}, fee)
    }

    #[execute("mixnet")]
    fn _claim_operator_reward(&self, fee: Option<Fee>) -> (ExecuteMsg, Option<Fee>)
    where
        C: SigningCosmWasmClient + Sync,
    {
        (ExecuteMsg::ClaimOperatorReward {}, fee)
    }

    #[execute("mixnet")]
    fn _compound_delegator_reward(
        &self,
        mix_identity: IdentityKey,
        fee: Option<Fee>,
    ) -> (ExecuteMsg, Option<Fee>)
    where
        C: SigningCosmWasmClient + Sync,
    {
        todo!()
        // (ExecuteMsg::CompoundDelegatorReward { mix_identity }, fee)
    }

    #[execute("mixnet")]
    fn _claim_delegator_reward(
        &self,
        mix_identity: IdentityKey,
        fee: Option<Fee>,
    ) -> (ExecuteMsg, Option<Fee>)
    where
        C: SigningCosmWasmClient + Sync,
    {
        (ExecuteMsg::ClaimDelegatorReward { mix_identity }, fee)
    }

    #[execute("vesting")]
    fn _vesting_compound_delegator_reward(
        &self,
        mix_identity: IdentityKey,
        fee: Option<Fee>,
    ) -> (VestingExecuteMsg, Option<Fee>)
    where
        C: SigningCosmWasmClient + Sync,
    {
        (
            VestingExecuteMsg::CompoundDelegatorReward { mix_identity },
            fee,
        )
    }

    #[execute("vesting")]
    fn _vesting_claim_operator_reward(&self, fee: Option<Fee>) -> (VestingExecuteMsg, Option<Fee>)
    where
        C: SigningCosmWasmClient + Sync,
    {
        (VestingExecuteMsg::ClaimOperatorReward {}, fee)
    }

    #[execute("vesting")]
    fn _vesting_compound_operator_reward(
        &self,
        fee: Option<Fee>,
    ) -> (VestingExecuteMsg, Option<Fee>)
    where
        C: SigningCosmWasmClient + Sync,
    {
        (VestingExecuteMsg::CompoundOperatorReward {}, fee)
    }

    #[execute("vesting")]
    fn _vesting_claim_delegator_reward(
        &self,
        mix_identity: IdentityKey,
        fee: Option<Fee>,
    ) -> (VestingExecuteMsg, Option<Fee>)
    where
        C: SigningCosmWasmClient + Sync,
    {
        (
            VestingExecuteMsg::ClaimDelegatorReward { mix_identity },
            fee,
        )
    }

    #[execute("vesting")]
    fn _vesting_update_locked_pledge_cap(
        &self,
        amount: Uint128,
        fee: Option<Fee>,
    ) -> (VestingExecuteMsg, Option<Fee>)
    where
        C: SigningCosmWasmClient + Sync,
    {
        (VestingExecuteMsg::UpdateLockedPledgeCap { amount }, fee)
    }

    /// Announce a mixnode, paying a fee.
    pub async fn bond_mixnode(
        &self,
        mixnode: MixNode,
        owner_signature: String,
        pledge: Coin,
        fee: Option<Fee>,
    ) -> Result<ExecuteResult, NymdError>
    where
        C: SigningCosmWasmClient + Sync,
    {
        let fee = fee.unwrap_or(Fee::Auto(Some(self.simulated_gas_multiplier)));

        todo!()
        // let req = ExecuteMsg::BondMixnode {
        //     mix_node: mixnode,
        //     owner_signature,
        // };
        // self.client
        //     .execute(
        //         self.address(),
        //         self.mixnet_contract_address(),
        //         &req,
        //         fee,
        //         "Bonding mixnode from rust!",
        //         vec![pledge],
        //     )
        //     .await
    }

    /// Announce a mixnode on behalf of the owner, paying a fee.
    pub async fn bond_mixnode_on_behalf(
        &self,
        mixnode: MixNode,
        owner: String,
        owner_signature: String,
        pledge: Coin,
        fee: Option<Fee>,
    ) -> Result<ExecuteResult, NymdError>
    where
        C: SigningCosmWasmClient + Sync,
    {
        let fee = fee.unwrap_or(Fee::Auto(Some(self.simulated_gas_multiplier)));

        let req = ExecuteMsg::BondMixnodeOnBehalf {
            mix_node: mixnode,
            owner,
            owner_signature,
        };
        self.client
            .execute(
                self.address(),
                self.mixnet_contract_address(),
                &req,
                fee,
                "Bonding mixnode on behalf from rust!",
                vec![pledge],
            )
            .await
    }

    /// Announce multiple mixnodes on behalf of other owners, paying a fee.
    pub async fn bond_multiple_mixnodes_on_behalf(
        &self,
        mixnode_bonds_with_sigs: Vec<(MixNodeBond, String)>,
        fee: Option<Fee>,
    ) -> Result<ExecuteResult, NymdError>
    where
        C: SigningCosmWasmClient + Sync,
    {
        let fee = fee.unwrap_or(Fee::Auto(Some(self.simulated_gas_multiplier)));

        let reqs: Vec<(ExecuteMsg, Vec<Coin>)> = mixnode_bonds_with_sigs
            .into_iter()
            .map(|(bond, owner_signature)| {
                (
                    ExecuteMsg::BondMixnodeOnBehalf {
                        mix_node: bond.mix_node,
                        owner: bond.owner.to_string(),
                        owner_signature,
                    },
                    vec![bond.original_pledge.into()],
                )
            })
            .collect();

        self.client
            .execute_multiple(
                self.address(),
                self.mixnet_contract_address(),
                reqs,
                fee,
                "Bonding multiple mixnodes on behalf from rust!",
            )
            .await
    }

    /// Unbond a mixnode, removing it from the network and reclaiming staked coins
    pub async fn unbond_mixnode(&self, fee: Option<Fee>) -> Result<ExecuteResult, NymdError>
    where
        C: SigningCosmWasmClient + Sync,
    {
        let fee = fee.unwrap_or(Fee::Auto(Some(self.simulated_gas_multiplier)));

        let req = ExecuteMsg::UnbondMixnode {};
        self.client
            .execute(
                self.address(),
                self.mixnet_contract_address(),
                &req,
                fee,
                "Unbonding mixnode from rust!",
                vec![],
            )
            .await
    }

    /// Unbond a mixnode on behalf of the owner, removing it from the network and reclaiming staked coins
    pub async fn unbond_mixnode_on_behalf(
        &self,
        owner: String,
        fee: Option<Fee>,
    ) -> Result<ExecuteResult, NymdError>
    where
        C: SigningCosmWasmClient + Sync,
    {
        let fee = fee.unwrap_or(Fee::Auto(Some(self.simulated_gas_multiplier)));

        let req = ExecuteMsg::UnbondMixnodeOnBehalf { owner };
        self.client
            .execute(
                self.address(),
                self.mixnet_contract_address(),
                &req,
                fee,
                "Unbonding mixnode on behalf from rust!",
                vec![],
            )
            .await
    }

    /// Update the configuration of a mixnode. Right now, only possible for profit margin.
    pub async fn update_mixnode_config(
        &self,
        profit_margin_percent: u8,
        fee: Option<Fee>,
    ) -> Result<ExecuteResult, NymdError>
    where
        C: SigningCosmWasmClient + Sync,
    {
        let fee = fee.unwrap_or(Fee::Auto(Some(self.simulated_gas_multiplier)));

        todo!()
        // let req = ExecuteMsg::UpdateMixnodeConfig {
        //     profit_margin_percent,
        // };
        // self.client
        //     .execute(
        //         self.address(),
        //         self.mixnet_contract_address(),
        //         &req,
        //         fee,
        //         "Updating mixnode configuration from rust!",
        //         vec![],
        //     )
        //     .await
    }

    /// Delegates specified amount of stake to particular mixnode.
    pub async fn delegate_to_mixnode(
        &self,
        mix_identity: &str,
        amount: Coin,
        fee: Option<Fee>,
    ) -> Result<ExecuteResult, NymdError>
    where
        C: SigningCosmWasmClient + Sync,
    {
        let fee = fee.unwrap_or(Fee::Auto(Some(self.simulated_gas_multiplier)));

        let req = ExecuteMsg::DelegateToMixnode {
            mix_identity: mix_identity.to_string(),
        };
        self.client
            .execute(
                self.address(),
                self.mixnet_contract_address(),
                &req,
                fee,
                "Delegating to mixnode from rust!",
                vec![amount],
            )
            .await
    }

    /// Delegates specified amount of stake to particular mixnode on
    /// behalf of a particular delegator.
    pub async fn delegate_to_mixnode_on_behalf(
        &self,
        mix_identity: &str,
        delegate: &str,
        amount: Coin,
        fee: Option<Fee>,
    ) -> Result<ExecuteResult, NymdError>
    where
        C: SigningCosmWasmClient + Sync,
    {
        let fee = fee.unwrap_or(Fee::Auto(Some(self.simulated_gas_multiplier)));

        let req = ExecuteMsg::DelegateToMixnodeOnBehalf {
            mix_identity: mix_identity.to_string(),
            delegate: delegate.to_string(),
        };
        self.client
            .execute(
                self.address(),
                self.mixnet_contract_address(),
                &req,
                fee,
                "Delegating to mixnode on behalf from rust!",
                vec![amount],
            )
            .await
    }

    /// Delegates specified amount of stake to multiple mixnodes on behalf of multiple delegators.
    pub async fn delegate_to_multiple_mixnodes_on_behalf(
        &self,
        mixnode_delegations: Vec<Delegation>,
        fee: Option<Fee>,
    ) -> Result<ExecuteResult, NymdError>
    where
        C: SigningCosmWasmClient + Sync,
    {
        let fee = fee.unwrap_or(Fee::Auto(Some(self.simulated_gas_multiplier)));

        todo!()

        // let reqs: Vec<(ExecuteMsg, Vec<Coin>)> = mixnode_delegations
        //     .into_iter()
        //     .map(|delegation| {
        //         (
        //             ExecuteMsg::DelegateToMixnodeOnBehalf {
        //                 mix_identity: delegation.node_identity(),
        //                 delegate: delegation.owner().to_string(),
        //             },
        //             vec![delegation.amount().clone().into()],
        //         )
        //     })
        //     .collect();
        //
        // self.client
        //     .execute_multiple(
        //         self.address(),
        //         self.mixnet_contract_address(),
        //         reqs,
        //         fee,
        //         "Delegating to multiple mixnodes on behalf from rust!",
        //     )
        //     .await
    }

    /// Removes stake delegation from a particular mixnode.
    pub async fn remove_mixnode_delegation(
        &self,
        mix_identity: &str,
        fee: Option<Fee>,
    ) -> Result<ExecuteResult, NymdError>
    where
        C: SigningCosmWasmClient + Sync,
    {
        let fee = fee.unwrap_or(Fee::Auto(Some(self.simulated_gas_multiplier)));

        let req = ExecuteMsg::UndelegateFromMixnode {
            mix_identity: mix_identity.to_string(),
        };
        self.client
            .execute(
                self.address(),
                self.mixnet_contract_address(),
                &req,
                fee,
                "Removing mixnode delegation from rust!",
                vec![],
            )
            .await
    }

    /// Removes stake delegation from a particular mixnode on behalf of a particular delegator.
    pub async fn remove_mixnode_delegation_on_behalf(
        &self,
        mix_identity: &str,
        delegate: &str,
        fee: Option<Fee>,
    ) -> Result<ExecuteResult, NymdError>
    where
        C: SigningCosmWasmClient + Sync,
    {
        let fee = fee.unwrap_or(Fee::Auto(Some(self.simulated_gas_multiplier)));

        let req = ExecuteMsg::UndelegateFromMixnodeOnBehalf {
            mix_identity: mix_identity.to_string(),
            delegate: delegate.to_string(),
        };
        self.client
            .execute(
                self.address(),
                self.mixnet_contract_address(),
                &req,
                fee,
                "Removing mixnode delegation on behalf from rust!",
                vec![],
            )
            .await
    }

    /// Announce a gateway, paying a fee.
    pub async fn bond_gateway(
        &self,
        gateway: Gateway,
        owner_signature: String,
        pledge: Coin,
        fee: Option<Fee>,
    ) -> Result<ExecuteResult, NymdError>
    where
        C: SigningCosmWasmClient + Sync,
    {
        let fee = fee.unwrap_or(Fee::Auto(Some(self.simulated_gas_multiplier)));

        let req = ExecuteMsg::BondGateway {
            gateway,
            owner_signature,
        };
        self.client
            .execute(
                self.address(),
                self.mixnet_contract_address(),
                &req,
                fee,
                "Bonding gateway from rust!",
                vec![pledge],
            )
            .await
    }

    /// Announce a gateway on behalf of the owner, paying a fee.
    pub async fn bond_gateway_on_behalf(
        &self,
        gateway: Gateway,
        owner: String,
        owner_signature: String,
        pledge: Coin,
        fee: Option<Fee>,
    ) -> Result<ExecuteResult, NymdError>
    where
        C: SigningCosmWasmClient + Sync,
    {
        let fee = fee.unwrap_or(Fee::Auto(Some(self.simulated_gas_multiplier)));

        let req = ExecuteMsg::BondGatewayOnBehalf {
            gateway,
            owner,
            owner_signature,
        };
        self.client
            .execute(
                self.address(),
                self.mixnet_contract_address(),
                &req,
                fee,
                "Bonding gateway on behalf from rust!",
                vec![pledge],
            )
            .await
    }

    /// Announce multiple gateways on behalf of other owners, paying a fee.
    pub async fn bond_multiple_gateways_on_behalf(
        &self,
        gateway_bonds_with_sigs: Vec<(GatewayBond, String)>,
        fee: Option<Fee>,
    ) -> Result<ExecuteResult, NymdError>
    where
        C: SigningCosmWasmClient + Sync,
    {
        let fee = fee.unwrap_or(Fee::Auto(Some(self.simulated_gas_multiplier)));

        let reqs: Vec<(ExecuteMsg, Vec<Coin>)> = gateway_bonds_with_sigs
            .into_iter()
            .map(|(bond, owner_signature)| {
                (
                    ExecuteMsg::BondGatewayOnBehalf {
                        gateway: bond.gateway,
                        owner: bond.owner.to_string(),
                        owner_signature,
                    },
                    vec![bond.pledge_amount.into()],
                )
            })
            .collect();

        self.client
            .execute_multiple(
                self.address(),
                self.mixnet_contract_address(),
                reqs,
                fee,
                "Bonding multiple gateways on behalf from rust!",
            )
            .await
    }

    /// Unbond a gateway, removing it from the network and reclaiming staked coins
    pub async fn unbond_gateway(&self, fee: Option<Fee>) -> Result<ExecuteResult, NymdError>
    where
        C: SigningCosmWasmClient + Sync,
    {
        let fee = fee.unwrap_or(Fee::Auto(Some(self.simulated_gas_multiplier)));

        let req = ExecuteMsg::UnbondGateway {};
        self.client
            .execute(
                self.address(),
                self.mixnet_contract_address(),
                &req,
                fee,
                "Unbonding gateway from rust!",
                vec![],
            )
            .await
    }

    /// Unbond a gateway on behalf of the owner, removing it from the
    /// network and reclaiming staked coins
    pub async fn unbond_gateway_on_behalf(
        &self,
        owner: String,
        fee: Option<Fee>,
    ) -> Result<ExecuteResult, NymdError>
    where
        C: SigningCosmWasmClient + Sync,
    {
        let fee = fee.unwrap_or(Fee::Auto(Some(self.simulated_gas_multiplier)));

        let req = ExecuteMsg::UnbondGatewayOnBehalf { owner };
        self.client
            .execute(
                self.address(),
                self.mixnet_contract_address(),
                &req,
                fee,
                "Unbonding gateway on behalf from rust!",
                vec![],
            )
            .await
    }

    pub async fn update_contract_settings(
        &self,
        new_params: ContractStateParams,
        fee: Option<Fee>,
    ) -> Result<ExecuteResult, NymdError>
    where
        C: SigningCosmWasmClient + Sync,
    {
        let fee = fee.unwrap_or(Fee::Auto(Some(self.simulated_gas_multiplier)));

        todo!()
        // let req = ExecuteMsg::UpdateContractStateParams(new_params);
        // self.client
        //     .execute(
        //         self.address(),
        //         self.mixnet_contract_address(),
        //         &req,
        //         fee,
        //         "Updating contract state from rust!",
        //         vec![],
        //     )
        //     .await
    }

    pub async fn advance_current_epoch(&self, fee: Option<Fee>) -> Result<ExecuteResult, NymdError>
    where
        C: SigningCosmWasmClient + Sync,
    {
        let fee = fee.unwrap_or(Fee::Auto(Some(self.simulated_gas_multiplier)));

        todo!()
        // let req = ExecuteMsg::AdvanceCurrentEpoch {};
        // self.client
        //     .execute(
        //         self.address(),
        //         self.mixnet_contract_address(),
        //         &req,
        //         fee,
        //         "Advance current epoch",
        //         vec![],
        //     )
        //     .await
    }

    pub async fn reconcile_delegations(&self, fee: Option<Fee>) -> Result<ExecuteResult, NymdError>
    where
        C: SigningCosmWasmClient + Sync,
    {
        let fee = fee.unwrap_or(Fee::Auto(Some(self.simulated_gas_multiplier)));

        todo!()
        // let req = ExecuteMsg::ReconcileDelegations {};
        // self.client
        //     .execute(
        //         self.address(),
        //         self.mixnet_contract_address(),
        //         &req,
        //         fee,
        //         "Reconciling delegation events",
        //         vec![],
        //     )
        //     .await
    }

    pub async fn checkpoint_mixnodes(&self, fee: Option<Fee>) -> Result<ExecuteResult, NymdError>
    where
        C: SigningCosmWasmClient + Sync,
    {
        let fee = fee.unwrap_or(Fee::Auto(Some(self.simulated_gas_multiplier)));

        todo!()
        //
        // let req = ExecuteMsg::CheckpointMixnodes {};
        // self.client
        //     .execute(
        //         self.address(),
        //         self.mixnet_contract_address(),
        //         &req,
        //         fee,
        //         "Snapshotting mixnodes",
        //         vec![],
        //     )
        //     .await
    }

    pub async fn write_rewarded_set(
        &self,
        rewarded_set: Vec<IdentityKey>,
        expected_active_set_size: u32,
        fee: Option<Fee>,
    ) -> Result<ExecuteResult, NymdError>
    where
        C: SigningCosmWasmClient + Sync,
    {
        let fee = fee.unwrap_or(Fee::Auto(Some(self.simulated_gas_multiplier)));

        todo!()
        //
        // let req = ExecuteMsg::WriteRewardedSet {
        //     rewarded_set,
        //     expected_active_set_size,
        // };
        // self.client
        //     .execute(
        //         self.address(),
        //         self.mixnet_contract_address(),
        //         &req,
        //         fee,
        //         "Writing rewarded set",
        //         vec![],
        //     )
        //     .await
    }
}
