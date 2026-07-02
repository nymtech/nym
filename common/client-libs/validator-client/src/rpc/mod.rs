// Copyright 2023 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

// TEMPORARY WORKAROUND:
// those features are expected as the below should only get activated whenever
// the corresponding features in tendermint-rpc are enabled transitively
#![allow(unexpected_cfgs)]

use crate::nyxd;
use crate::nyxd::cosmwasm_client::client_traits::query_client::{
    DEFAULT_BROADCAST_POLLING_RATE, DEFAULT_BROADCAST_TIMEOUT,
};
use crate::nyxd::cosmwasm_client::types::{Account, SequenceResponse, SimulateResponse};
use crate::nyxd::error::NyxdError;
use crate::nyxd::helpers::{create_pagination, next_page_key};
use crate::nyxd::{BlockResponse, Coin, TxResponse};
use async_trait::async_trait;
use cosmrs::proto::cosmos::auth::v1beta1::{QueryAccountRequest, QueryAccountResponse};
use cosmrs::proto::cosmos::bank::v1beta1::{
    QueryAllBalancesRequest, QueryAllBalancesResponse, QueryBalanceRequest, QueryBalanceResponse,
    QueryTotalSupplyRequest, QueryTotalSupplyResponse,
};
use cosmrs::proto::cosmos::tx::v1beta1::{
    SimulateRequest, SimulateResponse as ProtoSimulateResponse,
};
use cosmrs::tendermint::{self, abci, block::Height, chain, evidence::Evidence, Genesis, Hash};
use cosmrs::{AccountId, Coin as CosmosCoin};
use prost::Message;
use serde::{de::DeserializeOwned, Serialize};
use std::fmt;
use std::time::Duration;
use tendermint_rpc::{
    endpoint::{validators::DEFAULT_VALIDATORS_PER_PAGE, *},
    query::Query,
    Error, Order, Paging, SimpleRequest,
};

#[cfg(feature = "http-client")]
use crate::error::TendermintRpcError;
#[cfg(feature = "http-client")]
use crate::HttpRpcClient;
#[cfg(feature = "http-client")]
use tendermint_rpc::client::CompatMode;
#[cfg(feature = "http-client")]
use tendermint_rpc::HttpClientUrl;

#[cfg(not(target_arch = "wasm32"))]
use tokio::time::sleep;
#[cfg(not(target_arch = "wasm32"))]
use tokio::time::Instant;

use crate::rpc::types::ProvableAbciQueryResponse;
#[cfg(target_arch = "wasm32")]
use wasmtimer::std::Instant;
#[cfg(target_arch = "wasm32")]
use wasmtimer::tokio::sleep;

pub mod reqwest;
pub mod types;

#[cfg(feature = "http-client")]
pub fn http_client<U>(url: U) -> Result<HttpRpcClient, TendermintRpcError>
where
    U: TryInto<HttpClientUrl, Error = Error>,
{
    HttpRpcClient::builder(url.try_into()?)
        .compat_mode(CompatMode::V0_37)
        .build()
}

#[cfg_attr(target_arch = "wasm32", async_trait(?Send))]
#[cfg_attr(not(target_arch = "wasm32"), async_trait)]
pub trait TendermintRpcClientExt: TendermintRpcClient {
    async fn make_abci_query_with_proof<Req, Res>(
        &self,
        path: Option<String>,
        req: Req,
        height: Option<Height>,
    ) -> Result<ProvableAbciQueryResponse<Res>, NyxdError>
    where
        Req: Message,
        Res: Message + Default,
    {
        if let Some(ref abci_path) = path {
            tracing::trace!("performing query on abci path {abci_path}")
        }
        let mut buf = Vec::with_capacity(req.encoded_len());
        req.encode(&mut buf)?;

        let res = self.abci_query(path, buf, height, true).await?;
        let res_success = nyxd::error::parse_abci_query_result(res)?;

        let Some(proof) = res_success.proof else {
            return Err(NyxdError::MissingProof);
        };

        let response = Res::decode(res_success.value.as_ref())?;

        Ok(ProvableAbciQueryResponse {
            response,
            height: res_success.height,
            proof,
        })
    }

