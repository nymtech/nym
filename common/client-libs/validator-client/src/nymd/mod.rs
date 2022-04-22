// Copyright 2021 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use crate::nymd::cosmwasm_client::signing_client;
use crate::nymd::cosmwasm_client::types::{
    Account, ChangeAdminResult, ContractCodeId, ExecuteResult, InstantiateOptions,
    InstantiateResult, MigrateResult, SequenceResponse, UploadResult,
};
use crate::nymd::error::NymdError;
use crate::nymd::wallet::DirectSecp256k1HdWallet;
use cosmrs::rpc::endpoint::broadcast;
use cosmrs::rpc::Error as TendermintRpcError;
use cosmrs::rpc::HttpClientUrl;
use cosmwasm_std::{Coin, Uint128};
pub use fee::gas_price::GasPrice;
use fee::helpers::Operation;
use mixnet_contract_common::mixnode::DelegationEvent;
use mixnet_contract_common::{
    ContractStateParams, Delegation, ExecuteMsg, Gateway, GatewayBond, GatewayOwnershipResponse,
    IdentityKey, Interval, LayerDistribution, MixNode, MixNodeBond, MixOwnershipResponse,
    MixnetContractVersion, MixnodeRewardingStatusResponse, PagedDelegatorDelegationsResponse,
    PagedGatewayResponse, PagedMixDelegationsResponse, PagedMixnodeResponse,
    PagedRewardedSetResponse, QueryMsg, RewardedSetUpdateDetails,
};
use serde::Serialize;
use std::convert::TryInto;

pub use crate::nymd::cosmwasm_client::client::CosmWasmClient;
pub use crate::nymd::cosmwasm_client::signing_client::SigningCosmWasmClient;
pub use crate::nymd::fee::Fee;
use crate::nymd::fee::DEFAULT_SIMULATED_GAS_MULTIPLIER;
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
pub use cosmrs::{AccountId, Decimal, Denom};
pub use signing_client::Client as SigningNymdClient;
use std::collections::HashMap;
pub use traits::{VestingQueryClient, VestingSigningClient};

pub mod cosmwasm_client;
pub mod error;
pub mod fee;
pub mod traits;
pub mod wallet;

#[derive(Debug)]
pub struct NymdClient<C> {
    client: C,
    mixnet_contract_address: Option<AccountId>,
    vesting_contract_address: Option<AccountId>,
    erc20_bridge_contract_address: Option<AccountId>,
    client_address: Option<Vec<AccountId>>,
    custom_gas_limits: HashMap<Operation, Gas>,
    simulated_gas_multiplier: f32,
}

impl NymdClient<QueryNymdClient> {
    pub fn connect<U>(
        endpoint: U,
        mixnet_contract_address: Option<AccountId>,
        vesting_contract_address: Option<AccountId>,
        erc20_bridge_contract_address: Option<AccountId>,
    ) -> Result<NymdClient<QueryNymdClient>, NymdError>
    where
        U: TryInto<HttpClientUrl, Error = TendermintRpcError>,
    {
        Ok(NymdClient {
            client: QueryNymdClient::new(endpoint)?,
            mixnet_contract_address,
            vesting_contract_address,
            erc20_bridge_contract_address,
            client_address: None,
            custom_gas_limits: HashMap::default(),
            simulated_gas_multiplier: DEFAULT_SIMULATED_GAS_MULTIPLIER,
        })
    }
}

impl NymdClient<SigningNymdClient> {
    // maybe the wallet could be made into a generic, but for now, let's just have this one implementation
    pub fn connect_with_signer<U: Clone>(
        network: config::defaults::all::Network,
        endpoint: U,
        mixnet_contract_address: Option<AccountId>,
        vesting_contract_address: Option<AccountId>,
        erc20_bridge_contract_address: Option<AccountId>,
        signer: DirectSecp256k1HdWallet,
        gas_price: Option<GasPrice>,
    ) -> Result<NymdClient<SigningNymdClient>, NymdError>
    where
        U: TryInto<HttpClientUrl, Error = TendermintRpcError>,
    {
        let denom = network.denom();
        let client_address = signer
            .try_derive_accounts()?
            .into_iter()
            .map(|account| account.address)
            .collect();
        let gas_price = gas_price.unwrap_or(GasPrice::new_with_default_price(denom)?);

        Ok(NymdClient {
            client: SigningNymdClient::connect_with_signer(endpoint, signer, gas_price)?,
            mixnet_contract_address,
            vesting_contract_address,
            erc20_bridge_contract_address,
            client_address: Some(client_address),
            custom_gas_limits: HashMap::default(),
            simulated_gas_multiplier: DEFAULT_SIMULATED_GAS_MULTIPLIER,
        })
    }

