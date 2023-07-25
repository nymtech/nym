// Copyright 2021 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use crate::nyxd::cosmwasm_client::types::Account;
use crate::nyxd::error::NyxdError;
use log::{debug, trace};
use nym_network_defaults::{ChainDetails, NymNetworkDetails};
use serde::{Deserialize, Serialize};
use tendermint_rpc::{endpoint::block::Response as BlockResponse, query::Query};

#[cfg(feature = "http-client")]
use tendermint_rpc::Error as TendermintRpcError;

pub use crate::nyxd::cosmwasm_client::client::CosmWasmClient;
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

#[cfg(feature = "http-client")]
pub use cosmrs::rpc::{HttpClient as QueryNyxdClient, HttpClientUrl};

#[cfg(all(feature = "signing", feature = "http-client"))]
use crate::nyxd::cosmwasm_client::signing_client;
#[cfg(feature = "signing")]
use crate::nyxd::cosmwasm_client::types::{
    ChangeAdminResult, ContractCodeId, ExecuteResult, InstantiateOptions, InstantiateResult,
    MigrateResult, SequenceResponse, SimulateResponse, UploadResult,
};
#[cfg(all(feature = "signing", feature = "http-client"))]
use crate::signing::direct_wallet::DirectSecp256k1HdWallet;
#[cfg(all(feature = "signing", feature = "http-client"))]
use crate::signing::signer::OfflineSigner;
#[cfg(feature = "signing")]
use cosmrs::cosmwasm;
#[cfg(feature = "signing")]
use cosmrs::tx::Msg;
#[cfg(feature = "signing")]
use cosmwasm_std::Addr;
#[cfg(feature = "signing")]
use std::time::SystemTime;

#[cfg(feature = "signing")]
pub use crate::nyxd::cosmwasm_client::signing_client::SigningCosmWasmClient;

#[cfg(all(feature = "signing", feature = "http-client"))]
pub use signing_client::Client as SigningNyxdClient;

#[cfg(all(feature = "signing", feature = "http-client"))]
pub type DirectSigningNyxdClient = SigningNyxdClient<DirectSecp256k1HdWallet>;

pub mod coin;
pub mod cosmwasm_client;
pub mod error;
pub mod fee;
pub mod traits;

#[derive(Debug, Clone)]
pub struct Config {
    pub(crate) chain_details: ChainDetails,

    // I'd love to have used `NymContracts` struct directly here instead,
    // however, I'd really prefer to use something more strongly typed (i.e. AccountId vs String)
    pub(crate) mixnet_contract_address: Option<AccountId>,
    pub(crate) vesting_contract_address: Option<AccountId>,
    pub(crate) bandwidth_claim_contract_address: Option<AccountId>,
    pub(crate) coconut_bandwidth_contract_address: Option<AccountId>,
    pub(crate) group_contract_address: Option<AccountId>,
    pub(crate) multisig_contract_address: Option<AccountId>,
    pub(crate) coconut_dkg_contract_address: Option<AccountId>,
    pub(crate) service_provider_contract_address: Option<AccountId>,
    pub(crate) name_service_contract_address: Option<AccountId>,
    // TODO: add this in later commits
    // pub(crate) gas_price: GasPrice,
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
            group_contract_address: Self::parse_optional_account(
                details.contracts.group_contract_address.as_ref(),
                prefix,
            )?,
            multisig_contract_address: Self::parse_optional_account(
                details.contracts.multisig_contract_address.as_ref(),
                prefix,
            )?,
            coconut_dkg_contract_address: Self::parse_optional_account(
                details.contracts.coconut_dkg_contract_address.as_ref(),
                prefix,
            )?,
            service_provider_contract_address: Self::parse_optional_account(
                details
                    .contracts
                    .service_provider_directory_contract_address
                    .as_ref(),
                prefix,
            )?,
            name_service_contract_address: Self::parse_optional_account(
                details.contracts.name_service_contract_address.as_ref(),
                prefix,
            )?,
        })
    }
}

#[derive(Debug)]
pub struct NyxdClient<C> {
    client: C,
    config: Config,
    // TODO: refactor because that field is only really used for signing
    #[allow(dead_code)]
    client_address: Option<Vec<AccountId>>,
    simulated_gas_multiplier: f32,
}

