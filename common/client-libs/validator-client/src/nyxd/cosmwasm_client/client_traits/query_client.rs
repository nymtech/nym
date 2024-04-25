// Copyright 2023 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use crate::nyxd;
use crate::nyxd::coin::Coin;
use crate::nyxd::cosmwasm_client::helpers::{create_pagination, next_page_key};
use crate::nyxd::cosmwasm_client::types::{
    Account, CodeDetails, Contract, ContractCodeId, SequenceResponse, SimulateResponse,
};
use crate::nyxd::error::NyxdError;
use crate::nyxd::Query;
use crate::rpc::TendermintRpcClient;
use async_trait::async_trait;
use cosmrs::cosmwasm::{CodeInfoResponse, ContractCodeHistoryEntry};
use cosmrs::proto::cosmos::auth::v1beta1::{QueryAccountRequest, QueryAccountResponse};
use cosmrs::proto::cosmos::bank::v1beta1::{
    QueryAllBalancesRequest, QueryAllBalancesResponse, QueryBalanceRequest, QueryBalanceResponse,
    QueryTotalSupplyRequest, QueryTotalSupplyResponse,
};
use cosmrs::proto::cosmos::tx::v1beta1::{
    SimulateRequest, SimulateResponse as ProtoSimulateResponse,
};
use cosmrs::proto::cosmwasm::wasm::v1::{
    QueryCodeRequest, QueryCodeResponse, QueryCodesRequest, QueryCodesResponse,
    QueryContractHistoryRequest, QueryContractHistoryResponse, QueryContractInfoRequest,
    QueryContractInfoResponse, QueryContractsByCodeRequest, QueryContractsByCodeResponse,
    QueryRawContractStateRequest, QueryRawContractStateResponse, QuerySmartContractStateRequest,
    QuerySmartContractStateResponse,
};
use cosmrs::tendermint::{block, chain, Hash};
use cosmrs::{AccountId, Coin as CosmosCoin, Tx};
use log::trace;
use prost::Message;
use serde::{Deserialize, Serialize};

use std::time::Duration;
use tendermint_rpc::{
    endpoint::{block::Response as BlockResponse, broadcast, tx::Response as TxResponse},
    Order,
};

#[cfg(not(target_arch = "wasm32"))]
use tokio::time::sleep;
#[cfg(not(target_arch = "wasm32"))]
use tokio::time::Instant;

#[cfg(target_arch = "wasm32")]
use wasmtimer::std::Instant;
#[cfg(target_arch = "wasm32")]
use wasmtimer::tokio::sleep;

pub const DEFAULT_BROADCAST_POLLING_RATE: Duration = Duration::from_secs(4);
pub const DEFAULT_BROADCAST_TIMEOUT: Duration = Duration::from_secs(60);

#[cfg_attr(target_arch = "wasm32", async_trait(?Send))]
#[cfg_attr(not(target_arch = "wasm32"), async_trait)]
pub trait CosmWasmClient: TendermintRpcClient {
    // helper method to remove duplicate code involved in making abci requests with protobuf messages
    // TODO: perhaps it should have an additional argument to determine whether the response should
    // require proof?
    async fn make_abci_query<Req, Res>(
        &self,
        path: Option<String>,
        req: Req,
    ) -> Result<Res, NyxdError>
    where
        Req: Message,
        Res: Message + Default,
    {
        if let Some(ref abci_path) = path {
            trace!("performing query on abci path {abci_path}")
        }
        let mut buf = Vec::with_capacity(req.encoded_len());
        req.encode(&mut buf)?;

        let res = self.abci_query(path, buf, None, false).await?;
        let res_success = nyxd::error::parse_abci_query_result(res)?;

        Ok(Res::decode(res_success.value.as_ref())?)
    }

    async fn get_chain_id(&self) -> Result<chain::Id, NyxdError> {
        Ok(self.status().await?.node_info.network)
    }

    async fn get_height(&self) -> Result<block::Height, NyxdError> {
        Ok(self.status().await?.sync_info.latest_block_height)
    }

