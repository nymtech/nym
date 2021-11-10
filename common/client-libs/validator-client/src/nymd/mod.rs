// Copyright 2021 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use crate::nymd::cosmwasm_client::signing_client;
use crate::nymd::cosmwasm_client::types::{
    ChangeAdminResult, ContractCodeId, ExecuteResult, InstantiateOptions, InstantiateResult,
    MigrateResult, SequenceResponse, UploadMeta, UploadResult,
};
use crate::nymd::error::NymdError;
use crate::nymd::fee_helpers::Operation;
use crate::nymd::wallet::DirectSecp256k1HdWallet;
use cosmrs::rpc::endpoint::broadcast;
use cosmrs::rpc::{Error as TendermintRpcError, HttpClientUrl};
use cosmwasm_std::{Coin, Uint128};
use mixnet_contract::{
    Addr, Delegation, ExecuteMsg, Gateway, GatewayOwnershipResponse, IdentityKey,
    LayerDistribution, MixNode, MixOwnershipResponse, PagedAllDelegationsResponse,
    PagedGatewayResponse, PagedMixDelegationsResponse, PagedMixnodeResponse,
    PagedReverseMixDelegationsResponse, QueryMsg, RawDelegationData, RewardingIntervalResponse,
    StateParams,
};
use serde::Serialize;
use std::collections::HashMap;
use std::convert::TryInto;

pub use crate::nymd::cosmwasm_client::client::CosmWasmClient;
pub use crate::nymd::cosmwasm_client::signing_client::SigningCosmWasmClient;
pub use crate::nymd::gas_price::GasPrice;
pub use cosmrs::rpc::HttpClient as QueryNymdClient;
pub use cosmrs::tendermint::block::Height;
pub use cosmrs::tendermint::hash;
pub use cosmrs::tendermint::Time as TendermintTime;
pub use cosmrs::tx::{Fee, Gas};
pub use cosmrs::Coin as CosmosCoin;
pub use cosmrs::{AccountId, Decimal, Denom};
pub use signing_client::Client as SigningNymdClient;

pub mod cosmwasm_client;
pub mod error;
pub mod fee_helpers;
pub mod gas_price;
pub mod wallet;

#[derive(Debug)]
pub struct NymdClient<C> {
    client: C,
    contract_address: Option<AccountId>,
    client_address: Option<Vec<AccountId>>,
    gas_price: GasPrice,
    custom_gas_limits: HashMap<Operation, Gas>,
}

impl NymdClient<QueryNymdClient> {
    pub fn connect<U>(
        endpoint: U,
        contract_address: AccountId,
    ) -> Result<NymdClient<QueryNymdClient>, NymdError>
    where
        U: TryInto<HttpClientUrl, Error = TendermintRpcError>,
    {
        Ok(NymdClient {
            client: QueryNymdClient::new(endpoint)?,
            contract_address: Some(contract_address),
            client_address: None,
            gas_price: Default::default(),
            custom_gas_limits: Default::default(),
        })
    }
}

impl NymdClient<SigningNymdClient> {
    // maybe the wallet could be made into a generic, but for now, let's just have this one implementation
    pub fn connect_with_signer<U>(
        endpoint: U,
        contract_address: Option<AccountId>,
        signer: DirectSecp256k1HdWallet,
    ) -> Result<NymdClient<SigningNymdClient>, NymdError>
    where
        U: TryInto<HttpClientUrl, Error = TendermintRpcError>,
    {
        let client_address = signer
            .try_derive_accounts()?
            .into_iter()
            .map(|account| account.address)
            .collect();

        Ok(NymdClient {
            client: SigningNymdClient::connect_with_signer(endpoint, signer)?,
            contract_address,
            client_address: Some(client_address),
            gas_price: Default::default(),
            custom_gas_limits: Default::default(),
        })
    }

    pub fn connect_with_mnemonic<U>(
        endpoint: U,
        contract_address: Option<AccountId>,
        mnemonic: bip39::Mnemonic,
    ) -> Result<NymdClient<SigningNymdClient>, NymdError>
    where
        U: TryInto<HttpClientUrl, Error = TendermintRpcError>,
    {
        let wallet = DirectSecp256k1HdWallet::from_mnemonic(mnemonic)?;
        let client_address = wallet
            .try_derive_accounts()?
            .into_iter()
            .map(|account| account.address)
            .collect();

        Ok(NymdClient {
            client: SigningNymdClient::connect_with_signer(endpoint, wallet)?,
            contract_address,
            client_address: Some(client_address),
            gas_price: Default::default(),
            custom_gas_limits: Default::default(),
        })
    }
}