#[cfg(feature = "http-client")]
impl NyxdClient<QueryNyxdClient> {
    pub fn connect<U>(config: Config, endpoint: U) -> Result<NyxdClient<QueryNyxdClient>, NyxdError>
    where
        U: TryInto<HttpClientUrl, Error = TendermintRpcError>,
    {
        Ok(NyxdClient {
            client: QueryNyxdClient::new(endpoint)?,
            config,
            client_address: None,
            simulated_gas_multiplier: crate::nyxd::fee::DEFAULT_SIMULATED_GAS_MULTIPLIER,
        })
    }
}

#[cfg(all(feature = "signing", feature = "http-client"))]
impl NyxdClient<SigningNyxdClient<DirectSecp256k1HdWallet>> {
    // TODO: rename this one
    pub fn connect_with_mnemonic<U: Clone>(
        config: Config,
        endpoint: U,
        mnemonic: bip39::Mnemonic,
        gas_price: Option<GasPrice>,
    ) -> Result<NyxdClient<SigningNyxdClient<DirectSecp256k1HdWallet>>, NyxdError>
    where
        U: TryInto<HttpClientUrl, Error = TendermintRpcError>,
    {
        let prefix = &config.chain_details.bech32_account_prefix;
        let wallet = DirectSecp256k1HdWallet::from_mnemonic(prefix, mnemonic);
        Self::connect_with_signer(config, endpoint, wallet, gas_price)
    }
}

#[cfg(all(feature = "signing", feature = "http-client"))]
impl<S> NyxdClient<SigningNyxdClient<S>>
where
    S: OfflineSigner,
    // I have no idea why S::Error: Into<NyxdError> bound wouldn't do the trick
    NyxdError: From<S::Error>,
{
    #[cfg(feature = "http-client")]
    pub fn connect_with_signer<U: Clone>(
        config: Config,
        endpoint: U,
        signer: S,
        gas_price: Option<GasPrice>,
    ) -> Result<NyxdClient<SigningNyxdClient<S>>, NyxdError>
    where
        U: TryInto<HttpClientUrl, Error = TendermintRpcError>,
    {
        let denom = &config.chain_details.mix_denom.base;
        let client_address = signer
            .get_accounts()?
            .into_iter()
            .map(|account| account.address)
            .collect();
        let gas_price = gas_price.unwrap_or(GasPrice::new_with_default_price(denom)?);

        Ok(NyxdClient {
            client: SigningNyxdClient::connect_with_signer(endpoint, signer, gas_price)?,
            config,
            client_address: Some(client_address),
            simulated_gas_multiplier: crate::nyxd::fee::DEFAULT_SIMULATED_GAS_MULTIPLIER,
        })
    }

    #[cfg(feature = "http-client")]
    pub fn change_endpoint<U>(&mut self, new_endpoint: U) -> Result<(), NyxdError>
    where
        U: TryInto<HttpClientUrl, Error = TendermintRpcError>,
    {
        self.client.change_endpoint(new_endpoint)
    }

    pub fn into_signer(self) -> S {
        self.client.into_signer()
    }
}

