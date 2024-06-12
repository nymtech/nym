// Copyright 2021 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use crate::nyxd::contract_traits::{NymContractsProvider, TypedNymContracts};
use crate::nyxd::cosmwasm_client::types::{
    ChangeAdminResult, ContractCodeId, ExecuteResult, InstantiateOptions, InstantiateResult,
    MigrateResult, SequenceResponse, SimulateResponse, UploadResult,
};
use crate::nyxd::cosmwasm_client::MaybeSigningClient;
use crate::nyxd::error::NyxdError;
use crate::nyxd::fee::DEFAULT_SIMULATED_GAS_MULTIPLIER;
use crate::signing::direct_wallet::DirectSecp256k1HdWallet;
use crate::signing::signer::NoSigner;
use crate::signing::signer::OfflineSigner;
use crate::signing::tx_signer::TxSigner;
use crate::signing::AccountData;
use crate::{DirectSigningReqwestRpcNyxdClient, QueryReqwestRpcNyxdClient, ReqwestRpcClient};
use async_trait::async_trait;
use cosmrs::tendermint::{abci, evidence::Evidence, Genesis};
use cosmrs::tx::{Raw, SignDoc};
use cosmwasm_std::Addr;
use nym_network_defaults::{ChainDetails, NymNetworkDetails};
use serde::{de::DeserializeOwned, Serialize};
use std::fmt::Debug;
use std::time::SystemTime;
use tendermint_rpc::endpoint::block::Response as BlockResponse;
use tendermint_rpc::endpoint::*;
use tendermint_rpc::{Error as TendermintRpcError, Order};
use url::Url;

pub use crate::nyxd::{
    cosmwasm_client::{
        client_traits::{CosmWasmClient, SigningCosmWasmClient},
        module_traits::{self, StakingQueryClient},
    },
    fee::Fee,
};
pub use crate::rpc::TendermintRpcClient;
pub use coin::Coin;
pub use cosmrs::{
    bank::MsgSend,
    bip32, cosmwasm,
    crypto::PublicKey,
    query::{PageRequest, PageResponse},
    tendermint::{
        abci::{response::DeliverTx, types::ExecTxResult, Event, EventAttribute},
        block::Height,
        hash::{self, Algorithm, Hash},
        validator::Info as TendermintValidatorInfo,
        Time as TendermintTime,
    },
    tx::{self, Msg},
    AccountId, Any, Coin as CosmosCoin, Denom, Gas,
};
pub use cosmwasm_std::Coin as CosmWasmCoin;
pub use cw2;
pub use cw3;
pub use cw4;
pub use cw_controllers;
pub use fee::{gas_price::GasPrice, GasAdjustable, GasAdjustment};
pub use tendermint_rpc::{
    endpoint::{tx::Response as TxResponse, validators::Response as ValidatorResponse},
    query::Query,
    Paging, Request, Response, SimpleRequest,
};

#[cfg(feature = "http-client")]
use crate::http_client;
#[cfg(feature = "http-client")]
use crate::{DirectSigningHttpRpcNyxdClient, QueryHttpRpcNyxdClient};
#[cfg(feature = "http-client")]
use cosmrs::rpc::{HttpClient, HttpClientUrl};
use nym_contracts_common::build_information::CONTRACT_BUILD_INFO_STORAGE_KEY;
use nym_contracts_common::ContractBuildInformation;

pub mod coin;
pub mod contract_traits;
pub mod cosmwasm_client;
pub mod error;
pub mod fee;
pub mod helpers;

#[derive(Debug, Clone)]
pub struct Config {
    pub(crate) chain_details: ChainDetails,
    pub(crate) contracts: TypedNymContracts,
    pub(crate) gas_price: GasPrice,
    pub(crate) simulated_gas_multiplier: f32,
}

impl Config {
    pub fn try_from_nym_network_details(details: &NymNetworkDetails) -> Result<Self, NyxdError> {
        Ok(Config {
            chain_details: details.chain_details.clone(),
            contracts: TypedNymContracts::try_from(details.contracts.clone())?,
            gas_price: details.try_into()?,
            simulated_gas_multiplier: DEFAULT_SIMULATED_GAS_MULTIPLIER,
        })
    }

    pub fn with_simulated_gas_multplier(mut self, simulated_gas_multiplier: f32) -> Self {
        self.simulated_gas_multiplier = simulated_gas_multiplier;
        self
    }
}

