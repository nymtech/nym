// Copyright 2023 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use crate::nyxd::coin::Coin;
use crate::nyxd::cosmwasm_client::types::{
    Account, CodeDetails, Contract, ContractCodeId, Model, SequenceResponse, SimulateResponse,
};
use crate::nyxd::error::NyxdError;
use crate::nyxd::{Height, Query};
use crate::rpc::{TendermintRpcClient, TendermintRpcClientExt};
use async_trait::async_trait;
use cosmrs::cosmwasm::{CodeInfoResponse, ContractCodeHistoryEntry};
use cosmrs::proto::cosmos::tx::v1beta1::{
    SimulateRequest, SimulateResponse as ProtoSimulateResponse,
};
use cosmrs::proto::cosmwasm::wasm::v1::{
    QueryAllContractStateRequest, QueryAllContractStateResponse, QueryCodeRequest,
    QueryCodeResponse, QueryCodesRequest, QueryCodesResponse, QueryContractHistoryRequest,
    QueryContractHistoryResponse, QueryContractInfoRequest, QueryContractInfoResponse,
    QueryContractsByCodeRequest, QueryContractsByCodeResponse, QueryRawContractStateRequest,
    QueryRawContractStateResponse, QuerySmartContractStateRequest, QuerySmartContractStateResponse,
};
use cosmrs::tendermint::{block, chain, Hash};
use prost::Message;
use serde::{Deserialize, Serialize};
use std::iter::once;

use cosmrs::AccountId;
use std::time::Duration;
use tendermint_rpc::endpoint::{
    block::Response as BlockResponse, broadcast, tx::Response as TxResponse,
};

use crate::nyxd::helpers::{create_pagination, next_page_key};
use crate::rpc::types::ProvableAbciQueryResponse;

pub const DEFAULT_BROADCAST_POLLING_RATE: Duration = Duration::from_secs(4);
pub const DEFAULT_BROADCAST_TIMEOUT: Duration = Duration::from_secs(60);

// this trait should only be concerned with the cosmwasm module,
// so all other legacy methods are deprecated and will be removed in the future
#[cfg_attr(target_arch = "wasm32", async_trait(?Send))]
#[cfg_attr(not(target_arch = "wasm32"), async_trait)]
pub trait CosmWasmClient: TendermintRpcClientExt {
    // helper method to remove duplicate code involved in making abci requests with protobuf messages
    // TODO: perhaps it should have an additional argument to determine whether the response should
    // require proof?
    #[deprecated(note = "use TendermintRpcClientExt::make_abci_query_without_proof instead")]
    async fn make_abci_query<Req, Res>(
        &self,
        path: Option<String>,
        req: Req,
    ) -> Result<Res, NyxdError>
    where
        Req: Message,
        Res: Message + Default,
    {
        TendermintRpcClientExt::make_abci_query_without_proof(self, path, req, None).await
    }

    #[deprecated(note = "use TendermintRpcClientExt::get_chain_id instead")]
    async fn get_chain_id(&self) -> Result<chain::Id, NyxdError> {
        TendermintRpcClientExt::get_chain_id(self).await
    }

    #[deprecated(note = "use TendermintRpcClientExt::get_height instead")]
    async fn get_height(&self) -> Result<block::Height, NyxdError> {
        TendermintRpcClientExt::get_height(self).await
    }

    #[deprecated(note = "use TendermintRpcClientExt::get_account instead")]
    async fn get_account(&self, address: &AccountId) -> Result<Option<Account>, NyxdError> {
        TendermintRpcClientExt::get_account(self, address).await
    }

    #[deprecated(note = "use TendermintRpcClientExt::get_sequence instead")]
    async fn get_sequence(&self, address: &AccountId) -> Result<SequenceResponse, NyxdError> {
        TendermintRpcClientExt::get_sequence(self, address).await
    }

    #[deprecated(note = "use TendermintRpcClientExt::get_block instead")]
    async fn get_block(&self, height: Option<u32>) -> Result<BlockResponse, NyxdError> {
        TendermintRpcClientExt::get_block(self, height).await
    }