#[cfg(feature = "signing")]
impl<C> NyxdClient<C>
where
    C: SigningCosmWasmClient + Sync,
{
    pub fn address(&self) -> &AccountId
    where
        C: SigningCosmWasmClient,
    {
        // if this is a signing client (as required by the trait bound), it must have the address set
        &self.client_address.as_ref().unwrap()[0]
    }

    pub fn cw_address(&self) -> Addr
    where
        C: SigningCosmWasmClient,
    {
        // the call to unchecked is fine here as we're converting directly from `AccountId`
        // which must have been a valid bech32 address
        Addr::unchecked(self.address().as_ref())
    }

    pub async fn account_sequence(&self) -> Result<SequenceResponse, NyxdError>
    where
        C: SigningCosmWasmClient + Sync,
    {
        self.client.get_sequence(self.address()).await
    }

    pub fn signer(&self) -> &<C as SigningCosmWasmClient>::Signer
    where
        C: SigningCosmWasmClient,
    {
        self.client.signer()
    }

    pub fn gas_price(&self) -> &GasPrice
    where
        C: SigningCosmWasmClient,
    {
        self.client.gas_price()
    }

    pub fn wrap_contract_execute_message<M>(
        &self,
        contract_address: &AccountId,
        msg: &M,
        funds: Vec<Coin>,
    ) -> Result<cosmwasm::MsgExecuteContract, NyxdError>
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

    pub async fn simulate<I, M>(&self, messages: I) -> Result<SimulateResponse, NyxdError>
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
    ) -> Result<TxResponse, NyxdError>
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
    ) -> Result<TxResponse, NyxdError>
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
    ) -> Result<TxResponse, NyxdError>
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
    ) -> Result<TxResponse, NyxdError>
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
        fee: Option<Fee>,
        memo: impl Into<String> + Send + 'static,
        funds: Vec<Coin>,
    ) -> Result<ExecuteResult, NyxdError>
    where
        C: SigningCosmWasmClient + Sync,
        M: ?Sized + Serialize + Sync,
    {
        let fee = fee.unwrap_or(Fee::Auto(Some(self.simulated_gas_multiplier)));
        self.client
            .execute(self.address(), contract_address, msg, fee, memo, funds)
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
        C: SigningCosmWasmClient + Sync,
        I: IntoIterator<Item = (M, Vec<Coin>)> + Send,
        M: Serialize,
    {
        let fee = fee.unwrap_or(Fee::Auto(Some(self.simulated_gas_multiplier)));
        self.client
            .execute_multiple(self.address(), contract_address, msgs, fee, memo)
            .await
    }

    pub async fn upload(
        &self,
        wasm_code: Vec<u8>,
        memo: impl Into<String> + Send + 'static,
        fee: Option<Fee>,
    ) -> Result<UploadResult, NyxdError>
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
    ) -> Result<InstantiateResult, NyxdError>
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
    ) -> Result<ChangeAdminResult, NyxdError>
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
    ) -> Result<ChangeAdminResult, NyxdError>
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
    ) -> Result<MigrateResult, NyxdError>
    where
        C: SigningCosmWasmClient + Sync,
        M: ?Sized + Serialize + Sync,
    {
        let fee = fee.unwrap_or(Fee::Auto(Some(self.simulated_gas_multiplier)));
        self.client
            .migrate(self.address(), contract_address, code_id, fee, msg, memo)
            .await
    }
}

impl<C> NyxdClient<C> {
    pub fn current_config(&self) -> &Config {
        &self.config
    }