    async fn make_abci_query_without_proof<Req, Res>(
        &self,
        path: Option<String>,
        req: Req,
        height: Option<Height>,
    ) -> Result<Res, NyxdError>
    where
        Req: Message,
        Res: Message + Default,
    {
        if let Some(ref abci_path) = path {
            tracing::trace!("performing query on abci path {abci_path}")
        }
        let mut buf = Vec::with_capacity(req.encoded_len());
        req.encode(&mut buf)?;

        let res = self.abci_query(path, buf, height, false).await?;
        let res_success = nyxd::error::parse_abci_query_result(res)?;

        Ok(Res::decode(res_success.value.as_ref())?)
    }

    async fn get_chain_id(&self) -> Result<chain::Id, NyxdError> {
        Ok(self.status().await?.node_info.network)
    }

    async fn get_height(&self) -> Result<cosmrs::tendermint::block::Height, NyxdError> {
        Ok(self.status().await?.sync_info.latest_block_height)
    }

    // TODO: the return type should probably be changed to a non-proto, type-safe Account alternative
    async fn get_account(&self, address: &AccountId) -> Result<Option<Account>, NyxdError> {
        let path = Some("/cosmos.auth.v1beta1.Query/Account".to_owned());

        let req = QueryAccountRequest {
            address: address.to_string(),
        };

        let res = self
            .make_abci_query_without_proof::<_, QueryAccountResponse>(path, req, None)
            .await?;

        res.account.map(TryFrom::try_from).transpose()
    }

    async fn get_sequence(&self, address: &AccountId) -> Result<SequenceResponse, NyxdError> {
        let account = self
            .get_account(address)
            .await?
            .ok_or_else(|| NyxdError::NonExistentAccountError(address.clone()))?;
        let base_account = account.try_get_base_account()?;

        Ok(SequenceResponse {
            account_number: base_account.account_number,
            sequence: base_account.sequence,
        })
    }

    async fn get_block(&self, height: Option<u32>) -> Result<BlockResponse, NyxdError> {
        match height {
            Some(height) => self.block(height).await.map_err(|err| err.into()),
            None => self.latest_block().await.map_err(|err| err.into()),
        }
    }

    async fn get_balance(
        &self,
        address: &AccountId,
        search_denom: String,
    ) -> Result<Option<Coin>, NyxdError> {
        let path = Some("/cosmos.bank.v1beta1.Query/Balance".to_owned());

        let req = QueryBalanceRequest {
            address: address.to_string(),
            denom: search_denom,
        };

        let res = self
            .make_abci_query_without_proof::<_, QueryBalanceResponse>(path, req, None)
            .await?;

        res.balance
            .map(|proto| CosmosCoin::try_from(proto).map(Into::into))
            .transpose()
            .map_err(|_| NyxdError::SerializationError("Coin".to_owned()))
    }

    async fn get_all_balances(&self, address: &AccountId) -> Result<Vec<Coin>, NyxdError> {
        let path = Some("/cosmos.bank.v1beta1.Query/AllBalances".to_owned());

        let mut raw_balances = Vec::new();
        let mut pagination = None;

        loop {
            let req = QueryAllBalancesRequest {
                address: address.to_string(),
                pagination,
                resolve_denom: false,
            };

            let mut res = self
                .make_abci_query_without_proof::<_, QueryAllBalancesResponse>(
                    path.clone(),
                    req,
                    None,
                )
                .await?;

            let early_break = res.balances.is_empty();
            raw_balances.append(&mut res.balances);

            if early_break {
                break;
            }

            if let Some(next_key) = next_page_key(res.pagination) {
                pagination = Some(create_pagination(next_key))
            } else {
                break;
            }
        }

        raw_balances
            .into_iter()
            .map(|proto| CosmosCoin::try_from(proto).map(Into::into))
            .collect::<Result<_, _>>()
            .map_err(|_| NyxdError::SerializationError("Coins".to_owned()))
    }

