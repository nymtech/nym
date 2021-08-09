// Copyright 2021 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use crate::nymd::cosmwasm_client::helpers::create_pagination;
use crate::nymd::cosmwasm_client::types::{
    Account, Code, CodeDetails, Contract, ContractCodeHistoryEntry, ContractCodeId,
    SequenceResponse,
};
use crate::ValidatorClientError;
use async_trait::async_trait;
use cosmos_sdk::proto::cosmos::auth::v1beta1::{
    BaseAccount, QueryAccountRequest, QueryAccountResponse,
};
use cosmos_sdk::proto::cosmos::bank::v1beta1::{
    QueryAllBalancesRequest, QueryAllBalancesResponse, QueryBalanceRequest, QueryBalanceResponse,
};
use cosmos_sdk::proto::cosmwasm::wasm::v1beta1::*;
use cosmos_sdk::rpc::endpoint::block::Response as BlockResponse;
use cosmos_sdk::rpc::endpoint::broadcast;
use cosmos_sdk::rpc::endpoint::tx::Response as TxResponse;
use cosmos_sdk::rpc::query::Query;
use cosmos_sdk::rpc::{self, HttpClient, Order};
use cosmos_sdk::tendermint::abci::Transaction;
use cosmos_sdk::tendermint::{abci, block, chain};
use cosmos_sdk::{tx, AccountId, Coin, Denom};
use prost::Message;
use serde::{Deserialize, Serialize};
use std::convert::{TryFrom, TryInto};

#[async_trait]
impl CosmWasmClient for HttpClient {}

#[async_trait]
pub trait CosmWasmClient: rpc::Client {
    // helper method to remove duplicate code involved in making abci requests with protobuf messages
    // TODO: perhaps it should have an additional argument to determine whether the response should
    // require proof?
    async fn make_abci_query<Req, Res>(
        &self,
        path: Option<abci::Path>,
        req: Req,
    ) -> Result<Res, ValidatorClientError>
    where
        Req: Message,
        Res: Message + Default,
    {
        let mut buf = Vec::with_capacity(req.encoded_len());
        req.encode(&mut buf)?;

        let res = self.abci_query(path, buf, None, false).await?;

        Ok(Res::decode(res.value.as_ref())?)
    }

    async fn get_chain_id(&self) -> Result<chain::Id, ValidatorClientError> {
        Ok(self.status().await?.node_info.network)
    }

    async fn get_height(&self) -> Result<block::Height, ValidatorClientError> {
        Ok(self.status().await?.sync_info.latest_block_height)
    }

    // TODO: the return type should probably be changed to a non-proto, type-safe Account alternative
    async fn get_account(
        &self,
        address: &AccountId,
    ) -> Result<Option<Account>, ValidatorClientError> {
        let path = Some("/cosmos.auth.v1beta1.Query/Account".parse().unwrap());

        let req = QueryAccountRequest {
            address: address.to_string(),
        };

        let res = self
            .make_abci_query::<_, QueryAccountResponse>(path, req)
            .await?;

        let base_account = res
            .account
            .map(|account| BaseAccount::decode(account.value.as_ref()))
            .transpose()?;

        base_account
            .map(|base_account| base_account.try_into())
            .transpose()
    }

    async fn get_sequence(
        &self,
        address: &AccountId,
    ) -> Result<SequenceResponse, ValidatorClientError> {
        let base_account = self
            .get_account(address)
            .await?
            .ok_or_else(|| ValidatorClientError::NonExistentAccountError(address.clone()))?;
        Ok(SequenceResponse {
            account_number: base_account.account_number,
            sequence: base_account.sequence,
        })
    }

    async fn get_block(&self, height: Option<u32>) -> Result<BlockResponse, ValidatorClientError> {
        match height {
            Some(height) => self.block(height).await.map_err(|err| err.into()),
            None => self.latest_block().await.map_err(|err| err.into()),
        }
    }

    async fn get_balance(
        &self,
        address: &AccountId,
        search_denom: Denom,
    ) -> Result<Option<Coin>, ValidatorClientError> {
        let path = Some("/cosmos.bank.v1beta1.Query/Balance".parse().unwrap());

        let req = QueryBalanceRequest {
            address: address.to_string(),
            denom: search_denom.to_string(),
        };

        let res = self
            .make_abci_query::<_, QueryBalanceResponse>(path, req)
            .await?;

        res.balance
            .map(TryFrom::try_from)
            .transpose()
            .map_err(|_| ValidatorClientError::SerializationError("Coin".to_owned()))
    }

    async fn get_all_balances(
        &self,
        address: &AccountId,
    ) -> Result<Vec<Coin>, ValidatorClientError> {
        let path = Some("/cosmos.bank.v1beta1.Query/AllBalances".parse().unwrap());

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
            if let Some(pagination_info) = res.pagination {
                pagination = Some(create_pagination(pagination_info.next_key))
            } else {
                break;
            }
        }