    pub fn connect_with_mnemonic<U: Clone>(
        network: config::defaults::all::Network,
        endpoint: U,
        mixnet_contract_address: Option<AccountId>,
        vesting_contract_address: Option<AccountId>,
        erc20_bridge_contract_address: Option<AccountId>,
        mnemonic: bip39::Mnemonic,
        gas_price: Option<GasPrice>,
    ) -> Result<NymdClient<SigningNymdClient>, NymdError>
    where
        U: TryInto<HttpClientUrl, Error = TendermintRpcError>,
    {
        let prefix = network.bech32_prefix();
        let denom = network.denom();
        let wallet = DirectSecp256k1HdWallet::from_mnemonic(prefix, mnemonic)?;
        let client_address = wallet
            .try_derive_accounts()?
            .into_iter()
            .map(|account| account.address)
            .collect();
        let gas_price = gas_price.unwrap_or(GasPrice::new_with_default_price(denom)?);

        Ok(NymdClient {
            client: SigningNymdClient::connect_with_signer(endpoint, wallet, gas_price)?,
            mixnet_contract_address,
            vesting_contract_address,
            erc20_bridge_contract_address,
            client_address: Some(client_address),
            custom_gas_limits: HashMap::default(),
            simulated_gas_multiplier: DEFAULT_SIMULATED_GAS_MULTIPLIER,
        })
    }
}

impl<C> NymdClient<C> {
    pub fn mixnet_contract_address(&self) -> Result<&AccountId, NymdError> {
        self.mixnet_contract_address
            .as_ref()
            .ok_or(NymdError::NoContractAddressAvailable)
    }

    pub fn vesting_contract_address(&self) -> Result<&AccountId, NymdError> {
        self.vesting_contract_address
            .as_ref()
            .ok_or(NymdError::NoContractAddressAvailable)
    }

    pub fn erc20_bridge_contract_address(&self) -> Result<&AccountId, NymdError> {
        self.erc20_bridge_contract_address
            .as_ref()
            .ok_or(NymdError::NoContractAddressAvailable)
    }

    pub fn set_simulated_gas_multiplier(&mut self, multiplier: f32) {
        self.simulated_gas_multiplier = multiplier;
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

    pub fn set_custom_gas_limit(&mut self, operation: Operation, limit: Gas)
    where
        C: SigningCosmWasmClient + Sync,
    {
        self.custom_gas_limits.insert(operation, limit);
    }

    pub fn operation_fee(&self, operation: Operation) -> Fee
    where
        C: SigningCosmWasmClient + Sync,
    {
        if let Some(&gas_limit) = self.custom_gas_limits.get(&operation) {
            Operation::determine_custom_fee(self.client.gas_price(), gas_limit).into()
        } else {
            Fee::Auto(Some(self.simulated_gas_multiplier))
        }
    }

    pub fn repeated_operation_fee(&self, operation: Operation, count: u64) -> Fee
    where
        C: SigningCosmWasmClient + Sync,
    {
        if let Some(&gas_limit) = self.custom_gas_limits.get(&operation) {
            Operation::determine_custom_fee(
                self.client.gas_price(),
                (gas_limit.value() * count).into(),
            )
            .into()
        } else {
            Fee::Auto(Some(self.simulated_gas_multiplier))
        }
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
        Ok(self.client.get_block(None).await?.block.header.time)
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
        denom: Denom,
    ) -> Result<Option<CosmosCoin>, NymdError>
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
            .query_contract_smart(self.mixnet_contract_address()?, &request)
            .await
    }

