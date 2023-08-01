// Copyright 2021 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use crate::nyxd::error::NyxdError;
use async_trait::async_trait;
use log::{debug, trace};
use nym_network_defaults::{ChainDetails, NymNetworkDetails};
use serde::Serialize;
use tendermint_rpc::endpoint::block::Response as BlockResponse;
use tendermint_rpc::Error as TendermintRpcError;

pub use crate::nyxd::cosmwasm_client::client_traits::{CosmWasmClient, SigningCosmWasmClient};
pub use crate::nyxd::fee::Fee;
pub use coin::Coin;
pub use cosmrs::bank::MsgSend;
pub use cosmrs::tendermint::abci::{response::DeliverTx, Event, EventAttribute};
pub use cosmrs::tendermint::block::Height;
pub use cosmrs::tendermint::hash::{self, Algorithm, Hash};
pub use cosmrs::tendermint::validator::Info as TendermintValidatorInfo;
pub use cosmrs::tendermint::Time as TendermintTime;
pub use cosmrs::tx::{self};
pub use cosmrs::Coin as CosmosCoin;
pub use cosmrs::Gas;
pub use cosmrs::{bip32, AccountId, Denom};
pub use cosmwasm_std::Coin as CosmWasmCoin;
pub use fee::{gas_price::GasPrice, GasAdjustable, GasAdjustment};
pub use tendermint_rpc::{client::Client as TendermintClient, Request, Response, SimpleRequest};
pub use tendermint_rpc::{
    endpoint::{tx::Response as TxResponse, validators::Response as ValidatorResponse},
    Paging,
};

use crate::nyxd::cosmwasm_client::types::{
    ChangeAdminResult, ContractCodeId, ExecuteResult, InstantiateOptions, InstantiateResult,
    MigrateResult, SequenceResponse, SimulateResponse, UploadResult,
};
use crate::signing::signer::OfflineSigner;
use cosmrs::cosmwasm;
use cosmrs::tx::Msg;
use cosmwasm_std::Addr;
use std::time::SystemTime;

#[cfg(feature = "http-client")]
use cosmrs::rpc::{HttpClient, HttpClientUrl};

use crate::nyxd::contract_traits::{NymContractsProvider, TypedNymContracts};
use crate::nyxd::cosmwasm_client::MaybeSigningClient;
use crate::nyxd::fee::DEFAULT_SIMULATED_GAS_MULTIPLIER;
use crate::signing::signer::NoSigner;

pub mod coin;
pub mod contract_traits;
pub mod cosmwasm_client;
pub mod error;
pub mod fee;

// helper types
#[cfg(feature = "http-client")]
pub type DirectSigningHttpNyxdClient =
    NyxdClient<HttpClient, crate::signing::direct_wallet::DirectSecp256k1HdWallet>;

#[cfg(feature = "http-client")]
pub type QueryHttpNyxdClient = NyxdClient<HttpClient>;

// TODO: reqwest based for wasm

#[derive(Debug, Clone)]
pub struct Config {
    pub(crate) chain_details: ChainDetails,
    pub(crate) contracts: TypedNymContracts,
    pub(crate) gas_price: GasPrice,
    pub(crate) simulated_gas_multiplier: f32,
}

