// Copyright 2021 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use crate::nyxd::cosmwasm_client::client_traits::SigningCosmWasmClient;
use crate::nyxd::error::NyxdError;
use crate::nyxd::{Config, GasPrice, Hash, Height};
use crate::rpc::TendermintRpcClient;
use crate::signing::{
    signer::{NoSigner, OfflineSigner},
    AccountData,
};
use async_trait::async_trait;
use cosmrs::tendermint::{abci, evidence::Evidence, Genesis};
use cosmrs::tx::{Raw, SignDoc};
use serde::{de::DeserializeOwned, Serialize};
use std::fmt::Debug;
use tendermint_rpc::endpoint::*;
use tendermint_rpc::query::Query;
use tendermint_rpc::{Error as TendermintRpcError, Order, Paging, SimpleRequest};

pub use helpers::ToContractResponseData;

#[cfg(feature = "http-client")]
use crate::http_client;
#[cfg(feature = "http-client")]
use cosmrs::rpc::{HttpClient, HttpClientUrl};

pub mod client_traits;
mod helpers;
pub mod logs;
pub mod module_traits;
pub mod types;

#[derive(Debug)]
pub(crate) struct SigningClientOptions {
    gas_price: GasPrice,
    simulated_gas_multiplier: f32,
}

impl<'a> From<&'a Config> for SigningClientOptions {
    fn from(value: &'a Config) -> Self {
        SigningClientOptions {
            gas_price: value.gas_price.clone(),
            simulated_gas_multiplier: value.simulated_gas_multiplier,
        }
    }
}

// convenience wrapper around query client to allow for optional signing
#[derive(Debug)]
pub(crate) struct MaybeSigningClient<C, S = NoSigner> {
    client: C,
    signer: S,
    opts: SigningClientOptions,
}

impl<C> MaybeSigningClient<C> {
    pub(crate) fn new(client: C, opts: SigningClientOptions) -> Self {
        MaybeSigningClient {
            client,
            signer: Default::default(),
            opts,
        }
    }
}

impl<C, S> MaybeSigningClient<C, S> {
    pub(crate) fn new_signing(client: C, signer: S, opts: SigningClientOptions) -> Self
    where
        S: OfflineSigner,
    {
        MaybeSigningClient {
            client,
            signer,
            opts,
        }
    }
}

#[cfg(feature = "http-client")]
impl<S> MaybeSigningClient<HttpClient, S> {
    pub(crate) fn change_endpoint<U>(&mut self, new_endpoint: U) -> Result<(), NyxdError>
    where
        U: TryInto<HttpClientUrl, Error = TendermintRpcError>,
    {
        self.client = http_client(new_endpoint)?;
        Ok(())
    }
}

#[cfg_attr(target_arch = "wasm32", async_trait(?Send))]
#[cfg_attr(not(target_arch = "wasm32"), async_trait)]
impl<C, S> TendermintRpcClient for MaybeSigningClient<C, S>
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
        self.client.broadcast_tx_async(tx).await
    }

    async fn broadcast_tx_sync<T>(
        &self,
        tx: T,
    ) -> Result<broadcast::tx_sync::Response, TendermintRpcError>
    where
        T: Into<Vec<u8>> + Send,
    {
        self.client.broadcast_tx_sync(tx).await
    }

    async fn broadcast_tx_commit<T>(
        &self,
        tx: T,
    ) -> Result<broadcast::tx_commit::Response, TendermintRpcError>
    where
        T: Into<Vec<u8>> + Send,
    {
        self.client.broadcast_tx_commit(tx).await
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
        self.client.validators(height, paging).await
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

    async fn tx(&self, hash: Hash, prove: bool) -> Result<tx::Response, TendermintRpcError> {
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

impl<C, S> OfflineSigner for MaybeSigningClient<C, S>
where
    S: OfflineSigner,
{
    type Error = S::Error;

    fn get_accounts(&self) -> Result<Vec<AccountData>, Self::Error> {
        self.signer.get_accounts()
    }

    fn sign_direct_with_account(
        &self,
        signer: &AccountData,
        sign_doc: SignDoc,
    ) -> Result<Raw, Self::Error> {
        self.signer.sign_direct_with_account(signer, sign_doc)
    }
}

#[async_trait]
impl<C, S> SigningCosmWasmClient for MaybeSigningClient<C, S>
where
    C: TendermintRpcClient + Send + Sync,
    S: OfflineSigner + Send + Sync,
    NyxdError: From<S::Error>,
{
    fn gas_price(&self) -> &GasPrice {
        &self.opts.gas_price
    }

    fn simulated_gas_multiplier(&self) -> f32 {
        self.opts.simulated_gas_multiplier
    }
}

//
// #[cfg(feature = "http-client")]
// pub fn connect<U>(endpoint: U) -> Result<MaybeSigningClient<HttpClient>, NyxdError>
// where
//     U: TryInto<HttpClientUrl, Error = TendermintRpcError>,
// {
//     Ok(HttpClient::new(endpoint)?)
// }
//
// #[cfg(all(feature = "signing", feature = "http-client"))]
// pub fn connect_with_signer<S, U: Clone>(
//     endpoint: U,
//     signer: S,
//     gas_price: crate::nyxd::GasPrice,
// ) -> Result<MaybeSigningClient<HttpClient, S>, NyxdError>
// where
//     U: TryInto<HttpClientUrl, Error = TendermintRpcError>,
// {
//     signing_client::Client::connect_with_signer(endpoint, signer, gas_price)
// }