    pub async fn get_operator_rewards(&self, address: String) -> Result<Uint128, NymdError>
    where
        C: CosmWasmClient + Sync,
    {
        let request = QueryMsg::QueryOperatorReward { address };
        self.client
            .query_contract_smart(self.mixnet_contract_address()?, &request)
            .await
    }

    pub async fn get_delegator_rewards(
        &self,
        address: String,
        mix_identity: IdentityKey,
    ) -> Result<Uint128, NymdError>
    where
        C: CosmWasmClient + Sync,
    {
        let request = QueryMsg::QueryDelegatorReward {
            address,
            mix_identity,
        };
        self.client
            .query_contract_smart(self.mixnet_contract_address()?, &request)
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
            .query_contract_smart(self.mixnet_contract_address()?, &request)
            .await
    }

    pub async fn get_current_epoch(&self) -> Result<Interval, NymdError>
    where
        C: CosmWasmClient + Sync,
    {
        let request = QueryMsg::GetCurrentEpoch {};
        self.client
            .query_contract_smart(self.mixnet_contract_address()?, &request)
            .await
    }

    pub async fn get_mixnet_contract_version(&self) -> Result<MixnetContractVersion, NymdError>
    where
        C: CosmWasmClient + Sync,
    {
        let request = QueryMsg::GetContractVersion {};
        self.client
            .query_contract_smart(self.mixnet_contract_address()?, &request)
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
            .query_contract_smart(self.mixnet_contract_address()?, &request)
            .await
    }

    pub async fn query_current_rewarded_set_height(&self) -> Result<u64, NymdError>
    where
        C: CosmWasmClient + Sync,
    {
        let request = QueryMsg::GetCurrentRewardedSetHeight {};
        self.client
            .query_contract_smart(self.mixnet_contract_address()?, &request)
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
            .query_contract_smart(self.mixnet_contract_address()?, &request)
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
            .query_contract_smart(self.mixnet_contract_address()?, &request)
            .await
    }

    pub async fn get_layer_distribution(&self) -> Result<LayerDistribution, NymdError>
    where
        C: CosmWasmClient + Sync,
    {
        let request = QueryMsg::LayerDistribution {};
        self.client
            .query_contract_smart(self.mixnet_contract_address()?, &request)
            .await
    }

    pub async fn get_reward_pool(&self) -> Result<Uint128, NymdError>
    where
        C: CosmWasmClient + Sync,
    {
        let request = QueryMsg::GetRewardPool {};
        self.client
            .query_contract_smart(self.mixnet_contract_address()?, &request)
            .await
    }

    pub async fn get_circulating_supply(&self) -> Result<Uint128, NymdError>
    where
        C: CosmWasmClient + Sync,
    {
        let request = QueryMsg::GetCirculatingSupply {};
        self.client
            .query_contract_smart(self.mixnet_contract_address()?, &request)
            .await
    }

    pub async fn get_sybil_resistance_percent(&self) -> Result<u8, NymdError>
    where
        C: CosmWasmClient + Sync,
    {
        let request = QueryMsg::GetSybilResistancePercent {};
        self.client
            .query_contract_smart(self.mixnet_contract_address()?, &request)
            .await
    }

    pub async fn get_active_set_work_factor(&self) -> Result<u8, NymdError>
    where
        C: CosmWasmClient + Sync,
    {
        let request = QueryMsg::GetActiveSetWorkFactor {};
        self.client
            .query_contract_smart(self.mixnet_contract_address()?, &request)
            .await
    }

    pub async fn get_interval_reward_percent(&self) -> Result<u8, NymdError>
    where
        C: CosmWasmClient + Sync,
    {
        let request = QueryMsg::GetIntervalRewardPercent {};
        self.client
            .query_contract_smart(self.mixnet_contract_address()?, &request)
            .await
    }

    pub async fn get_epochs_in_interval(&self) -> Result<u64, NymdError>
    where
        C: CosmWasmClient + Sync,
    {
        let request = QueryMsg::GetEpochsInInterval {};
        self.client
            .query_contract_smart(self.mixnet_contract_address()?, &request)
            .await
    }