    #[deprecated(note = "use TendermintRpcClientExt::get_balance instead")]
    async fn get_balance(
        &self,
        address: &AccountId,
        search_denom: String,
    ) -> Result<Option<Coin>, NyxdError> {
        TendermintRpcClientExt::get_balance(self, address, search_denom).await
    }

    #[deprecated(note = "use TendermintRpcClientExt::get_all_balances instead")]
    async fn get_all_balances(&self, address: &AccountId) -> Result<Vec<Coin>, NyxdError> {
        TendermintRpcClientExt::get_all_balances(self, address).await
    }

    #[deprecated(note = "use TendermintRpcClientExt::get_total_supply instead")]
    async fn get_total_supply(&self) -> Result<Vec<Coin>, NyxdError> {
        TendermintRpcClientExt::get_total_supply(self).await
    }

    #[deprecated(note = "use TendermintRpcClientExt::get_tx instead")]
    async fn get_tx(&self, id: Hash) -> Result<TxResponse, NyxdError> {
        TendermintRpcClientExt::get_tx(self, id).await
    }

    #[deprecated(note = "use TendermintRpcClientExt::search_tx instead")]
    async fn search_tx(&self, query: Query) -> Result<Vec<TxResponse>, NyxdError> {
        TendermintRpcClientExt::search_tx(self, query).await
    }

    /// Broadcast a transaction, returning immediately.
    #[deprecated(note = "use TendermintRpcClientExt::broadcast_tx_async instead")]
    async fn broadcast_tx_async<T>(&self, tx: T) -> Result<broadcast::tx_async::Response, NyxdError>
    where
        T: Into<Vec<u8>> + Send,
    {
        TendermintRpcClientExt::broadcast_tx_async(self, tx).await
    }

    /// Broadcast a transaction, returning the response from `CheckTx`.
    #[deprecated(note = "use TendermintRpcClientExt::broadcast_tx_sync instead")]
    async fn broadcast_tx_sync<T>(&self, tx: T) -> Result<broadcast::tx_sync::Response, NyxdError>
    where
        T: Into<Vec<u8>> + Send,
    {
        TendermintRpcClientExt::broadcast_tx_sync(self, tx).await
    }

    /// Broadcast a transaction, returning the response from `DeliverTx`.
    #[deprecated(note = "use TendermintRpcClientExt::broadcast_tx_commit instead")]
    async fn broadcast_tx_commit<T>(
        &self,
        tx: T,
    ) -> Result<broadcast::tx_commit::Response, NyxdError>
    where
        T: Into<Vec<u8>> + Send,
    {
        TendermintRpcClientExt::broadcast_tx_commit(self, tx).await
    }

    #[deprecated(note = "use TendermintRpcClientExt::broadcast_tx instead")]
    async fn broadcast_tx<T>(
        &self,
        tx: T,
        timeout: impl Into<Option<Duration>> + Send,
        poll_interval: impl Into<Option<Duration>> + Send,
    ) -> Result<TxResponse, NyxdError>
    where
        T: Into<Vec<u8>> + Send,
    {
        TendermintRpcClientExt::broadcast_tx(self, tx, timeout, poll_interval).await
    }