impl TryFrom<NymNetworkDetails> for Config {
    type Error = NyxdError;

    fn try_from(value: NymNetworkDetails) -> Result<Self, Self::Error> {
        Config::try_from_nym_network_details(&value)
    }
}

#[derive(Debug)]
pub struct NyxdClient<C, S = NoSigner> {
    client: MaybeSigningClient<C, S>,
    config: Config,
}

// terrible name, but can't really change it because it will break so many uses
#[cfg(feature = "http-client")]
impl NyxdClient<HttpClient> {
    pub fn connect<U>(config: Config, endpoint: U) -> Result<QueryHttpRpcNyxdClient, NyxdError>
    where
        U: TryInto<HttpClientUrl, Error = TendermintRpcError>,
    {
        let client = http_client(endpoint)?;

        Ok(NyxdClient {
            client: MaybeSigningClient::new(client, (&config).into()),
            config,
        })
    }
}

impl NyxdClient<ReqwestRpcClient> {
    pub fn connect_reqwest(
        config: Config,
        endpoint: Url,
    ) -> Result<QueryReqwestRpcNyxdClient, NyxdError> {
        let client = ReqwestRpcClient::new(endpoint);

        Ok(NyxdClient {
            client: MaybeSigningClient::new(client, (&config).into()),
            config,
        })
    }
}

impl<C> NyxdClient<C> {
    pub fn new(config: Config, client: C) -> Self {
        NyxdClient {
            client: MaybeSigningClient::new(client, (&config).into()),
            config,
        }
    }
}

// terrible name, but can't really change it because it will break so many uses
#[cfg(feature = "http-client")]
impl NyxdClient<HttpClient, DirectSecp256k1HdWallet> {
    pub fn connect_with_mnemonic<U>(
        config: Config,
        endpoint: U,
        mnemonic: bip39::Mnemonic,
    ) -> Result<DirectSigningHttpRpcNyxdClient, NyxdError>
    where
        U: TryInto<HttpClientUrl, Error = TendermintRpcError>,
    {
        let client = http_client(endpoint)?;

        let prefix = &config.chain_details.bech32_account_prefix;
        let wallet = DirectSecp256k1HdWallet::from_mnemonic(prefix, mnemonic);
        Ok(Self::connect_with_signer(config, client, wallet))
    }
}

impl NyxdClient<ReqwestRpcClient, DirectSecp256k1HdWallet> {
    pub fn connect_reqwest_with_mnemonic(
        config: Config,
        endpoint: Url,
        mnemonic: bip39::Mnemonic,
    ) -> DirectSigningReqwestRpcNyxdClient {
        let client = ReqwestRpcClient::new(endpoint);

        let prefix = &config.chain_details.bech32_account_prefix;
        let wallet = DirectSecp256k1HdWallet::from_mnemonic(prefix, mnemonic);
        Self::connect_with_signer(config, client, wallet)
    }
}

impl<C, S> NyxdClient<C, S>
where
    S: OfflineSigner,
{
    pub fn connect_with_signer(config: Config, client: C, signer: S) -> NyxdClient<C, S> {
        NyxdClient {
            client: MaybeSigningClient::new_signing(client, signer, (&config).into()),
            config,
        }
    }
}

#[cfg(feature = "http-client")]
impl<S> NyxdClient<HttpClient, S> {
    pub fn change_endpoint<U>(&mut self, new_endpoint: U) -> Result<(), NyxdError>
    where
        U: TryInto<HttpClientUrl, Error = TendermintRpcError>,
    {
        self.client.change_endpoint(new_endpoint)
    }
}

// no trait bounds
impl<C, S> NyxdClient<C, S> {
    pub fn new_signing(config: Config, client: C, signer: S) -> Self
    where
        S: OfflineSigner,
    {
        NyxdClient {
            client: MaybeSigningClient::new_signing(client, signer, (&config).into()),
            config,
        }
    }

    pub fn current_config(&self) -> &Config {
        &self.config
    }

    pub fn current_chain_details(&self) -> &ChainDetails {
        &self.config.chain_details
    }

    pub fn set_mixnet_contract_address(&mut self, address: AccountId) {
        self.config.contracts.mixnet_contract_address = Some(address);
    }

    pub fn set_vesting_contract_address(&mut self, address: AccountId) {
        self.config.contracts.vesting_contract_address = Some(address);
    }