    /// Checks whether there is a bonded mixnode associated with the provided client's address
    pub async fn owns_mixnode(&self, address: &AccountId) -> Result<Option<MixNodeBond>, NymdError>
    where
        C: CosmWasmClient + Sync,
    {
        let request = QueryMsg::OwnsMixnode {
            address: address.to_string(),
        };
        let response: MixOwnershipResponse = self
            .client
            .query_contract_smart(self.mixnet_contract_address()?, &request)
            .await?;
        Ok(response.mixnode)
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
            .query_contract_smart(self.mixnet_contract_address()?, &request)
            .await?;
        Ok(response.gateway)
    }

    pub async fn get_mixnodes_paged(
        &self,
        start_after: Option<IdentityKey>,
        page_limit: Option<u32>,
    ) -> Result<PagedMixnodeResponse, NymdError>
    where
        C: CosmWasmClient + Sync,
    {
        let request = QueryMsg::GetMixNodes {
            start_after,
            limit: page_limit,
        };
        self.client
            .query_contract_smart(self.mixnet_contract_address()?, &request)
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
            .query_contract_smart(self.mixnet_contract_address()?, &request)
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
            .query_contract_smart(self.mixnet_contract_address()?, &request)
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
            .query_contract_smart(self.mixnet_contract_address()?, &request)
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
            .query_contract_smart(self.mixnet_contract_address()?, &request)
            .await
    }

    /// Send funds from one address to another
    pub async fn send(
        &self,
        recipient: &AccountId,
        amount: Vec<CosmosCoin>,
        memo: impl Into<String> + Send + 'static,
    ) -> Result<broadcast::tx_commit::Response, NymdError>
    where
        C: SigningCosmWasmClient + Sync,
    {
        let fee = self.operation_fee(Operation::Send);
        self.client
            .send_tokens(self.address(), recipient, amount, fee, memo)
            .await
    }

    /// Send funds from one address to multiple others
    pub async fn send_multiple(
        &self,
        msgs: Vec<(AccountId, Vec<CosmosCoin>)>,
        memo: impl Into<String> + Send + 'static,
    ) -> Result<broadcast::tx_commit::Response, NymdError>
    where
        C: SigningCosmWasmClient + Sync,
    {
        let fee = self.repeated_operation_fee(Operation::Send, msgs.len() as u64);
        self.client
            .send_tokens_multiple(self.address(), msgs, fee, memo)
            .await
    }

    pub async fn execute<M>(
        &self,
        contract_address: &AccountId,
        msg: &M,
        fee: Fee,
        memo: impl Into<String> + Send + 'static,
        funds: Vec<CosmosCoin>,
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
        I: IntoIterator<Item = (M, Vec<CosmosCoin>)> + Send,
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
    ) -> Result<UploadResult, NymdError>
    where
        C: SigningCosmWasmClient + Sync,
    {
        let fee = self.operation_fee(Operation::Upload);
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
    ) -> Result<InstantiateResult, NymdError>
    where
        C: SigningCosmWasmClient + Sync,
        M: ?Sized + Serialize + Sync,
    {
        let fee = self.operation_fee(Operation::Init);
        self.client
            .instantiate(self.address(), code_id, msg, label, fee, memo, options)
            .await
    }

    pub async fn update_admin(
        &self,
        contract_address: &AccountId,
        new_admin: &AccountId,
        memo: impl Into<String> + Send + 'static,
    ) -> Result<ChangeAdminResult, NymdError>
    where
        C: SigningCosmWasmClient + Sync,
    {
        let fee = self.operation_fee(Operation::ChangeAdmin);
        self.client
            .update_admin(self.address(), contract_address, new_admin, fee, memo)
            .await
    }