    // TODO: the return type should probably be changed to a non-proto, type-safe Account alternative
    async fn get_account(&self, address: &AccountId) -> Result<Option<Account>, NyxdError> {
        let path = Some("/cosmos.auth.v1beta1.Query/Account".to_owned());

        let req = QueryAccountRequest {
            address: address.to_string(),
        };

        let res = self
            .make_abci_query::<_, QueryAccountResponse>(path, req)
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
            denom: search_denom.to_string(),
        };

        let res = self
            .make_abci_query::<_, QueryBalanceResponse>(path, req)
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
            };

            let mut res = self
                .make_abci_query::<_, QueryAllBalancesResponse>(path.clone(), req)
                .await?;

            raw_balances.append(&mut res.balances);
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
                .make_abci_query::<_, QueryTotalSupplyResponse>(path.clone(), req)
                .await?;

            supply.append(&mut res.supply);
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
                .tx_search(query.clone(), false, page, 100, Order::Ascending)
                .await?;

            results.append(&mut res.txs);
            // sanity check for if tendermint's maximum per_page was modified -
            // we don't want to accidentally be stuck in an infinite loop
            if res.total_count == 0 || res.txs.is_empty() {
                break;
            }

            if res.total_count >= per_page {
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

        let broadcasted = CosmWasmClient::broadcast_tx_sync(self, tx).await?;

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
            log::debug!(
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

    async fn get_codes(&self) -> Result<Vec<CodeInfoResponse>, NyxdError> {
        let path = Some("/cosmwasm.wasm.v1.Query/Codes".to_owned());

        let mut raw_codes = Vec::new();
        let mut pagination = None;

        loop {
            let req = QueryCodesRequest { pagination };

            let mut res = self
                .make_abci_query::<_, QueryCodesResponse>(path.clone(), req)
                .await?;

            raw_codes.append(&mut res.code_infos);
            if let Some(next_key) = next_page_key(res.pagination) {
                pagination = Some(create_pagination(next_key))
            } else {
                break;
            }
        }

        Ok(raw_codes
            .into_iter()
            .map(TryFrom::try_from)
            .collect::<Result<_, _>>()?)
    }

    async fn get_code_details(&self, code_id: ContractCodeId) -> Result<CodeDetails, NyxdError> {
        let path = Some("/cosmwasm.wasm.v1.Query/Code".to_owned());

        let req = QueryCodeRequest { code_id };

        let res = self
            .make_abci_query::<_, QueryCodeResponse>(path, req)
            .await?;

        if let Some(code_info) = res.code_info {
            Ok(CodeDetails::new(code_info.try_into()?, res.data))
        } else {
            Err(NyxdError::NoCodeInformation(code_id))
        }
    }
    async fn get_contracts(&self, code_id: ContractCodeId) -> Result<Vec<AccountId>, NyxdError> {
        let path = Some("/cosmwasm.wasm.v1.Query/ContractsByCode".to_owned());

        let mut raw_contracts = Vec::new();
        let mut pagination = None;

        loop {
            let req = QueryContractsByCodeRequest {
                code_id,
                pagination,
            };

            let mut res = self
                .make_abci_query::<_, QueryContractsByCodeResponse>(path.clone(), req)
                .await?;

            raw_contracts.append(&mut res.contracts);
            if let Some(next_key) = next_page_key(res.pagination) {
                pagination = Some(create_pagination(next_key))
            } else {
                break;
            }
        }

        raw_contracts
            .iter()
            .map(|raw| raw.parse())
            .collect::<Result<_, _>>()
            .map_err(|_| NyxdError::DeserializationError("Contract addresses".to_owned()))
    }

    async fn get_contract(&self, address: &AccountId) -> Result<Contract, NyxdError> {
        let path = Some("/cosmwasm.wasm.v1.Query/ContractInfo".to_owned());

        let req = QueryContractInfoRequest {
            address: address.to_string(),
        };

        let res = self
            .make_abci_query::<_, QueryContractInfoResponse>(path, req)
            .await?;

        let response_address = res.address;
        if let Some(contract_info) = res.contract_info {
            let address = response_address
                .parse()
                .map_err(|_| NyxdError::MalformedAccountAddress(response_address))?;
            Ok(Contract::new(address, contract_info.try_into()?))
        } else {
            Err(NyxdError::NoContractInformation(address.clone()))
        }
    }

    async fn get_contract_code_history(
        &self,
        address: &AccountId,
    ) -> Result<Vec<ContractCodeHistoryEntry>, NyxdError> {
        let path = Some("/cosmwasm.wasm.v1.Query/ContractHistory".to_owned());

        let mut raw_entries = Vec::new();
        let mut pagination = None;

        loop {
            let req = QueryContractHistoryRequest {
                address: address.to_string(),
                pagination,
            };

            let mut res = self
                .make_abci_query::<_, QueryContractHistoryResponse>(path.clone(), req)
                .await?;

            raw_entries.append(&mut res.entries);
            if let Some(next_key) = next_page_key(res.pagination) {
                pagination = Some(create_pagination(next_key))
            } else {
                break;
            }
        }

        Ok(raw_entries
            .into_iter()
            .map(TryFrom::try_from)
            .collect::<Result<_, _>>()?)
    }

    async fn query_contract_raw(
        &self,
        address: &AccountId,
        query_data: Vec<u8>,
    ) -> Result<Vec<u8>, NyxdError> {
        let path = Some("/cosmwasm.wasm.v1.Query/RawContractState".to_owned());

        let req = QueryRawContractStateRequest {
            address: address.to_string(),
            query_data,
        };

        let res = self
            .make_abci_query::<_, QueryRawContractStateResponse>(path, req)
            .await?;

        Ok(res.data)
    }

    async fn query_contract_smart<M, T>(
        &self,
        address: &AccountId,
        query_msg: &M,
    ) -> Result<T, NyxdError>
    where
        M: ?Sized + Serialize + Sync,
        for<'a> T: Deserialize<'a>,
    {
        let path = Some(
            "/cosmwasm.wasm.v1.Query/SmartContractState"
                .parse()
                .unwrap(),
        );

        // As per serde documentation:
        // Serialization can fail if `T`'s implementation of `Serialize` decides to
        // fail, or if `T` contains a map with non-string keys.
        let req = QuerySmartContractStateRequest {
            address: address.to_string(),
            query_data: serde_json::to_vec(query_msg)?,
        };

        let res = self
            .make_abci_query::<_, QuerySmartContractStateResponse>(path, req)
            .await?;

        trace!("raw query response: {}", String::from_utf8_lossy(&res.data));
        Ok(serde_json::from_slice(&res.data)?)
    }

    // deprecation warning is due to the fact the protobuf files built were based on cosmos-sdk 0.44,
    // where they prefer using tx_bytes directly. However, in 0.42, which we are using at the time
    // of writing this, the option does not work
    // TODO: we should really stop using the `tx` argument here and use `tx_bytes` exlusively,
    // however, at the time of writing this update, while our QA and mainnet networks do support it,
    // sandbox is still running old version of wasmd that lacks support for `tx_bytes`
    #[allow(deprecated)]
    async fn query_simulate(
        &self,
        tx: Option<Tx>,
        tx_bytes: Vec<u8>,
    ) -> Result<SimulateResponse, NyxdError> {
        let path = Some("/cosmos.tx.v1beta1.Service/Simulate".to_owned());

        let req = SimulateRequest {
            tx: tx.map(Into::into),
            tx_bytes,
        };

        let res = self
            .make_abci_query::<_, ProtoSimulateResponse>(path, req)
            .await?;

        res.try_into()
    }
}

#[cfg_attr(target_arch = "wasm32", async_trait(?Send))]
#[cfg_attr(not(target_arch = "wasm32"), async_trait)]
impl<T> CosmWasmClient for T where T: TendermintRpcClient {}