impl<C> NymdClient<C> {
    pub fn set_gas_price(&mut self, gas_price: GasPrice) {
        self.gas_price = gas_price
    }

    pub fn set_custom_gas_limit(&mut self, operation: Operation, limit: Gas) {
        self.custom_gas_limits.insert(operation, limit);
    }

    pub fn get_gas_price(&self) -> GasPrice {
        self.gas_price.clone()
    }

    pub fn get_custom_gas_limits(&self) -> HashMap<Operation, Gas> {
        self.custom_gas_limits.clone()
    }

    pub fn contract_address(&self) -> Result<&AccountId, NymdError> {
        self.contract_address
            .as_ref()
            .ok_or(NymdError::NoContractAddressAvailable)
    }

    // now the question is as follows: will denom always be in the format of `u{prefix}`?
    pub fn denom(&self) -> Result<Denom, NymdError> {
        Ok(format!("u{}", self.contract_address()?.prefix())
            .parse()
            .unwrap())
    }

    pub fn address(&self) -> &AccountId
    where
        C: SigningCosmWasmClient,
    {
        // if this is a signing client (as required by the trait bound), it must have the address set
        &self.client_address.as_ref().unwrap()[0]
    }

    pub async fn account_sequence(&self) -> Result<SequenceResponse, NymdError>
    where
        C: SigningCosmWasmClient + Sync,
    {
        self.client.get_sequence(self.address()).await
    }

    pub fn get_fee(&self, operation: Operation) -> Fee {
        let gas_limit = self.custom_gas_limits.get(&operation).cloned();
        operation.determine_fee(&self.gas_price, gas_limit)
    }