    pub async fn clear_admin(
        &self,
        contract_address: &AccountId,
        memo: impl Into<String> + Send + 'static,
    ) -> Result<ChangeAdminResult, NymdError>
    where
        C: SigningCosmWasmClient + Sync,
    {
        let fee = self.operation_fee(Operation::ChangeAdmin);
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
    ) -> Result<MigrateResult, NymdError>
    where
        C: SigningCosmWasmClient + Sync,
        M: ?Sized + Serialize + Sync,
    {
        let fee = self.operation_fee(Operation::Migrate);
        self.client
            .migrate(self.address(), contract_address, code_id, fee, msg, memo)
            .await
    }

    /// Announce a mixnode, paying a fee.
    pub async fn bond_mixnode(
        &self,
        mixnode: MixNode,
        owner_signature: String,
        pledge: Coin,
    ) -> Result<ExecuteResult, NymdError>
    where
        C: SigningCosmWasmClient + Sync,
    {
        let fee = self.operation_fee(Operation::BondMixnode);

        let req = ExecuteMsg::BondMixnode {
            mix_node: mixnode,
            owner_signature,
        };
        self.client
            .execute(
                self.address(),
                self.mixnet_contract_address()?,
                &req,
                fee,
                "Bonding mixnode from rust!",
                vec![cosmwasm_coin_to_cosmos_coin(pledge)],
            )
            .await
    }

    /// Announce a mixnode on behalf of the owner, paying a fee.
    pub async fn bond_mixnode_on_behalf(
        &self,
        mixnode: MixNode,
        owner: String,
        owner_signature: String,
        pledge: Coin,
    ) -> Result<ExecuteResult, NymdError>
    where
        C: SigningCosmWasmClient + Sync,
    {
        let fee = self.operation_fee(Operation::BondMixnodeOnBehalf);

        let req = ExecuteMsg::BondMixnodeOnBehalf {
            mix_node: mixnode,
            owner,
            owner_signature,
        };
        self.client
            .execute(
                self.address(),
                self.mixnet_contract_address()?,
                &req,
                fee,
                "Bonding mixnode on behalf from rust!",
                vec![cosmwasm_coin_to_cosmos_coin(pledge)],
            )
            .await
    }

    /// Announce multiple mixnodes on behalf of other owners, paying a fee.
    pub async fn bond_multiple_mixnodes_on_behalf(
        &self,
        mixnode_bonds_with_sigs: Vec<(MixNodeBond, String)>,
    ) -> Result<ExecuteResult, NymdError>
    where
        C: SigningCosmWasmClient + Sync,
    {
        let fee = self.repeated_operation_fee(
            Operation::BondMixnodeOnBehalf,
            mixnode_bonds_with_sigs.len() as u64,
        );

        let reqs: Vec<(ExecuteMsg, Vec<CosmosCoin>)> = mixnode_bonds_with_sigs
            .into_iter()
            .map(|(bond, owner_signature)| {
                (
                    ExecuteMsg::BondMixnodeOnBehalf {
                        mix_node: bond.mix_node,
                        owner: bond.owner.to_string(),
                        owner_signature,
                    },
                    vec![cosmwasm_coin_to_cosmos_coin(bond.pledge_amount)],
                )
            })
            .collect();

        self.client
            .execute_multiple(
                self.address(),
                self.mixnet_contract_address()?,
                reqs,
                fee,
                "Bonding multiple mixnodes on behalf from rust!",
            )
            .await
    }

    /// Unbond a mixnode, removing it from the network and reclaiming staked coins
    pub async fn unbond_mixnode(&self) -> Result<ExecuteResult, NymdError>
    where
        C: SigningCosmWasmClient + Sync,
    {
        let fee = self.operation_fee(Operation::UnbondMixnode);

        let req = ExecuteMsg::UnbondMixnode {};
        self.client
            .execute(
                self.address(),
                self.mixnet_contract_address()?,
                &req,
                fee,
                "Unbonding mixnode from rust!",
                Vec::new(),
            )
            .await
    }

    /// Unbond a mixnode on behalf of the owner, removing it from the network and reclaiming staked coins
    pub async fn unbond_mixnode_on_behalf(&self, owner: String) -> Result<ExecuteResult, NymdError>
    where
        C: SigningCosmWasmClient + Sync,
    {
        let fee = self.operation_fee(Operation::UnbondMixnodeOnBehalf);

        let req = ExecuteMsg::UnbondMixnodeOnBehalf { owner };
        self.client
            .execute(
                self.address(),
                self.mixnet_contract_address()?,
                &req,
                fee,
                "Unbonding mixnode on behalf from rust!",
                Vec::new(),
            )
            .await
    }