    async fn get_total_supply(&self) -> Result<Vec<Coin>, NyxdError> {
        let path = Some("/cosmos.bank.v1beta1.Query/TotalSupply".to_owned());

        let mut supply = Vec::new();
        let mut pagination = None;

        loop {
            let req = QueryTotalSupplyRequest { pagination };

            let mut res = self
                .make_abci_query_without_proof::<_, QueryTotalSupplyResponse>(
                    path.clone(),
                    req,
                    None,
                )
                .await?;

            let early_break = res.supply.is_empty();
            supply.append(&mut res.supply);

            if early_break {
                break;
            }

            if let Some(next_key) = next_page_key(res.pagination) {
                pagination = Some(create_pagination(next_key))
            } else {
                break;
            }
        }

        supply
            .into_iter()
            .map(|proto| CosmosCoin::try_from(proto).map(Into::into))
            .collect::<Result<_, _>>()
            .map_err(|_| NyxdError::SerializationError("Coins".to_owned()))
    }

    async fn get_tx(&self, id: Hash) -> Result<TxResponse, NyxdError> {
        Ok(self.tx(id, false).await?)
    }

    async fn search_tx(&self, query: Query) -> Result<Vec<TxResponse>, NyxdError> {
        // according to https://docs.tendermint.com/master/rpc/#/Info/tx_search
        // the maximum entries per page is 100 and the default is 30
        // so let's attempt to use the maximum
        let per_page = 100;

        let mut results = Vec::new();
        let mut page = 1;

        loop {
            let mut res = self
                .tx_search(query.clone(), false, page, per_page, Order::Ascending)
                .await?;

            // sanity check for if tendermint's maximum per_page was modified -
            // we don't want to accidentally be stuck in an infinite loop
            let early_break = res.total_count == 0 || res.txs.is_empty();
            results.append(&mut res.txs);

            if early_break {
                break;
            }

            if res.total_count > results.len() as u32 {
                page += 1
            } else {
                break;
            }
        }

        Ok(results)
    }

    /// Broadcast a transaction, returning immediately.
    async fn broadcast_tx_async<T>(&self, tx: T) -> Result<broadcast::tx_async::Response, NyxdError>
    where
        T: Into<Vec<u8>> + Send,
    {
        Ok(TendermintRpcClient::broadcast_tx_async(self, tx).await?)
    }

    /// Broadcast a transaction, returning the response from `CheckTx`.
    async fn broadcast_tx_sync<T>(&self, tx: T) -> Result<broadcast::tx_sync::Response, NyxdError>
    where
        T: Into<Vec<u8>> + Send,
    {
        Ok(TendermintRpcClient::broadcast_tx_sync(self, tx).await?)
    }

    /// Broadcast a transaction, returning the response from `DeliverTx`.
    async fn broadcast_tx_commit<T>(
        &self,
        tx: T,
    ) -> Result<broadcast::tx_commit::Response, NyxdError>
    where
        T: Into<Vec<u8>> + Send,
    {
        Ok(TendermintRpcClient::broadcast_tx_commit(self, tx).await?)
    }

    async fn broadcast_tx<T>(
        &self,
        tx: T,
        timeout: impl Into<Option<Duration>> + Send,
        poll_interval: impl Into<Option<Duration>> + Send,
    ) -> Result<TxResponse, NyxdError>
    where
        T: Into<Vec<u8>> + Send,
    {
        let timeout = timeout.into().unwrap_or(DEFAULT_BROADCAST_TIMEOUT);
        let poll_interval = poll_interval
            .into()
            .unwrap_or(DEFAULT_BROADCAST_POLLING_RATE);

        let broadcasted = TendermintRpcClientExt::broadcast_tx_sync(self, tx).await?;

        if broadcasted.code.is_err() {
            let code_val = broadcasted.code.value();
            return Err(NyxdError::BroadcastTxErrorDeliverTx {
                hash: broadcasted.hash,
                height: None,
                code: code_val,
                raw_log: broadcasted.log.to_string(),
            });
        }

        let tx_hash = broadcasted.hash;

        let start = Instant::now();
        loop {
            tracing::debug!(
                "Polling for result of including {} in a block...",
                broadcasted.hash
            );
            if Instant::now().duration_since(start) >= timeout {
                return Err(NyxdError::BroadcastTimeout {
                    hash: tx_hash,
                    timeout,
                });
            }

            if let Ok(poll_res) = self.get_tx(tx_hash).await {
                return Ok(poll_res);
            }

            sleep(poll_interval).await;
        }
    }