    pub fn calculate_custom_fee(&self, gas_limit: impl Into<Gas>) -> Fee {
        Operation::determine_custom_fee(&self.gas_price, gas_limit.into())
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

    pub async fn get_balance(&self, address: &AccountId) -> Result<Option<CosmosCoin>, NymdError>
    where
        C: CosmWasmClient + Sync,
    {
        self.client.get_balance(address, self.denom()?).await
    }

    pub async fn get_state_params(&self) -> Result<StateParams, NymdError>
    where
        C: CosmWasmClient + Sync,
    {
        let request = QueryMsg::StateParams {};
        self.client
            .query_contract_smart(self.contract_address()?, &request)
            .await
    }

    pub async fn get_current_rewarding_interval(
        &self,
    ) -> Result<RewardingIntervalResponse, NymdError>
    where
        C: CosmWasmClient + Sync,
    {
        let request = QueryMsg::CurrentRewardingInterval {};
        self.client
            .query_contract_smart(self.contract_address()?, &request)
            .await
    }

    pub async fn get_layer_distribution(&self) -> Result<LayerDistribution, NymdError>
    where
        C: CosmWasmClient + Sync,
    {
        let request = QueryMsg::LayerDistribution {};
        self.client
            .query_contract_smart(self.contract_address()?, &request)
            .await
    }

    pub async fn get_reward_pool(&self) -> Result<Uint128, NymdError>
    where
        C: CosmWasmClient + Sync,
    {
        let request = QueryMsg::GetRewardPool {};
        self.client
            .query_contract_smart(self.contract_address()?, &request)
            .await
    }

    pub async fn get_circulating_supply(&self) -> Result<Uint128, NymdError>
    where
        C: CosmWasmClient + Sync,
    {
        let request = QueryMsg::GetCirculatingSupply {};
        self.client
            .query_contract_smart(self.contract_address()?, &request)
            .await
    }

    pub async fn get_sybil_resistance_percent(&self) -> Result<u8, NymdError>
    where
        C: CosmWasmClient + Sync,
    {
        let request = QueryMsg::GetSybilResistancePercent {};
        self.client
            .query_contract_smart(self.contract_address()?, &request)
            .await
    }

    pub async fn get_epoch_reward_percent(&self) -> Result<u8, NymdError>
    where
        C: CosmWasmClient + Sync,
    {
        let request = QueryMsg::GetEpochRewardPercent {};
        self.client
            .query_contract_smart(self.contract_address()?, &request)
            .await
    }

    /// Checks whether there is a bonded mixnode associated with the provided client's address
    pub async fn owns_mixnode(&self, address: &AccountId) -> Result<bool, NymdError>
    where
        C: CosmWasmClient + Sync,
    {
        let request = QueryMsg::OwnsMixnode {
            address: Addr::unchecked(address.as_ref()),
        };
        let response: MixOwnershipResponse = self
            .client
            .query_contract_smart(self.contract_address()?, &request)
            .await?;
        Ok(response.has_node)
    }

    /// Checks whether there is a bonded gateway associated with the provided client's address
    pub async fn owns_gateway(&self, address: &AccountId) -> Result<bool, NymdError>
    where
        C: CosmWasmClient + Sync,
    {
        let request = QueryMsg::OwnsGateway {
            address: Addr::unchecked(address.as_ref()),
        };
        let response: GatewayOwnershipResponse = self
            .client
            .query_contract_smart(self.contract_address()?, &request)
            .await?;
        Ok(response.has_gateway)
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
            .query_contract_smart(self.contract_address()?, &request)
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
            .query_contract_smart(self.contract_address()?, &request)
            .await
    }

    /// Gets list of all delegations towards particular mixnode on particular page.
    pub async fn get_mix_delegations_paged(
        &self,
        mix_identity: IdentityKey,
        // I really hate mixing cosmwasm and cosmos-sdk types here...
        start_after: Option<Addr>,
        page_limit: Option<u32>,
    ) -> Result<PagedMixDelegationsResponse, NymdError>
    where
        C: CosmWasmClient + Sync,
    {
        let request = QueryMsg::GetMixDelegations {
            mix_identity: mix_identity.to_owned(),
            start_after,
            limit: page_limit,
        };
        self.client
            .query_contract_smart(self.contract_address()?, &request)
            .await
    }

    /// Gets list of all mixnode delegations on particular page.
    pub async fn get_all_mix_delegations_paged(
        &self,
        // I really hate mixing cosmwasm and cosmos-sdk types here...
        start_after: Option<Vec<u8>>,
        page_limit: Option<u32>,
    ) -> Result<PagedAllDelegationsResponse<RawDelegationData>, NymdError>
    where
        C: CosmWasmClient + Sync,
    {
        let request = QueryMsg::GetAllMixDelegations {
            start_after,
            limit: page_limit,
        };
        self.client
            .query_contract_smart(self.contract_address()?, &request)
            .await
    }

    /// Gets list of all the mixnodes on which a particular address delegated.
    pub async fn get_reverse_mix_delegations_paged(
        &self,
        delegation_owner: Addr,
        start_after: Option<IdentityKey>,
        page_limit: Option<u32>,
    ) -> Result<PagedReverseMixDelegationsResponse, NymdError>
    where
        C: CosmWasmClient + Sync,
    {
        let request = QueryMsg::GetReverseMixDelegations {
            delegation_owner,
            start_after,
            limit: page_limit,
        };
        self.client
            .query_contract_smart(self.contract_address()?, &request)
            .await
    }

    /// Checks value of delegation of given client towards particular mixnode.
    pub async fn get_mix_delegation(
        &self,
        mix_identity: IdentityKey,
        delegator: &AccountId,
    ) -> Result<Delegation, NymdError>
    where
        C: CosmWasmClient + Sync,
    {
        let request = QueryMsg::GetMixDelegation {
            mix_identity,
            address: Addr::unchecked(delegator.as_ref()),
        };
        self.client
            .query_contract_smart(self.contract_address()?, &request)
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
        let fee = self.get_fee(Operation::Send);
        self.client
            .send_tokens(self.address(), recipient, amount, fee, memo)
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
        meta: Option<UploadMeta>,
    ) -> Result<UploadResult, NymdError>
    where
        C: SigningCosmWasmClient + Sync,
    {
        let fee = self.get_fee(Operation::Upload);
        self.client
            .upload(self.address(), wasm_code, fee, memo, meta)
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
        let fee = self.get_fee(Operation::Init);
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
        let fee = self.get_fee(Operation::ChangeAdmin);
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
        let fee = self.get_fee(Operation::ChangeAdmin);
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
        let fee = self.get_fee(Operation::Migrate);
        self.client
            .migrate(self.address(), contract_address, code_id, fee, msg, memo)
            .await
    }

    /// Announce a mixnode, paying a fee.
    pub async fn bond_mixnode(
        &self,
        mixnode: MixNode,
        bond: Coin,
    ) -> Result<ExecuteResult, NymdError>
    where
        C: SigningCosmWasmClient + Sync,
    {
        let fee = self.get_fee(Operation::BondMixnode);

        let req = ExecuteMsg::BondMixnode { mix_node: mixnode };
        self.client
            .execute(
                self.address(),
                self.contract_address()?,
                &req,
                fee,
                "Bonding mixnode from rust!",
                vec![cosmwasm_coin_to_cosmos_coin(bond)],
            )
            .await
    }

    /// Unbond a mixnode, removing it from the network and reclaiming staked coins
    pub async fn unbond_mixnode(&self) -> Result<ExecuteResult, NymdError>
    where
        C: SigningCosmWasmClient + Sync,
    {
        let fee = self.get_fee(Operation::UnbondMixnode);

        let req = ExecuteMsg::UnbondMixnode {};
        self.client
            .execute(
                self.address(),
                self.contract_address()?,
                &req,
                fee,
                "Unbonding mixnode from rust!",
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
        let fee = self.get_fee(Operation::DelegateToMixnode);

        let req = ExecuteMsg::DelegateToMixnode {
            mix_identity: mix_identity.to_string(),
        };
        self.client
            .execute(
                self.address(),
                self.contract_address()?,
                &req,
                fee,
                "Delegating to mixnode from rust!",
                vec![cosmwasm_coin_ptr_to_cosmos_coin(amount)],
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
        let fee = self.get_fee(Operation::UndelegateFromMixnode);

        let req = ExecuteMsg::UndelegateFromMixnode {
            mix_identity: mix_identity.to_string(),
        };
        self.client
            .execute(
                self.address(),
                self.contract_address()?,
                &req,
                fee,
                "Removing mixnode delegation from rust!",
                Vec::new(),
            )
            .await
    }

    /// Announce a gateway, paying a fee.
    pub async fn bond_gateway(
        &self,
        gateway: Gateway,
        bond: Coin,
    ) -> Result<ExecuteResult, NymdError>
    where
        C: SigningCosmWasmClient + Sync,
    {
        let fee = self.get_fee(Operation::BondGateway);

        let req = ExecuteMsg::BondGateway { gateway };
        self.client
            .execute(
                self.address(),
                self.contract_address()?,
                &req,
                fee,
                "Bonding gateway from rust!",
                vec![cosmwasm_coin_to_cosmos_coin(bond)],
            )
            .await
    }

    /// Unbond a gateway, removing it from the network and reclaiming staked coins
    pub async fn unbond_gateway(&self) -> Result<ExecuteResult, NymdError>
    where
        C: SigningCosmWasmClient + Sync,
    {
        let fee = self.get_fee(Operation::UnbondGateway);

        let req = ExecuteMsg::UnbondGateway {};
        self.client
            .execute(
                self.address(),
                self.contract_address()?,
                &req,
                fee,
                "Unbonding gateway from rust!",
                Vec::new(),
            )
            .await
    }

    pub async fn update_state_params(
        &self,
        new_params: StateParams,
    ) -> Result<ExecuteResult, NymdError>
    where
        C: SigningCosmWasmClient + Sync,
    {
        let fee = self.get_fee(Operation::UpdateStateParams);

        let req = ExecuteMsg::UpdateStateParams(new_params);
        self.client
            .execute(
                self.address(),
                self.contract_address()?,
                &req,
                fee,
                "Updating contract state from rust!",
                Vec::new(),
            )
            .await
    }

    pub async fn begin_mixnode_rewarding(
        &self,
        rewarding_interval_nonce: u32,
    ) -> Result<ExecuteResult, NymdError>
    where
        C: SigningCosmWasmClient + Sync,
    {
        let fee = self.get_fee(Operation::BeginMixnodeRewarding);

        let req = ExecuteMsg::BeginMixnodeRewarding {
            rewarding_interval_nonce,
        };
        self.client
            .execute(
                self.address(),
                self.contract_address()?,
                &req,
                fee,
                "Beginning mixnode rewarding procedure",
                Vec::new(),
            )
            .await
    }

    pub async fn finish_mixnode_rewarding(
        &self,
        rewarding_interval_nonce: u32,
    ) -> Result<ExecuteResult, NymdError>
    where
        C: SigningCosmWasmClient + Sync,
    {
        let fee = self.get_fee(Operation::FinishMixnodeRewarding);

        let req = ExecuteMsg::FinishMixnodeRewarding {
            rewarding_interval_nonce,
        };
        self.client
            .execute(
                self.address(),
                self.contract_address()?,
                &req,
                fee,
                "Finishing mixnode rewarding procedure",
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