        raw_balances
            .into_iter()
            .map(TryFrom::try_from)
            .collect::<Result<_, _>>()
            .map_err(|_| ValidatorClientError::SerializationError("Coins".to_owned()))
    }

    async fn get_tx(&self, id: tx::Hash) -> Result<TxResponse, ValidatorClientError> {
        Ok(self.tx(id, false).await?)
    }

    async fn search_tx(&self, query: Query) -> Result<Vec<TxResponse>, ValidatorClientError> {
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
    async fn broadcast_tx_async(
        &self,
        tx: Transaction,
    ) -> Result<broadcast::tx_async::Response, ValidatorClientError> {
        Ok(rpc::Client::broadcast_tx_async(self, tx).await?)
    }

    /// Broadcast a transaction, returning the response from `CheckTx`.
    async fn broadcast_tx_sync(
        &self,
        tx: Transaction,
    ) -> Result<broadcast::tx_sync::Response, ValidatorClientError> {
        Ok(rpc::Client::broadcast_tx_sync(self, tx).await?)
    }

    /// Broadcast a transaction, returning the response from `DeliverTx`.
    async fn broadcast_tx_commit(
        &self,
        tx: Transaction,
    ) -> Result<broadcast::tx_commit::Response, ValidatorClientError> {
        Ok(rpc::Client::broadcast_tx_commit(self, tx).await?)
    }

    async fn get_codes(&self) -> Result<Vec<Code>, ValidatorClientError> {
        let path = Some("/cosmwasm.wasm.v1beta1.Query/Codes".parse().unwrap());

        let mut raw_codes = Vec::new();
        let mut pagination = None;

        loop {
            let req = QueryCodesRequest { pagination };

            let mut res = self
                .make_abci_query::<_, QueryCodesResponse>(path.clone(), req)
                .await?;

            raw_codes.append(&mut res.code_infos);
            if let Some(pagination_info) = res.pagination {
                pagination = Some(create_pagination(pagination_info.next_key))
            } else {
                break;
            }
        }

        raw_codes
            .into_iter()
            .map(TryFrom::try_from)
            .collect::<Result<_, _>>()
    }

    async fn get_code_details(
        &self,
        code_id: ContractCodeId,
    ) -> Result<CodeDetails, ValidatorClientError> {
        let path = Some("/cosmwasm.wasm.v1beta1.Query/Code".parse().unwrap());

        let req = QueryCodeRequest { code_id };

        let res = self
            .make_abci_query::<_, QueryCodeResponse>(path, req)
            .await?;

        if let Some(code_info) = res.code_info {
            Ok(CodeDetails::new(code_info.try_into()?, res.data))
        } else {
            Err(ValidatorClientError::NoCodeInformation(code_id))
        }
    }
    async fn get_contracts(
        &self,
        code_id: ContractCodeId,
    ) -> Result<Vec<AccountId>, ValidatorClientError> {
        let path = Some(
            "/cosmwasm.wasm.v1beta1.Query/ContractsByCode"
                .parse()
                .unwrap(),
        );

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
            if let Some(pagination_info) = res.pagination {
                pagination = Some(create_pagination(pagination_info.next_key))
            } else {
                break;
            }
        }

        raw_contracts
            .iter()
            .map(|raw| raw.parse())
            .collect::<Result<_, _>>()
            .map_err(|_| {
                ValidatorClientError::DeserializationError("Contract addresses".to_owned())
            })
    }

    async fn get_contract(&self, address: &AccountId) -> Result<Contract, ValidatorClientError> {
        let path = Some("/cosmwasm.wasm.v1beta1.Query/ContractInfo".parse().unwrap());

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
                .map_err(|_| ValidatorClientError::MalformedAccountAddress(response_address))?;
            Ok(Contract::new(address, contract_info.try_into()?))
        } else {
            Err(ValidatorClientError::NoContractInformation(address.clone()))
        }
    }

    async fn get_contract_code_history(
        &self,
        address: &AccountId,
    ) -> Result<Vec<ContractCodeHistoryEntry>, ValidatorClientError> {
        let path = Some(
            "/cosmwasm.wasm.v1beta1.Query/ContractHistory"
                .parse()
                .unwrap(),
        );

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
            if let Some(pagination_info) = res.pagination {
                pagination = Some(create_pagination(pagination_info.next_key))
            } else {
                break;
            }
        }

        raw_entries
            .into_iter()
            .map(TryFrom::try_from)
            .collect::<Result<_, _>>()
    }

    async fn query_contract_raw(
        &self,
        address: &AccountId,
        query_data: Vec<u8>,
    ) -> Result<Vec<u8>, ValidatorClientError> {
        let path = Some(
            "/cosmwasm.wasm.v1beta1.Query/RawContractState"
                .parse()
                .unwrap(),
        );

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
    ) -> Result<T, ValidatorClientError>
    where
        M: ?Sized + Serialize + Sync,
        for<'a> T: Deserialize<'a>,
    {
        let path = Some(
            "/cosmwasm.wasm.v1beta1.Query/SmartContractState"
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

        Ok(serde_json::from_slice(&res.data)?)
    }
}