impl Config {
    fn parse_optional_account(
        raw: Option<&String>,
        expected_prefix: &str,
    ) -> Result<Option<AccountId>, NyxdError> {
        if let Some(address) = raw {
            trace!("Raw address:{:?}", raw);
            trace!("Expected prefix:{:?}", expected_prefix);
            let parsed: AccountId = address
                .parse()
                .map_err(|_| NyxdError::MalformedAccountAddress(address.clone()))?;
            debug!("Parsed prefix:{:?}", parsed);
            if parsed.prefix() != expected_prefix {
                Err(NyxdError::UnexpectedBech32Prefix {
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

// so youd have:
// for queries:
// NyxdClient<HttpClient>
// NyxdClient<WasmReqwest>
//
// for signing:
// NyxdClient<HttpClient, Direct....>
// NyxdClient<WasmReqest, CosmJsWallet>

#[derive(Debug)]
pub struct NyxdClient<C, S = NoSigner> {
    client: MaybeSigningClient<C, S>,
    config: Config,
}

#[cfg(feature = "http-client")]
impl NyxdClient<HttpClient> {
    pub fn connect<U>(config: Config, endpoint: U) -> Result<QueryHttpNyxdClient, NyxdError>
    where
        U: TryInto<HttpClientUrl, Error = TendermintRpcError>,
    {
        let client = HttpClient::new(endpoint)?;

        Ok(NyxdClient {
            client: MaybeSigningClient::new(client, (&config).into()),
            config,
        })
    }
}

#[cfg(all(feature = "direct-secp256k1-wallet", feature = "http-client"))]
impl NyxdClient<HttpClient, DirectSecp256k1HdWallet> {
    // TODO: rename this one
    pub fn connect_with_mnemonic<U: Clone>(
        config: Config,
        endpoint: U,
        mnemonic: bip39::Mnemonic,
    ) -> Result<DirectSigningHttpNyxdClient, NyxdError>
    where
        U: TryInto<HttpClientUrl, Error = TendermintRpcError>,
    {
        let prefix = &config.chain_details.bech32_account_prefix;
        let wallet = DirectSecp256k1HdWallet::from_mnemonic(prefix, mnemonic);
        Self::connect_with_signer(config, endpoint, wallet)
    }
}

#[cfg(feature = "http-client")]
impl<S> NyxdClient<HttpClient, S>
where
    S: OfflineSigner,
    // I have no idea why S::Error: Into<NyxdError> bound wouldn't do the trick
    NyxdError: From<S::Error>,
{
    pub fn connect_with_signer<U: Clone>(
        config: Config,
        endpoint: U,
        signer: S,
    ) -> Result<NyxdClient<HttpClient, S>, NyxdError>
    where
        U: TryInto<HttpClientUrl, Error = TendermintRpcError>,
    {
        let client = HttpClient::new(endpoint)?;
        Ok(NyxdClient {
            client: MaybeSigningClient::new_signing(client, signer, (&config).into()),
            config,
        })
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

    pub fn set_coconut_bandwidth_contract_address(&mut self, address: AccountId) {
        self.config.contracts.coconut_bandwidth_contract_address = Some(address);
    }

    pub fn set_multisig_contract_address(&mut self, address: AccountId) {
        self.config.contracts.multisig_contract_address = Some(address);
    }

    pub fn set_service_provider_contract_address(&mut self, address: AccountId) {
        self.config
            .contracts
            .service_provider_directory_contract_address = Some(address);
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

    fn coconut_bandwidth_contract_address(&self) -> Option<&AccountId> {
        self.config
            .contracts
            .coconut_bandwidth_contract_address
            .as_ref()
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

    fn name_service_contract_address(&self) -> Option<&AccountId> {
        self.config.contracts.name_service_contract_address.as_ref()
    }

    fn service_provider_contract_address(&self) -> Option<&AccountId> {
        self.config
            .contracts
            .service_provider_directory_contract_address
            .as_ref()
    }
}

// queries
impl<C, S> NyxdClient<C, S>
where
    C: CosmWasmClient + Send + Sync,
    S: Send + Sync,
{
    pub async fn get_account_public_key(
        &self,
        address: &AccountId,
    ) -> Result<Option<cosmrs::crypto::PublicKey>, NyxdError>
    where
        C: CosmWasmClient + Sync,
    {
        if let Some(account) = self.client.get_account(address).await? {
            let base_account = account.try_get_base_account()?;
            return Ok(base_account.pubkey);
        }

        Ok(None)
    }

    pub async fn get_current_block_timestamp(&self) -> Result<TendermintTime, NyxdError>
    where
        C: CosmWasmClient + Sync,
    {
        self.get_block_timestamp(None).await
    }

    pub async fn get_block_timestamp(
        &self,
        height: Option<u32>,
    ) -> Result<TendermintTime, NyxdError>
    where
        C: CosmWasmClient + Sync,
    {
        Ok(self.client.get_block(height).await?.block.header.time)
    }

    pub async fn get_block(&self, height: Option<u32>) -> Result<BlockResponse, NyxdError>
    where
        C: CosmWasmClient + Sync,
    {
        self.client.get_block(height).await
    }

    pub async fn get_current_block_height(&self) -> Result<Height, NyxdError>
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
    pub async fn get_block_hash(&self, height: u32) -> Result<Hash, NyxdError>
    where
        C: CosmWasmClient + Sync,
    {
        self.client
            .get_block(Some(height))
            .await
            .map(|block| block.block_id.hash)
    }
}

// signing
impl<C, S> NyxdClient<C, S>
where
    C: SigningCosmWasmClient + Send + Sync,
    S: OfflineSigner + Send + Sync,

    // TODO: figure out those trait bounds lol
    NyxdError: From<<S as OfflineSigner>::Error>,
    NyxdError: From<<C as OfflineSigner>::Error>,
{
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

    pub async fn simulate<I, M>(&self, messages: I) -> Result<SimulateResponse, NyxdError>
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

#[async_trait]
impl<C> TendermintClient for NyxdClient<C>
where
    C: TendermintClient + Send + Sync,
{
    async fn perform<R>(&self, request: R) -> Result<R::Output, TendermintRpcError>
    where
        R: SimpleRequest,
    {
        self.client.perform(request).await
    }
}

#[async_trait]
impl<C> CosmWasmClient for NyxdClient<C> where C: CosmWasmClient + Send + Sync {}

// #[async_trait]
// impl<C> SigningCosmWasmClient for NyxdClient<C> where C: SigningCosmWasmClient + Send + Sync {
//     fn gas_price(&self) -> &GasPrice {
//         self.client.gas_price()
//     }
// }