    async fn query_simulate(&self, tx_bytes: Vec<u8>) -> Result<SimulateResponse, NyxdError> {
        let path = Some("/cosmos.tx.v1beta1.Service/Simulate".to_owned());

        let req = SimulateRequest {
            tx_bytes,
            ..Default::default()
        };

        let res = self
            .make_abci_query_without_proof::<_, ProtoSimulateResponse>(path, req, None)
            .await?;

        res.try_into()
    }
}

impl<T> TendermintRpcClientExt for T where T: TendermintRpcClient {}

// we have to create a sealed trait since `TendermintClient` needs T: Send (due to how async trait is created)
// which we can't do in wasm
#[cfg_attr(target_arch = "wasm32", async_trait(?Send))]
#[cfg_attr(not(target_arch = "wasm32"), async_trait)]
pub trait TendermintRpcClient {
    /// `/abci_info`: get information about the ABCI application.
    async fn abci_info(&self) -> Result<abci::response::Info, Error> {
        Ok(self.perform(abci_info::Request).await?.response)
    }

    /// `/abci_query`: query the ABCI application
    async fn abci_query<V>(
        &self,
        path: Option<String>,
        data: V,
        height: Option<Height>,
        prove: bool,
    ) -> Result<abci_query::AbciQuery, Error>
    where
        V: Into<Vec<u8>> + Send,
    {
        Ok(self
            .perform(abci_query::Request::new(path, data, height, prove))
            .await?
            .response)
    }

    /// `/block`: get block at a given height.
    async fn block<H>(&self, height: H) -> Result<block::Response, Error>
    where
        H: Into<Height> + Send,
    {
        self.perform(block::Request::new(height.into())).await
    }

    /// `/block_by_hash`: get block by hash.
    async fn block_by_hash(
        &self,
        hash: tendermint::Hash,
    ) -> Result<block_by_hash::Response, Error> {
        self.perform(block_by_hash::Request::new(hash)).await
    }

    /// `/block`: get the latest block.
    async fn latest_block(&self) -> Result<block::Response, Error> {
        self.perform(block::Request::default()).await
    }

    /// `/header`: get block header at a given height.
    async fn header<H>(&self, height: H) -> Result<header::Response, Error>
    where
        H: Into<Height> + Send,
    {
        self.perform(header::Request::new(height.into())).await
    }

    /// `/header_by_hash`: get block by hash.
    async fn header_by_hash(
        &self,
        hash: tendermint::Hash,
    ) -> Result<header_by_hash::Response, Error> {
        self.perform(header_by_hash::Request::new(hash)).await
    }

    /// `/block_results`: get ABCI results for a block at a particular height.
    async fn block_results<H>(&self, height: H) -> Result<block_results::Response, Error>
    where
        H: Into<Height> + Send,
    {
        self.perform(block_results::Request::new(height.into()))
            .await
    }

    /// `/block_results`: get ABCI results for the latest block.
    async fn latest_block_results(&self) -> Result<block_results::Response, Error> {
        self.perform(block_results::Request::default()).await
    }

    /// `/block_search`: search for blocks by BeginBlock and EndBlock events.
    async fn block_search(
        &self,
        query: Query,
        page: u32,
        per_page: u8,
        order: Order,
    ) -> Result<block_search::Response, Error> {
        self.perform(block_search::Request::new(query, page, per_page, order))
            .await
    }