    async fn get_codes(&self) -> Result<Vec<CodeInfoResponse>, NyxdError> {
        let path = Some("/cosmwasm.wasm.v1.Query/Codes".to_owned());

        let mut raw_codes = Vec::new();
        let mut pagination = None;

        loop {
            let req = QueryCodesRequest { pagination };

            let mut res = self
                .make_abci_query_without_proof::<_, QueryCodesResponse>(path.clone(), req, None)
                .await?;

            let early_break = res.code_infos.is_empty();
            raw_codes.append(&mut res.code_infos);

            if early_break {
                break;
            }

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
            .make_abci_query_without_proof::<_, QueryCodeResponse>(path, req, None)
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
                .make_abci_query_without_proof::<_, QueryContractsByCodeResponse>(
                    path.clone(),
                    req,
                    None,
                )
                .await?;

            let early_break = res.contracts.is_empty();
            raw_contracts.append(&mut res.contracts);

            if early_break {
                break;
            }

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
            .make_abci_query_without_proof::<_, QueryContractInfoResponse>(path, req, None)
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
                .make_abci_query_without_proof::<_, QueryContractHistoryResponse>(
                    path.clone(),
                    req,
                    None,
                )
                .await?;

            let early_break = res.entries.is_empty();
            raw_entries.append(&mut res.entries);

            if early_break {
                break;
            }

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

    async fn query_all_contract_state(&self, address: &AccountId) -> Result<Vec<Model>, NyxdError> {
        let path = Some("/cosmwasm.wasm.v1.Query/AllContractState".to_owned());

        let mut models = Vec::new();
        let mut pagination = None;

        loop {
            let req = QueryAllContractStateRequest {
                address: address.to_string(),
                pagination,
            };

            let mut res = self
                .make_abci_query_without_proof::<_, QueryAllContractStateResponse>(
                    path.clone(),
                    req,
                    None,
                )
                .await?;

            let empty_response = res.models.is_empty();
            models.append(&mut res.models);

            if empty_response {
                break;
            }
            if let Some(next_key) = next_page_key(res.pagination) {
                pagination = Some(create_pagination(next_key))
            } else {
                break;
            }
        }

        Ok(models.into_iter().map(Into::into).collect())
    }

    async fn query_contract_raw(
        &self,
        address: &AccountId,
        query_data: Vec<u8>,
    ) -> Result<Vec<u8>, NyxdError> {
        self.query_contract_raw_at_height(address, query_data, None)
            .await
    }

    async fn query_contract_raw_at_height(
        &self,
        address: &AccountId,
        query_data: Vec<u8>,
        height: Option<Height>,
    ) -> Result<Vec<u8>, NyxdError> {
        let path = Some("/cosmwasm.wasm.v1.Query/RawContractState".to_owned());

        let req = QueryRawContractStateRequest {
            address: address.to_string(),
            query_data,
        };

        let res = self
            .make_abci_query_without_proof::<_, QueryRawContractStateResponse>(path, req, height)
            .await?;

        Ok(res.data)
    }

    async fn query_contract_raw_with_proof(
        &self,
        address: &AccountId,
        query_data: Vec<u8>,
        height: Option<Height>,
    ) -> Result<ProvableAbciQueryResponse<Vec<u8>>, NyxdError> {
        let path = Some("/store/wasm/key".to_owned());

        // 0x03 is the 'ContractStorePrefix' constant
        // taken from https://github.com/CosmWasm/wasmd/blob/v0.60.0/x/wasm/types/keys.go#L30

        // the actual storage key is '0x03 || contract_address_bytes || namespaced_key'
        // (after tracing the calls within QueryRaw)
        // https://github.com/CosmWasm/wasmd/blob/v0.60.0/x/wasm/keeper/keeper.go#L924-L926
        let mut key = vec![0x03];
        key.extend_from_slice(&address.to_bytes());
        key.extend_from_slice(&query_data);

        self.make_raw_abci_query_with_proof(path, key, height).await
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
        self.query_contract_smart_at_height(address, query_msg, None)
            .await
    }

    async fn query_contract_smart_at_height<M, T>(
        &self,
        address: &AccountId,
        query_msg: &M,
        height: Option<Height>,
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
            .make_abci_query_without_proof::<_, QuerySmartContractStateResponse>(path, req, height)
            .await?;

        tracing::trace!("raw query response: {}", String::from_utf8_lossy(&res.data));
        Ok(serde_json::from_slice(&res.data)?)
    }

    #[deprecated(note = "use TendermintRpcClientExt::query_simulate instead")]
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

#[cfg_attr(target_arch = "wasm32", async_trait(?Send))]
#[cfg_attr(not(target_arch = "wasm32"), async_trait)]
impl<T> CosmWasmClient for T where T: TendermintRpcClient {}