    /// Update the configuration of a mixnode. Right now, only possible for profit margin.
    pub async fn update_mixnode_config(
        &self,
        profit_margin_percent: u8,
    ) -> Result<ExecuteResult, NymdError>
    where
        C: SigningCosmWasmClient + Sync,
    {
        let fee = self.operation_fee(Operation::UpdateMixnodeConfig);

        let req = ExecuteMsg::UpdateMixnodeConfig {
            profit_margin_percent,
        };
        self.client
            .execute(
                self.address(),
                self.mixnet_contract_address()?,
                &req,
                fee,
                "Updating mixnode configuration from rust!",
                Vec::new(),
            )
            .await
    }

    /// Delegates specified amount of stake to particular mixnode.
    pub async fn delegate_to_mixnode(
        &self,
        mix_identity: &str,
        amount: &Coin,
    ) -> Result<ExecuteResult, NymdError>
    where
        C: SigningCosmWasmClient + Sync,
    {
        let fee = self.operation_fee(Operation::DelegateToMixnode);

        let req = ExecuteMsg::DelegateToMixnode {
            mix_identity: mix_identity.to_string(),
        };
        self.client
            .execute(
                self.address(),
                self.mixnet_contract_address()?,
                &req,
                fee,
                "Delegating to mixnode from rust!",
                vec![cosmwasm_coin_ptr_to_cosmos_coin(amount)],
            )
            .await
    }

    /// Delegates specified amount of stake to particular mixnode on
    /// behalf of a particular delegator.
    pub async fn delegate_to_mixnode_on_behalf(
        &self,
        mix_identity: &str,
        delegate: &str,
        amount: &Coin,
    ) -> Result<ExecuteResult, NymdError>
    where
        C: SigningCosmWasmClient + Sync,
    {
        let fee = self.operation_fee(Operation::DelegateToMixnodeOnBehalf);

        let req = ExecuteMsg::DelegateToMixnodeOnBehalf {
            mix_identity: mix_identity.to_string(),
            delegate: delegate.to_string(),
        };
        self.client
            .execute(
                self.address(),
                self.mixnet_contract_address()?,
                &req,
                fee,
                "Delegating to mixnode on behalf from rust!",
                vec![cosmwasm_coin_ptr_to_cosmos_coin(amount)],
            )
            .await
    }

    /// Delegates specified amount of stake to multiple mixnodes on behalf of multiple delegators.
    pub async fn delegate_to_multiple_mixnodes_on_behalf(
        &self,
        mixnode_delegations: Vec<Delegation>,
    ) -> Result<ExecuteResult, NymdError>
    where
        C: SigningCosmWasmClient + Sync,
    {
        let fee = self.repeated_operation_fee(
            Operation::DelegateToMixnodeOnBehalf,
            mixnode_delegations.len() as u64,
        );

        let reqs: Vec<(ExecuteMsg, Vec<CosmosCoin>)> = mixnode_delegations
            .into_iter()
            .map(|delegation| {
                (
                    ExecuteMsg::DelegateToMixnodeOnBehalf {
                        mix_identity: delegation.node_identity(),
                        delegate: delegation.owner().to_string(),
                    },
                    vec![cosmwasm_coin_to_cosmos_coin(delegation.amount().clone())],
                )
            })
            .collect();

        self.client
            .execute_multiple(
                self.address(),
                self.mixnet_contract_address()?,
                reqs,
                fee,
                "Delegating to multiple mixnodes on behalf from rust!",
            )
            .await
    }