    /// `/blockchain`: get block headers for `min` <= `height` <= `max`.
    ///
    /// Block headers are returned in descending order (highest first).
    ///
    /// Returns at most 20 items.
    async fn blockchain<H>(&self, min: H, max: H) -> Result<blockchain::Response, Error>
    where
        H: Into<Height> + Send,
    {
        // TODO(tarcieri): return errors for invalid params before making request?
        self.perform(blockchain::Request::new(min.into(), max.into()))
            .await
    }

    /// `/broadcast_tx_async`: broadcast a transaction, returning immediately.
    async fn broadcast_tx_async<T>(&self, tx: T) -> Result<broadcast::tx_async::Response, Error>
    where
        T: Into<Vec<u8>> + Send,
    {
        self.perform(broadcast::tx_async::Request::new(tx)).await
    }

    /// `/broadcast_tx_sync`: broadcast a transaction, returning the response
    /// from `CheckTx`.
    async fn broadcast_tx_sync<T>(&self, tx: T) -> Result<broadcast::tx_sync::Response, Error>
    where
        T: Into<Vec<u8>> + Send,
    {
        self.perform(broadcast::tx_sync::Request::new(tx)).await
    }

    /// `/broadcast_tx_commit`: broadcast a transaction, returning the response
    /// from `DeliverTx`.
    async fn broadcast_tx_commit<T>(&self, tx: T) -> Result<broadcast::tx_commit::Response, Error>
    where
        T: Into<Vec<u8>> + Send,
    {
        self.perform(broadcast::tx_commit::Request::new(tx)).await
    }

    /// `/commit`: get block commit at a given height.
    async fn commit<H>(&self, height: H) -> Result<commit::Response, Error>
    where
        H: Into<Height> + Send,
    {
        self.perform(commit::Request::new(height.into())).await
    }

    /// `/consensus_params`: get current consensus parameters at the specified
    /// height.
    async fn consensus_params<H>(&self, height: H) -> Result<consensus_params::Response, Error>
    where
        H: Into<Height> + Send,
    {
        self.perform(consensus_params::Request::new(Some(height.into())))
            .await
    }

    /// `/consensus_state`: get current consensus state
    async fn consensus_state(&self) -> Result<consensus_state::Response, Error> {
        self.perform(consensus_state::Request::new()).await
    }

    // TODO(thane): Simplify once validators endpoint removes pagination.
    /// `/validators`: get validators a given height.
    async fn validators<H>(&self, height: H, paging: Paging) -> Result<validators::Response, Error>
    where
        H: Into<Height> + Send,
    {
        let height = height.into();
        match paging {
            Paging::Default => {
                self.perform(validators::Request::new(Some(height), None, None))
                    .await
            }
            Paging::Specific {
                page_number,
                per_page,
            } => {
                self.perform(validators::Request::new(
                    Some(height),
                    Some(page_number),
                    Some(per_page),
                ))
                .await
            }
            Paging::All => {
                let mut page_num = 1_usize;
                let mut validators = Vec::new();
                let per_page = DEFAULT_VALIDATORS_PER_PAGE.into();
                loop {
                    let response = self
                        .perform(validators::Request::new(
                            Some(height),
                            Some(page_num.into()),
                            Some(per_page),
                        ))
                        .await?;
                    validators.extend(response.validators);
                    if validators.len() as i32 == response.total {
                        return Ok(validators::Response::new(
                            response.block_height,
                            validators,
                            response.total,
                        ));
                    }
                    page_num += 1;
                }
            }
        }
    }

    /// `/consensus_params`: get the latest consensus parameters.
    async fn latest_consensus_params(&self) -> Result<consensus_params::Response, Error> {
        self.perform(consensus_params::Request::new(None)).await
    }

    /// `/commit`: get the latest block commit
    async fn latest_commit(&self) -> Result<commit::Response, Error> {
        self.perform(commit::Request::default()).await
    }

    /// `/health`: get node health.
    ///
    /// Returns empty result (200 OK) on success, no response in case of an error.
    async fn health(&self) -> Result<(), Error> {
        self.perform(health::Request).await?;
        Ok(())
    }

