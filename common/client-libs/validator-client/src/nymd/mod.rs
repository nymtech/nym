// Copyright 2021 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use crate::nymd::cosmwasm_client::client::CosmWasmClient;
use crate::nymd::cosmwasm_client::signing_client;
use crate::nymd::cosmwasm_client::signing_client::SigningCosmWasmClient;
use crate::nymd::cosmwasm_client::types::{
    ChangeAdminResult, ContractCodeId, ExecuteResult, InstantiateOptions, InstantiateResult,
    MigrateResult, UploadMeta, UploadResult,
};
use crate::nymd::fee_helpers::Operation;
pub use crate::nymd::gas_price::GasPrice;
use crate::nymd::wallet::DirectSecp256k1HdWallet;
use crate::ValidatorClientError;
use cosmos_sdk::rpc::endpoint::broadcast;
use cosmos_sdk::rpc::{Error as TendermintRpcError, HttpClient, HttpClientUrl};
use cosmos_sdk::tx::{Fee, Gas};
use cosmos_sdk::Coin as CosmosCoin;
use cosmos_sdk::{AccountId, Denom};
use cosmwasm_std::Coin;
use mixnet_contract::{
    Addr, Delegation, ExecuteMsg, Gateway, GatewayOwnershipResponse, IdentityKey,
    LayerDistribution, MixNode, MixOwnershipResponse, PagedGatewayDelegationsResponse,
    PagedGatewayResponse, PagedMixDelegationsResponse, PagedMixnodeResponse, QueryMsg, StateParams,
};
use serde::Serialize;
use std::collections::HashMap;
use std::convert::TryInto;

pub mod cosmwasm_client;
pub(crate) mod fee_helpers;
pub mod gas_price;
pub mod wallet;

pub struct NymdClient<C> {
    client: C,
    contract_address: Option<AccountId>,
    client_address: Option<Vec<AccountId>>,
    gas_price: GasPrice,
    custom_gas_limits: HashMap<Operation, Gas>,
}

impl NymdClient<HttpClient> {
    pub fn connect<U>(
        endpoint: U,
        contract_address: AccountId,
    ) -> Result<NymdClient<HttpClient>, ValidatorClientError>
    where
        U: TryInto<HttpClientUrl, Error = TendermintRpcError>,
    {
        Ok(NymdClient {
            client: HttpClient::new(endpoint)?,
            contract_address: Some(contract_address),
            client_address: None,
            gas_price: Default::default(),
            custom_gas_limits: Default::default(),
        })
    }
}