    pub fn set_ecash_contract_address(&mut self, address: AccountId) {
        self.config.contracts.ecash_contract_address = Some(address);
    }

    pub fn set_multisig_contract_address(&mut self, address: AccountId) {
        self.config.contracts.multisig_contract_address = Some(address);
    }

    pub fn set_simulated_gas_multiplier(&mut self, multiplier: f32) {
        self.config.simulated_gas_multiplier = multiplier;
    }
}

impl<C, S> NymContractsProvider for NyxdClient<C, S> {
    fn mixnet_contract_address(&self) -> Option<&AccountId> {
        self.config.contracts.mixnet_contract_address.as_ref()
    }

    fn vesting_contract_address(&self) -> Option<&AccountId> {
        self.config.contracts.vesting_contract_address.as_ref()
    }

    fn ecash_contract_address(&self) -> Option<&AccountId> {
        self.config.contracts.ecash_contract_address.as_ref()
    }

    fn dkg_contract_address(&self) -> Option<&AccountId> {
        self.config.contracts.coconut_dkg_contract_address.as_ref()
    }

    fn group_contract_address(&self) -> Option<&AccountId> {
        self.config.contracts.group_contract_address.as_ref()
    }

    fn multisig_contract_address(&self) -> Option<&AccountId> {
        self.config.contracts.multisig_contract_address.as_ref()
    }
}

// queries
impl<C, S> NyxdClient<C, S>
where
    C: TendermintRpcClient + Send + Sync,
    S: Send + Sync,
{
    pub async fn get_account_public_key(
        &self,
        address: &AccountId,
    ) -> Result<Option<cosmrs::crypto::PublicKey>, NyxdError> {
        if let Some(account) = self.client.get_account(address).await? {
            let base_account = account.try_get_base_account()?;
            return Ok(base_account.pubkey);
        }

        Ok(None)
    }

    pub async fn get_current_block_timestamp(&self) -> Result<TendermintTime, NyxdError> {
        self.get_block_timestamp(None).await
    }

    pub async fn get_block_timestamp(
        &self,
        height: Option<u32>,
    ) -> Result<TendermintTime, NyxdError> {
        Ok(self.client.get_block(height).await?.block.header.time)
    }

    pub async fn get_block(&self, height: Option<u32>) -> Result<BlockResponse, NyxdError> {
        self.client.get_block(height).await
    }

    pub async fn get_current_block_height(&self) -> Result<Height, NyxdError> {
        self.client.get_height().await
    }

    /// Obtains the hash of a block specified by the provided height.
    ///
    /// # Arguments
    ///
    /// * `height`: height of the block for which we want to obtain the hash.
    pub async fn get_block_hash(&self, height: u32) -> Result<Hash, NyxdError> {
        self.client
            .get_block(Some(height))
            .await
            .map(|block| block.block_id.hash)
    }

    pub async fn try_get_cw2_contract_version(
        &self,
        contract_address: &AccountId,
    ) -> Option<cw2::ContractVersion> {
        let raw_info = self
            .query_contract_raw(contract_address, b"contract_info".to_vec())
            .await
            .ok()?;

        serde_json::from_slice(&raw_info).ok()
    }

    pub async fn try_get_contract_build_information(
        &self,
        contract_address: &AccountId,
    ) -> Option<ContractBuildInformation> {
        let raw_info = self
            .query_contract_raw(
                contract_address,
                CONTRACT_BUILD_INFO_STORAGE_KEY.as_bytes().to_vec(),
            )
            .await
            .ok()?;

        serde_json::from_slice(&raw_info).ok()
    }
}