    /// `/genesis`: get genesis file.
    async fn genesis<AppState>(&self) -> Result<Genesis<AppState>, Error>
    where
        AppState: fmt::Debug + Serialize + DeserializeOwned + Send,
    {
        Ok(self.perform(genesis::Request::default()).await?.genesis)
    }

    /// `/net_info`: obtain information about P2P and other network connections.
    async fn net_info(&self) -> Result<net_info::Response, Error> {
        self.perform(net_info::Request).await
    }

    /// `/status`: get Tendermint status including node info, pubkey, latest
    /// block hash, app hash, block height and time.
    async fn status(&self) -> Result<status::Response, Error> {
        self.perform(status::Request).await
    }

    /// `/broadcast_evidence`: broadcast an evidence.
    async fn broadcast_evidence(&self, e: Evidence) -> Result<evidence::Response, Error> {
        self.perform(evidence::Request::new(e)).await
    }

    /// `/tx`: find transaction by hash.
    async fn tx(&self, hash: Hash, prove: bool) -> Result<tx::Response, Error> {
        self.perform(tx::Request::new(hash, prove)).await
    }

    /// `/tx_search`: search for transactions with their results.
    async fn tx_search(
        &self,
        query: Query,
        prove: bool,
        page: u32,
        per_page: u8,
        order: Order,
    ) -> Result<tx_search::Response, Error> {
        self.perform(tx_search::Request::new(query, prove, page, per_page, order))
            .await
    }

    #[cfg(any(
        feature = "tendermint-rpc-http-client",
        feature = "tendermint-rpc-websocket-client"
    ))]
    /// Poll the `/health` endpoint until it returns a successful result or
    /// the given `timeout` has elapsed.
    async fn wait_until_healthy<T>(&self, timeout: T) -> Result<(), Error>
    where
        T: Into<core::time::Duration> + Send,
    {
        let timeout = timeout.into();
        let poll_interval = core::time::Duration::from_millis(200);
        let mut attempts_remaining = timeout.as_millis() / poll_interval.as_millis();

        while self.health().await.is_err() {
            if attempts_remaining == 0 {
                return Err(Error::timeout(timeout));
            }

            attempts_remaining -= 1;
            tokio::time::sleep(poll_interval).await;
        }

        Ok(())
    }

    /// Perform a request against the RPC endpoint.
    ///
    /// This method is used by the default implementations of specific
    /// endpoint methods. The latest protocol dialect is assumed to be invoked.
    async fn perform<R>(&self, request: R) -> Result<R::Output, Error>
    where
        R: SimpleRequest;
}

#[cfg(not(target_arch = "wasm32"))]
mod non_wasm {
    use super::*;
    use cosmrs::tendermint::abci::response::Info;
    use std::fmt::Debug;
    use tendermint_rpc::endpoint::abci_query::AbciQuery;
    use tendermint_rpc::endpoint::block::Response;