    /// Removes stake delegation from a particular mixnode.
    pub async fn remove_mixnode_delegation(
        &self,
        mix_identity: &str,
    ) -> Result<ExecuteResult, NymdError>
    where
        C: SigningCosmWasmClient + Sync,
    {
        let fee = self.operation_fee(Operation::UndelegateFromMixnode);

        let req = ExecuteMsg::UndelegateFromMixnode {
            mix_identity: mix_identity.to_string(),
        };
        self.client
            .execute(
                self.address(),
                self.mixnet_contract_address()?,
                &req,
                fee,
                "Removing mixnode delegation from rust!",
                Vec::new(),
            )
            .await
    }

    /// Removes stake delegation from a particular mixnode on behalf of a particular delegator.
    pub async fn remove_mixnode_delegation_on_behalf(
        &self,
        mix_identity: &str,
        delegate: &str,
    ) -> Result<ExecuteResult, NymdError>
    where
        C: SigningCosmWasmClient + Sync,
    {
        let fee = self.operation_fee(Operation::UndelegateFromMixnodeOnBehalf);

        let req = ExecuteMsg::UndelegateFromMixnodeOnBehalf {
            mix_identity: mix_identity.to_string(),
            delegate: delegate.to_string(),
        };
        self.client
            .execute(
                self.address(),
                self.mixnet_contract_address()?,
                &req,
                fee,
                "Removing mixnode delegation on behalf from rust!",
                Vec::new(),
            )
            .await
    }