impl NymdClient<signing_client::Client> {
    // maybe the wallet could be made into a generic, but for now, let's just have this one implementation
    pub fn connect_with_signer<U>(
        endpoint: U,
        contract_address: Option<AccountId>,
        signer: DirectSecp256k1HdWallet,
    ) -> Result<NymdClient<signing_client::Client>, ValidatorClientError>
    where
        U: TryInto<HttpClientUrl, Error = TendermintRpcError>,
    {
        let client_address = signer
            .try_derive_accounts()?
            .into_iter()
            .map(|account| account.address)
            .collect();

        Ok(NymdClient {
            client: signing_client::Client::connect_with_signer(endpoint, signer)?,
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
    ) -> Result<NymdClient<signing_client::Client>, ValidatorClientError>
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
            client: signing_client::Client::connect_with_signer(endpoint, wallet)?,
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

    pub fn contract_address(&self) -> Result<&AccountId, ValidatorClientError> {
        self.contract_address
            .as_ref()
            .ok_or(ValidatorClientError::NoContractAddressAvailable)
    }

    // now the question is as follows: will denom always be in the format of `u{prefix}`?
    pub fn denom(&self) -> Result<Denom, ValidatorClientError> {
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

    fn get_fee(&self, operation: Operation) -> Fee {
        let gas_limit = self.custom_gas_limits.get(&operation).cloned();
        operation.determine_fee(&self.gas_price, gas_limit)
    }

    pub async fn get_balance(
        &self,
        address: &AccountId,
    ) -> Result<Option<CosmosCoin>, ValidatorClientError>
    where
        C: CosmWasmClient + Sync,
    {
        self.client.get_balance(address, self.denom()?).await
    }

    pub async fn get_state_params(&self) -> Result<StateParams, ValidatorClientError>
    where
        C: CosmWasmClient + Sync,
    {
        let request = QueryMsg::StateParams {};
        self.client
            .query_contract_smart(self.contract_address()?, &request)
            .await
    }

    pub async fn get_layer_distribution(&self) -> Result<LayerDistribution, ValidatorClientError>
    where
        C: CosmWasmClient + Sync,
    {
        let request = QueryMsg::LayerDistribution {};
        self.client
            .query_contract_smart(self.contract_address()?, &request)
            .await
    }

    /// Checks whether there is a bonded mixnode associated with the provided client's address
    pub async fn owns_mixnode(&self, address: &AccountId) -> Result<bool, ValidatorClientError>
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
    pub async fn owns_gateway(&self, address: &AccountId) -> Result<bool, ValidatorClientError>
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
    ) -> Result<PagedMixnodeResponse, ValidatorClientError>
    where
        C: CosmWasmClient + Sync,
    {
        let request = QueryMsg::GetMixNodes {
            limit: None,
            start_after,
        };
        self.client
            .query_contract_smart(self.contract_address()?, &request)
            .await
    }

    pub async fn get_gateways_paged(
        &self,
        start_after: Option<IdentityKey>,
    ) -> Result<PagedGatewayResponse, ValidatorClientError>
    where
        C: CosmWasmClient + Sync,
    {
        let request = QueryMsg::GetGateways {
            limit: None,
            start_after,
        };
        self.client
            .query_contract_smart(self.contract_address()?, &request)
            .await
    }

    /// Gets list of all delegations towards particular mixnode on particular page.
    pub async fn get_mix_delegations_paged(
        &self,
        mix_identity: IdentityKey,
        start_after: Option<AccountId>,
    ) -> Result<PagedMixDelegationsResponse, ValidatorClientError>
    where
        C: CosmWasmClient + Sync,
    {
        let request = QueryMsg::GetMixDelegations {
            mix_identity: mix_identity.to_owned(),
            start_after: start_after.map(|addr| Addr::unchecked(addr.as_ref())),
            limit: None,
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
    ) -> Result<Delegation, ValidatorClientError>
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

    /// Gets list of all delegations towards particular mixnode on particular page.
    pub async fn get_gateway_delegations(
        &self,
        gateway_identity: IdentityKey,
        start_after: Option<AccountId>,
    ) -> Result<PagedGatewayDelegationsResponse, ValidatorClientError>
    where
        C: CosmWasmClient + Sync,
    {
        let request = QueryMsg::GetGatewayDelegations {
            gateway_identity,
            start_after: start_after.map(|addr| Addr::unchecked(addr.as_ref())),
            limit: None,
        };
        self.client
            .query_contract_smart(self.contract_address()?, &request)
            .await
    }

    /// Checks value of delegation of given client towards particular gateway.
    pub async fn get_gateway_delegation(
        &self,
        gateway_identity: IdentityKey,
        delegator: &AccountId,
    ) -> Result<Delegation, ValidatorClientError>
    where
        C: CosmWasmClient + Sync,
    {
        let request = QueryMsg::GetGatewayDelegation {
            gateway_identity,
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
    ) -> Result<broadcast::tx_commit::Response, ValidatorClientError>
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
    ) -> Result<ExecuteResult, ValidatorClientError>
    where
        C: SigningCosmWasmClient + Sync,
        M: ?Sized + Serialize + Sync,
    {
        self.client
            .execute(self.address(), contract_address, msg, fee, memo, funds)
            .await
    }

    pub async fn upload(
        &self,
        wasm_code: Vec<u8>,
        memo: impl Into<String> + Send + 'static,
        meta: Option<UploadMeta>,
    ) -> Result<UploadResult, ValidatorClientError>
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
    ) -> Result<InstantiateResult, ValidatorClientError>
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
    ) -> Result<ChangeAdminResult, ValidatorClientError>
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
    ) -> Result<ChangeAdminResult, ValidatorClientError>
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
    ) -> Result<MigrateResult, ValidatorClientError>
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
    ) -> Result<ExecuteResult, ValidatorClientError>
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
    pub async fn unbond_mixnode(&self) -> Result<ExecuteResult, ValidatorClientError>
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
        mix_identity: IdentityKey,
        amount: Coin,
    ) -> Result<ExecuteResult, ValidatorClientError>
    where
        C: SigningCosmWasmClient + Sync,
    {
        let fee = self.get_fee(Operation::DelegateToMixnode);

        let req = ExecuteMsg::DelegateToMixnode { mix_identity };
        self.client
            .execute(
                self.address(),
                self.contract_address()?,
                &req,
                fee,
                "Delegating to mixnode from rust!",
                vec![cosmwasm_coin_to_cosmos_coin(amount)],
            )
            .await
    }

    /// Removes stake delegation from a particular mixnode.
    pub async fn remove_mixnode_delegation(
        &self,
        mix_identity: IdentityKey,
    ) -> Result<ExecuteResult, ValidatorClientError>
    where
        C: SigningCosmWasmClient + Sync,
    {
        let fee = self.get_fee(Operation::UndelegateFromMixnode);

        let req = ExecuteMsg::UndelegateFromMixnode { mix_identity };
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
    ) -> Result<ExecuteResult, ValidatorClientError>
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
    pub async fn unbond_gateway(&self) -> Result<ExecuteResult, ValidatorClientError>
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

    /// Delegates specified amount of stake to particular gateway.
    pub async fn delegate_to_gateway(
        &self,
        gateway_identity: IdentityKey,
        amount: Coin,
    ) -> Result<ExecuteResult, ValidatorClientError>
    where
        C: SigningCosmWasmClient + Sync,
    {
        let fee = self.get_fee(Operation::DelegateToGateway);

        let req = ExecuteMsg::DelegateToGateway { gateway_identity };
        self.client
            .execute(
                self.address(),
                self.contract_address()?,
                &req,
                fee,
                "Delegating to gateway from rust!",
                vec![cosmwasm_coin_to_cosmos_coin(amount)],
            )
            .await
    }

    /// Removes stake delegation from a particular gateway.
    pub async fn remove_gateway_delegation(
        &self,
        gateway_identity: IdentityKey,
    ) -> Result<ExecuteResult, ValidatorClientError>
    where
        C: SigningCosmWasmClient + Sync,
    {
        let fee = self.get_fee(Operation::UndelegateFromGateway);

        let req = ExecuteMsg::UndelegateFromGateway { gateway_identity };
        self.client
            .execute(
                self.address(),
                self.contract_address()?,
                &req,
                fee,
                "Removing gateway delegation from rust!",
                Vec::new(),
            )
            .await
    }

    pub async fn update_state_params(
        &self,
        new_params: StateParams,
    ) -> Result<ExecuteResult, ValidatorClientError>
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
}

fn cosmwasm_coin_to_cosmos_coin(coin: Coin) -> CosmosCoin {
    CosmosCoin {
        denom: coin.denom.parse().unwrap(),
        // this might be a bit iffy, cosmwasm coin stores value as u128, while cosmos does it as u64
        amount: (coin.amount.u128() as u64).into(),
    }
}