    #[async_trait]
    impl<C> TendermintRpcClient for C
    where
        C: tendermint_rpc::client::Client + Sync,
    {
        async fn abci_info(&self) -> Result<Info, Error> {
            self.abci_info().await
        }

        async fn abci_query<V>(
            &self,
            path: Option<String>,
            data: V,
            height: Option<Height>,
            prove: bool,
        ) -> Result<AbciQuery, Error>
        where
            V: Into<Vec<u8>> + Send,
        {
            self.abci_query(path, data, height, prove).await
        }

        async fn block<H>(&self, height: H) -> Result<Response, Error>
        where
            H: Into<Height> + Send,
        {
            self.block(height).await
        }

        async fn block_by_hash(&self, hash: Hash) -> Result<block_by_hash::Response, Error> {
            self.block_by_hash(hash).await
        }

        async fn latest_block(&self) -> Result<Response, Error> {
            self.latest_block().await
        }

        async fn header<H>(&self, height: H) -> Result<header::Response, Error>
        where
            H: Into<Height> + Send,
        {
            self.header(height).await
        }

        async fn header_by_hash(&self, hash: Hash) -> Result<header_by_hash::Response, Error> {
            self.header_by_hash(hash).await
        }

        async fn block_results<H>(&self, height: H) -> Result<block_results::Response, Error>
        where
            H: Into<Height> + Send,
        {
            self.block_results(height).await
        }

        async fn latest_block_results(&self) -> Result<block_results::Response, Error> {
            self.latest_block_results().await
        }

        async fn block_search(
            &self,
            query: Query,
            page: u32,
            per_page: u8,
            order: Order,
        ) -> Result<block_search::Response, Error> {
            self.block_search(query, page, per_page, order).await
        }

        async fn blockchain<H>(&self, min: H, max: H) -> Result<blockchain::Response, Error>
        where
            H: Into<Height> + Send,
        {
            self.blockchain(min, max).await
        }

        async fn broadcast_tx_async<T>(&self, tx: T) -> Result<broadcast::tx_async::Response, Error>
        where
            T: Into<Vec<u8>> + Send,
        {
            self.broadcast_tx_async(tx).await
        }

        async fn broadcast_tx_sync<T>(&self, tx: T) -> Result<broadcast::tx_sync::Response, Error>
        where
            T: Into<Vec<u8>> + Send,
        {
            self.broadcast_tx_sync(tx).await
        }

        async fn broadcast_tx_commit<T>(
            &self,
            tx: T,
        ) -> Result<broadcast::tx_commit::Response, Error>
        where
            T: Into<Vec<u8>> + Send,
        {
            self.broadcast_tx_commit(tx).await
        }

        async fn commit<H>(&self, height: H) -> Result<commit::Response, Error>
        where
            H: Into<Height> + Send,
        {
            self.commit(height).await
        }

        async fn consensus_params<H>(&self, height: H) -> Result<consensus_params::Response, Error>
        where
            H: Into<Height> + Send,
        {
            self.consensus_params(height).await
        }

        async fn consensus_state(&self) -> Result<consensus_state::Response, Error> {
            self.consensus_state().await
        }

        async fn validators<H>(
            &self,
            height: H,
            paging: Paging,
        ) -> Result<validators::Response, Error>
        where
            H: Into<Height> + Send,
        {
            self.validators(height, paging).await
        }

        async fn latest_consensus_params(&self) -> Result<consensus_params::Response, Error> {
            self.latest_consensus_params().await
        }

        async fn latest_commit(&self) -> Result<commit::Response, Error> {
            self.latest_commit().await
        }

        async fn health(&self) -> Result<(), Error> {
            self.health().await
        }

        async fn genesis<AppState>(&self) -> Result<Genesis<AppState>, Error>
        where
            AppState: Debug + Serialize + DeserializeOwned + Send,
        {
            self.genesis().await
        }

        async fn net_info(&self) -> Result<net_info::Response, Error> {
            self.net_info().await
        }

        async fn status(&self) -> Result<status::Response, Error> {
            self.status().await
        }

        async fn broadcast_evidence(&self, e: Evidence) -> Result<evidence::Response, Error> {
            self.broadcast_evidence(e).await
        }

        async fn tx(&self, hash: Hash, prove: bool) -> Result<tx::Response, Error> {
            self.tx(hash, prove).await
        }

        async fn tx_search(
            &self,
            query: Query,
            prove: bool,
            page: u32,
            per_page: u8,
            order: Order,
        ) -> Result<tx_search::Response, Error> {
            self.tx_search(query, prove, page, per_page, order).await
        }

        #[cfg(any(
            feature = "tendermint-rpc-http-client",
            feature = "tendermint-rpc-websocket-client"
        ))]
        async fn wait_until_healthy<T>(&self, timeout: T) -> Result<(), Error>
        where
            T: Into<core::time::Duration> + Send,
        {
            self.wait_until_healthy(timeout).await
        }

        async fn perform<R>(&self, request: R) -> Result<R::Output, Error>
        where
            R: SimpleRequest,
        {
            self.perform(request).await
        }
    }
}