    pub fn current_chain_details(&self) -> &ChainDetails {
        &self.config.chain_details
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

    pub fn set_service_provider_contract_address(&mut self, address: AccountId) {
        self.config.service_provider_contract_address = Some(address);
    }

    // TODO: this should get changed into Result<&AccountId, NyxdError> (or Option<&AccountId> in future commits
    // note: what unwrap is doing here is just moving a failure that would have normally
    // occurred in `connect` when attempting to parse an empty address,
    // so it's not introducing new source of failure (just moves it)
    pub fn mixnet_contract_address(&self) -> &AccountId {
        self.config.mixnet_contract_address.as_ref().unwrap()
    }

    // TODO: this should get changed into Result<&AccountId, NyxdError> (or Option<&AccountId> in future commits
    // note: what unwrap is doing here is just moving a failure that would have normally
    // occurred in `connect` when attempting to parse an empty address,
    // so it's not introducing new source of failure (just moves it)
    pub fn vesting_contract_address(&self) -> &AccountId {
        self.config.vesting_contract_address.as_ref().unwrap()
    }

    // TODO: this should get changed into Result<&AccountId, NyxdError> (or Option<&AccountId> in future commits
    // note: what unwrap is doing here is just moving a failure that would have normally
    // occurred in `connect` when attempting to parse an empty address,
    // so it's not introducing new source of failure (just moves it)
    pub fn bandwidth_claim_contract_address(&self) -> &AccountId {
        self.config
            .bandwidth_claim_contract_address
            .as_ref()
            .unwrap()
    }

    // TODO: this should get changed into Result<&AccountId, NyxdError> (or Option<&AccountId> in future commits
    // note: what unwrap is doing here is just moving a failure that would have normally
    // occurred in `connect` when attempting to parse an empty address,
    // so it's not introducing new source of failure (just moves it)
    pub fn coconut_bandwidth_contract_address(&self) -> &AccountId {
        self.config
            .coconut_bandwidth_contract_address
            .as_ref()
            .unwrap()
    }

    pub fn group_contract_address(&self) -> &AccountId {
        self.config.group_contract_address.as_ref().unwrap()
    }

    // TODO: this should get changed into Result<&AccountId, NyxdError> (or Option<&AccountId> in future commits
    // note: what unwrap is doing here is just moving a failure that would have normally
    // occurred in `connect` when attempting to parse an empty address,
    // so it's not introducing new source of failure (just moves it)
    pub fn multisig_contract_address(&self) -> &AccountId {
        self.config.multisig_contract_address.as_ref().unwrap()
    }

    // TODO: this should get changed into Result<&AccountId, NyxdError> (or Option<&AccountId> in future commits
    // note: what unwrap is doing here is just moving a failure that would have normally
    // occurred in `connect` when attempting to parse an empty address,
    // so it's not introducing new source of failure (just moves it)
    pub fn coconut_dkg_contract_address(&self) -> &AccountId {
        self.config.coconut_dkg_contract_address.as_ref().unwrap()
    }

    // The service provider directory contract is optional, so we return an Option not a Result
    pub fn service_provider_contract_address(&self) -> Option<&AccountId> {
        self.config.service_provider_contract_address.as_ref()
    }

    // The name service contract is optional, so we return an Option not a Result
    pub fn name_service_contract_address(&self) -> Option<&AccountId> {
        self.config.name_service_contract_address.as_ref()
    }

    pub fn set_simulated_gas_multiplier(&mut self, multiplier: f32) {
        self.simulated_gas_multiplier = multiplier;
    }

    pub async fn query_contract_smart<M, T>(
        &self,
        contract: &AccountId,
        query_msg: &M,
    ) -> Result<T, NyxdError>
    where
        C: CosmWasmClient + Sync,
        M: ?Sized + Serialize + Sync,
        for<'a> T: Deserialize<'a>,
    {
        self.client.query_contract_smart(contract, query_msg).await
    }

    pub async fn query_contract_raw(
        &self,
        contract: &AccountId,
        query_data: Vec<u8>,
    ) -> Result<Vec<u8>, NyxdError>
    where
        C: CosmWasmClient + Sync,
    {
        self.client.query_contract_raw(contract, query_data).await
    }

    pub fn gas_adjustment(&self) -> GasAdjustment {
        self.simulated_gas_multiplier
    }

    // =============
    // CHAIN RELATED
    // =============

    // CHAIN QUERIES

    pub async fn get_account_details(
        &self,
        address: &AccountId,
    ) -> Result<Option<Account>, NyxdError>
    where
        C: CosmWasmClient + Sync,
    {
        self.client.get_account(address).await
    }

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

    pub async fn get_validators(
        &self,
        height: u64,
        paging: Paging,
    ) -> Result<ValidatorResponse, NyxdError>
    where
        C: CosmWasmClient + Sync,
    {
        Ok(self.client.validators(height as u32, paging).await?)
    }

    pub async fn get_balance(
        &self,
        address: &AccountId,
        denom: String,
    ) -> Result<Option<Coin>, NyxdError>
    where
        C: CosmWasmClient + Sync,
    {
        self.client.get_balance(address, denom).await
    }

    pub async fn get_all_balances(&self, address: &AccountId) -> Result<Vec<Coin>, NyxdError>
    where
        C: CosmWasmClient + Sync,
    {
        self.client.get_all_balances(address).await
    }

    pub async fn get_tx(&self, id: Hash) -> Result<TxResponse, NyxdError>
    where
        C: CosmWasmClient + Sync,
    {
        self.client.get_tx(id).await
    }

    pub async fn search_tx(&self, query: Query) -> Result<Vec<TxResponse>, NyxdError>
    where
        C: CosmWasmClient + Sync,
    {
        self.client.search_tx(query).await
    }

    pub async fn get_total_supply(&self) -> Result<Vec<Coin>, NyxdError>
    where
        C: CosmWasmClient + Sync,
    {
        self.client.get_total_supply().await
    }
}