// signing
impl<C, S> NyxdClient<C, S>
where
    C: TendermintRpcClient + Send + Sync,
    S: OfflineSigner + Send + Sync,
    NyxdError: From<<S as OfflineSigner>::Error>,
{
    pub fn signing_account(&self) -> Result<AccountData, NyxdError> {
        Ok(self.find_account(&self.address())?)
    }

    pub fn address(&self) -> AccountId {
        match self.client.signer_addresses() {
            Ok(addresses) => addresses[0].clone(),
            Err(_) => {
                panic!("key derivation failure")
            }
        }
    }

    pub fn cw_address(&self) -> Addr {
        // the call to unchecked is fine here as we're converting directly from `AccountId`
        // which must have been a valid bech32 address
        Addr::unchecked(self.address().as_ref())
    }

    pub async fn account_sequence(&self) -> Result<SequenceResponse, NyxdError> {
        self.client.get_sequence(&self.address()).await
    }

    pub fn wrap_contract_execute_message<M>(
        &self,
        contract_address: &AccountId,
        msg: &M,
        funds: Vec<Coin>,
    ) -> Result<cosmwasm::MsgExecuteContract, NyxdError>
    where
        M: ?Sized + Serialize,
    {
        Ok(cosmwasm::MsgExecuteContract {
            sender: self.address(),
            contract: contract_address.clone(),
            msg: serde_json::to_vec(msg)?,
            funds: funds.into_iter().map(Into::into).collect(),
        })
    }

    pub async fn simulate<I, M>(
        &self,
        messages: I,
        memo: impl Into<String> + Send + 'static,
    ) -> Result<SimulateResponse, NyxdError>
    where
        I: IntoIterator<Item = M> + Send,
        M: Msg,
    {
        self.client
            .simulate(
                &self.address(),
                messages
                    .into_iter()
                    .map(|msg| msg.into_any())
                    .collect::<Result<Vec<_>, _>>()
                    .map_err(|_| {
                        NyxdError::SerializationError("custom simulate messages".to_owned())
                    })?,
                memo,
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
    ) -> Result<TxResponse, NyxdError> {
        let fee = fee.unwrap_or(Fee::Auto(Some(self.config.simulated_gas_multiplier)));
        self.client
            .send_tokens(&self.address(), recipient, amount, fee, memo)
            .await
    }

    /// Send funds from one address to multiple others
    pub async fn send_multiple(
        &self,
        msgs: Vec<(AccountId, Vec<Coin>)>,
        memo: impl Into<String> + Send + 'static,
        fee: Option<Fee>,
    ) -> Result<TxResponse, NyxdError> {
        let fee = fee.unwrap_or(Fee::Auto(Some(self.config.simulated_gas_multiplier)));
        self.client
            .send_tokens_multiple(&self.address(), msgs, fee, memo)
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
    ) -> Result<TxResponse, NyxdError> {
        let fee = fee.unwrap_or(Fee::Auto(Some(self.config.simulated_gas_multiplier)));
        self.client
            .grant_allowance(
                &self.address(),
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
    ) -> Result<TxResponse, NyxdError> {
        let fee = fee.unwrap_or(Fee::Auto(Some(self.config.simulated_gas_multiplier)));
        self.client
            .revoke_allowance(&self.address(), grantee, fee, memo)
            .await
    }

    pub async fn execute<M>(
        &self,
        contract_address: &AccountId,
        msg: &M,
        fee: Option<Fee>,
        memo: impl Into<String> + Send + 'static,
        funds: Vec<Coin>,
    ) -> Result<ExecuteResult, NyxdError>
    where
        M: ?Sized + Serialize + Sync,
    {
        let fee = fee.unwrap_or(Fee::Auto(Some(self.config.simulated_gas_multiplier)));
        self.client
            .execute(&self.address(), contract_address, msg, fee, memo, funds)
            .await
    }

    pub async fn execute_multiple<I, M>(
        &self,
        contract_address: &AccountId,
        msgs: I,
        fee: Option<Fee>,
        memo: impl Into<String> + Send + 'static,
    ) -> Result<ExecuteResult, NyxdError>
    where
        I: IntoIterator<Item = (M, Vec<Coin>)> + Send,
        M: Serialize,
    {
        let fee = fee.unwrap_or(Fee::Auto(Some(self.config.simulated_gas_multiplier)));
        self.client
            .execute_multiple(&self.address(), contract_address, msgs, fee, memo)
            .await
    }

    pub async fn upload(
        &self,
        wasm_code: Vec<u8>,
        memo: impl Into<String> + Send + 'static,
        fee: Option<Fee>,
    ) -> Result<UploadResult, NyxdError> {
        let fee = fee.unwrap_or(Fee::Auto(Some(self.config.simulated_gas_multiplier)));
        self.client
            .upload(&self.address(), wasm_code, fee, memo)
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
    ) -> Result<InstantiateResult, NyxdError>
    where
        M: ?Sized + Serialize + Sync,
    {
        let fee = fee.unwrap_or(Fee::Auto(Some(self.config.simulated_gas_multiplier)));
        self.client
            .instantiate(&self.address(), code_id, msg, label, fee, memo, options)
            .await
    }

    pub async fn update_admin(
        &self,
        contract_address: &AccountId,
        new_admin: &AccountId,
        memo: impl Into<String> + Send + 'static,
        fee: Option<Fee>,
    ) -> Result<ChangeAdminResult, NyxdError> {
        let fee = fee.unwrap_or(Fee::Auto(Some(self.config.simulated_gas_multiplier)));
        self.client
            .update_admin(&self.address(), contract_address, new_admin, fee, memo)
            .await
    }

    pub async fn clear_admin(
        &self,
        contract_address: &AccountId,
        memo: impl Into<String> + Send + 'static,
        fee: Option<Fee>,
    ) -> Result<ChangeAdminResult, NyxdError> {
        let fee = fee.unwrap_or(Fee::Auto(Some(self.config.simulated_gas_multiplier)));
        self.client
            .clear_admin(&self.address(), contract_address, fee, memo)
            .await
    }

    pub async fn migrate<M>(
        &self,
        contract_address: &AccountId,
        code_id: ContractCodeId,
        msg: &M,
        memo: impl Into<String> + Send + 'static,
        fee: Option<Fee>,
    ) -> Result<MigrateResult, NyxdError>
    where
        M: ?Sized + Serialize + Sync,
    {
        let fee = fee.unwrap_or(Fee::Auto(Some(self.config.simulated_gas_multiplier)));
        self.client
            .migrate(&self.address(), contract_address, code_id, fee, msg, memo)
            .await
    }
}

// ugh. is there a way to avoid that nasty trait implementation?

#[cfg_attr(target_arch = "wasm32", async_trait(?Send))]
#[cfg_attr(not(target_arch = "wasm32"), async_trait)]
impl<C, S> TendermintRpcClient for NyxdClient<C, S>
where
    C: TendermintRpcClient + Send + Sync,
    S: Send + Sync,
{
    async fn abci_info(&self) -> Result<abci::response::Info, TendermintRpcError> {
        self.client.abci_info().await
    }

    async fn abci_query<V>(
        &self,
        path: Option<String>,
        data: V,
        height: Option<Height>,
        prove: bool,
    ) -> Result<abci_query::AbciQuery, TendermintRpcError>
    where
        V: Into<Vec<u8>> + Send,
    {
        self.client.abci_query(path, data, height, prove).await
    }

    async fn block<H>(&self, height: H) -> Result<block::Response, TendermintRpcError>
    where
        H: Into<Height> + Send,
    {
        self.client.block(height).await
    }

    async fn block_by_hash(
        &self,
        hash: Hash,
    ) -> Result<block_by_hash::Response, TendermintRpcError> {
        self.client.block_by_hash(hash).await
    }

    async fn latest_block(&self) -> Result<block::Response, TendermintRpcError> {
        self.client.latest_block().await
    }

    async fn header<H>(&self, height: H) -> Result<header::Response, TendermintRpcError>
    where
        H: Into<Height> + Send,
    {
        self.client.header(height).await
    }

    async fn header_by_hash(
        &self,
        hash: Hash,
    ) -> Result<header_by_hash::Response, TendermintRpcError> {
        self.client.header_by_hash(hash).await
    }

    async fn block_results<H>(
        &self,
        height: H,
    ) -> Result<block_results::Response, TendermintRpcError>
    where
        H: Into<Height> + Send,
    {
        self.client.block_results(height).await
    }

    async fn latest_block_results(&self) -> Result<block_results::Response, TendermintRpcError> {
        self.client.latest_block_results().await
    }

    async fn block_search(
        &self,
        query: Query,
        page: u32,
        per_page: u8,
        order: Order,
    ) -> Result<block_search::Response, TendermintRpcError> {
        self.client.block_search(query, page, per_page, order).await
    }

    async fn blockchain<H>(
        &self,
        min: H,
        max: H,
    ) -> Result<blockchain::Response, TendermintRpcError>
    where
        H: Into<Height> + Send,
    {
        self.client.blockchain(min, max).await
    }

    async fn broadcast_tx_async<T>(
        &self,
        tx: T,
    ) -> Result<broadcast::tx_async::Response, TendermintRpcError>
    where
        T: Into<Vec<u8>> + Send,
    {
        TendermintRpcClient::broadcast_tx_async(&self.client, tx).await
    }

    async fn broadcast_tx_sync<T>(
        &self,
        tx: T,
    ) -> Result<broadcast::tx_sync::Response, TendermintRpcError>
    where
        T: Into<Vec<u8>> + Send,
    {
        TendermintRpcClient::broadcast_tx_sync(&self.client, tx).await
    }

    async fn broadcast_tx_commit<T>(
        &self,
        tx: T,
    ) -> Result<broadcast::tx_commit::Response, TendermintRpcError>
    where
        T: Into<Vec<u8>> + Send,
    {
        TendermintRpcClient::broadcast_tx_commit(&self.client, tx).await
    }

    async fn commit<H>(&self, height: H) -> Result<commit::Response, TendermintRpcError>
    where
        H: Into<Height> + Send,
    {
        self.client.commit(height).await
    }

    async fn consensus_params<H>(
        &self,
        height: H,
    ) -> Result<consensus_params::Response, TendermintRpcError>
    where
        H: Into<Height> + Send,
    {
        self.client.consensus_params(height).await
    }

    async fn consensus_state(&self) -> Result<consensus_state::Response, TendermintRpcError> {
        self.client.consensus_state().await
    }

    async fn validators<H>(
        &self,
        height: H,
        paging: Paging,
    ) -> Result<validators::Response, TendermintRpcError>
    where
        H: Into<Height> + Send,
    {
        TendermintRpcClient::validators(&self.client, height, paging).await
    }

    async fn latest_consensus_params(
        &self,
    ) -> Result<consensus_params::Response, TendermintRpcError> {
        self.client.latest_consensus_params().await
    }

    async fn latest_commit(&self) -> Result<commit::Response, TendermintRpcError> {
        self.client.latest_commit().await
    }

    async fn health(&self) -> Result<(), TendermintRpcError> {
        self.client.health().await
    }

    async fn genesis<AppState>(&self) -> Result<Genesis<AppState>, TendermintRpcError>
    where
        AppState: Debug + Serialize + DeserializeOwned + Send,
    {
        self.client.genesis().await
    }

    async fn net_info(&self) -> Result<net_info::Response, TendermintRpcError> {
        self.client.net_info().await
    }

    async fn status(&self) -> Result<status::Response, TendermintRpcError> {
        self.client.status().await
    }

    async fn broadcast_evidence(
        &self,
        e: Evidence,
    ) -> Result<evidence::Response, TendermintRpcError> {
        self.client.broadcast_evidence(e).await
    }

    async fn tx(&self, hash: Hash, prove: bool) -> Result<TxResponse, TendermintRpcError> {
        self.client.tx(hash, prove).await
    }

    async fn tx_search(
        &self,
        query: Query,
        prove: bool,
        page: u32,
        per_page: u8,
        order: Order,
    ) -> Result<tx_search::Response, TendermintRpcError> {
        self.client
            .tx_search(query, prove, page, per_page, order)
            .await
    }

    #[cfg(any(
        feature = "tendermint-rpc/http-client",
        feature = "tendermint-rpc/websocket-client"
    ))]
    async fn wait_until_healthy<T>(&self, timeout: T) -> Result<(), Error>
    where
        T: Into<core::time::Duration> + Send,
    {
        self.client.wait_until_healthy(timeout).await
    }

    async fn perform<R>(&self, request: R) -> Result<R::Output, TendermintRpcError>
    where
        R: SimpleRequest,
    {
        self.client.perform(request).await
    }
}

impl<C, S> OfflineSigner for NyxdClient<C, S>
where
    S: OfflineSigner,
{
    type Error = S::Error;

    fn get_accounts(&self) -> Result<Vec<AccountData>, Self::Error> {
        self.client.get_accounts()
    }

    fn sign_direct_with_account(
        &self,
        signer: &AccountData,
        sign_doc: SignDoc,
    ) -> Result<Raw, Self::Error> {
        self.client.sign_direct_with_account(signer, sign_doc)
    }
}

#[async_trait]
impl<C, S> SigningCosmWasmClient for NyxdClient<C, S>
where
    C: TendermintRpcClient + Send + Sync,
    S: TxSigner + Send + Sync,
    NyxdError: From<S::Error>,
{
    fn gas_price(&self) -> &GasPrice {
        self.client.gas_price()
    }

    fn simulated_gas_multiplier(&self) -> f32 {
        self.client.simulated_gas_multiplier()
    }
}