    /// Announce a gateway, paying a fee.
    pub async fn bond_gateway(
        &self,
        gateway: Gateway,
        owner_signature: String,
        pledge: Coin,
    ) -> Result<ExecuteResult, NymdError>
    where
        C: SigningCosmWasmClient + Sync,
    {
        let fee = self.operation_fee(Operation::BondGateway);

        let req = ExecuteMsg::BondGateway {
            gateway,
            owner_signature,
        };
        self.client
            .execute(
                self.address(),
                self.mixnet_contract_address()?,
                &req,
                fee,
                "Bonding gateway from rust!",
                vec![cosmwasm_coin_to_cosmos_coin(pledge)],
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
    ) -> Result<ExecuteResult, NymdError>
    where
        C: SigningCosmWasmClient + Sync,
    {
        let fee = self.operation_fee(Operation::BondGatewayOnBehalf);

        let req = ExecuteMsg::BondGatewayOnBehalf {
            gateway,
            owner,
            owner_signature,
        };
        self.client
            .execute(
                self.address(),
                self.mixnet_contract_address()?,
                &req,
                fee,
                "Bonding gateway on behalf from rust!",
                vec![cosmwasm_coin_to_cosmos_coin(pledge)],
            )
            .await
    }

    /// Announce multiple gateways on behalf of other owners, paying a fee.
    pub async fn bond_multiple_gateways_on_behalf(
        &self,
        gateway_bonds_with_sigs: Vec<(GatewayBond, String)>,
    ) -> Result<ExecuteResult, NymdError>
    where
        C: SigningCosmWasmClient + Sync,
    {
        let fee = self.repeated_operation_fee(
            Operation::BondGatewayOnBehalf,
            gateway_bonds_with_sigs.len() as u64,
        );

        let reqs: Vec<(ExecuteMsg, Vec<CosmosCoin>)> = gateway_bonds_with_sigs
            .into_iter()
            .map(|(bond, owner_signature)| {
                (
                    ExecuteMsg::BondGatewayOnBehalf {
                        gateway: bond.gateway,
                        owner: bond.owner.to_string(),
                        owner_signature,
                    },
                    vec![cosmwasm_coin_to_cosmos_coin(bond.pledge_amount)],
                )
            })
            .collect();

        self.client
            .execute_multiple(
                self.address(),
                self.mixnet_contract_address()?,
                reqs,
                fee,
                "Bonding multiple gateways on behalf from rust!",
            )
            .await
    }

    /// Unbond a gateway, removing it from the network and reclaiming staked coins
    pub async fn unbond_gateway(&self) -> Result<ExecuteResult, NymdError>
    where
        C: SigningCosmWasmClient + Sync,
    {
        let fee = self.operation_fee(Operation::UnbondGateway);

        let req = ExecuteMsg::UnbondGateway {};
        self.client
            .execute(
                self.address(),
                self.mixnet_contract_address()?,
                &req,
                fee,
                "Unbonding gateway from rust!",
                Vec::new(),
            )
            .await
    }

    /// Unbond a gateway on behalf of the owner, removing it from the
    /// network and reclaiming staked coins
    pub async fn unbond_gateway_on_behalf(&self, owner: String) -> Result<ExecuteResult, NymdError>
    where
        C: SigningCosmWasmClient + Sync,
    {
        let fee = self.operation_fee(Operation::UnbondGatewayOnBehalf);

        let req = ExecuteMsg::UnbondGatewayOnBehalf { owner };
        self.client
            .execute(
                self.address(),
                self.mixnet_contract_address()?,
                &req,
                fee,
                "Unbonding gateway on behalf from rust!",
                Vec::new(),
            )
            .await
    }

    pub async fn update_contract_settings(
        &self,
        new_params: ContractStateParams,
    ) -> Result<ExecuteResult, NymdError>
    where
        C: SigningCosmWasmClient + Sync,
    {
        let fee = self.operation_fee(Operation::UpdateContractSettings);

        let req = ExecuteMsg::UpdateContractStateParams(new_params);
        self.client
            .execute(
                self.address(),
                self.mixnet_contract_address()?,
                &req,
                fee,
                "Updating contract state from rust!",
                Vec::new(),
            )
            .await
    }

    pub async fn advance_current_epoch(&self) -> Result<ExecuteResult, NymdError>
    where
        C: SigningCosmWasmClient + Sync,
    {
        let fee = self.operation_fee(Operation::AdvanceCurrentEpoch);

        let req = ExecuteMsg::AdvanceCurrentEpoch {};
        self.client
            .execute(
                self.address(),
                self.mixnet_contract_address()?,
                &req,
                fee,
                "Advance current epoch",
                Vec::new(),
            )
            .await
    }

    pub async fn reconcile_delegations(&self) -> Result<ExecuteResult, NymdError>
    where
        C: SigningCosmWasmClient + Sync,
    {
        let fee = self.operation_fee(Operation::ReconcileDelegations);

        let req = ExecuteMsg::ReconcileDelegations {};
        self.client
            .execute(
                self.address(),
                self.mixnet_contract_address()?,
                &req,
                fee,
                "Reconciling delegation events",
                Vec::new(),
            )
            .await
    }

    pub async fn checkpoint_mixnodes(&self) -> Result<ExecuteResult, NymdError>
    where
        C: SigningCosmWasmClient + Sync,
    {
        let fee = self.operation_fee(Operation::CheckpointMixnodes);

        let req = ExecuteMsg::CheckpointMixnodes {};
        self.client
            .execute(
                self.address(),
                self.mixnet_contract_address()?,
                &req,
                fee,
                "Snapshotting mixnodes",
                Vec::new(),
            )
            .await
    }

    pub async fn write_rewarded_set(
        &self,
        rewarded_set: Vec<IdentityKey>,
        expected_active_set_size: u32,
    ) -> Result<ExecuteResult, NymdError>
    where
        C: SigningCosmWasmClient + Sync,
    {
        let fee = self.operation_fee(Operation::WriteRewardedSet);

        let req = ExecuteMsg::WriteRewardedSet {
            rewarded_set,
            expected_active_set_size,
        };
        self.client
            .execute(
                self.address(),
                self.mixnet_contract_address()?,
                &req,
                fee,
                "Writing rewarded set",
                Vec::new(),
            )
            .await
    }
}

fn cosmwasm_coin_to_cosmos_coin(coin: Coin) -> CosmosCoin {
    CosmosCoin {
        denom: coin.denom.parse().unwrap(),
        // this might be a bit iffy, cosmwasm coin stores value as u128, while cosmos does it as u64
        amount: (coin.amount.u128() as u64).into(),
    }
}

fn cosmwasm_coin_ptr_to_cosmos_coin(coin: &Coin) -> CosmosCoin {
    CosmosCoin {
        denom: coin.denom.parse().unwrap(),
        // this might be a bit iffy, cosmwasm coin stores value as u128, while cosmos does it as u64
        amount: (coin.amount.u128() as u64).into(),
    }
}
