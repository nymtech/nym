// Copyright 2021 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use crate::ValidatorClientError;
use async_trait::async_trait;
use cosmos_sdk::proto::cosmos::bank::v1beta1::{QueryAllBalancesRequest, QueryAllBalancesResponse};
use cosmos_sdk::rpc::endpoint::block::Response as BlockResponse;
use cosmos_sdk::rpc::endpoint::broadcast;
use cosmos_sdk::rpc::endpoint::tx::Response as TxResponse;
use cosmos_sdk::rpc::endpoint::tx_search::Response as TxSearchResponse;
use cosmos_sdk::rpc::query::Query;
use cosmos_sdk::rpc::{
    Client, Error as TendermintRpcError, HttpClient, HttpClientUrl, SimpleRequest,
};
use cosmos_sdk::tendermint::abci::Transaction;
use cosmos_sdk::tendermint::{block, chain};
use cosmos_sdk::tx::SequenceNumber;
use cosmos_sdk::{rpc, AccountId, Coin, Denom};
use prost::Message;
use std::convert::TryInto;

// #[async_trait]
pub struct CosmWasmClient {
    tm_client: HttpClient,
}

impl CosmWasmClient {
    pub fn connect<U>(endpoint: U) -> Result<Self, ValidatorClientError>
    where
        U: TryInto<HttpClientUrl, Error = TendermintRpcError>,
    {
        let tm_client = HttpClient::new(endpoint)?;

        Ok(CosmWasmClient { tm_client })
    }

    pub async fn get_chain_id(&self) -> Result<chain::Id, ValidatorClientError> {
        Ok(self.tm_client.status().await?.node_info.network)
    }

    pub async fn get_height(&self) -> Result<block::Height, ValidatorClientError> {
        Ok(self.tm_client.status().await?.sync_info.latest_block_height)
    }

    // getAccount(searchAddress: string): Promise<Account | null>;
    pub async fn get_account(&self, search_address: AccountId) -> Result<(), ValidatorClientError> {
        // here be abci query
        todo!()
    }

    // getSequence(address: string): Promise<SequenceResponse>;
    pub async fn get_sequence(
        &self,
        address: AccountId,
    ) -> Result<SequenceNumber, ValidatorClientError> {
        // here be abci query
        todo!()
    }

    pub async fn get_block(
        &self,
        height: Option<u32>,
    ) -> Result<BlockResponse, ValidatorClientError> {
        match height {
            Some(height) => self.tm_client.block(height).await.map_err(|err| err.into()),
            None => self
                .tm_client
                .latest_block()
                .await
                .map_err(|err| err.into()),
        }
    }

    pub async fn get_balance(
        &self,
        address: &AccountId,
        search_denom: Denom,
    ) -> Result<Coin, ValidatorClientError> {
        // here also be abci_query land
        todo!()
    }

    fn some_generic_abci_query_thing<Req, Res>(
        &self,
        req: Req,
    ) -> Result<Res, ValidatorClientError> {
        todo!()
    }

    pub async fn get_all_balances(
        &self,
        address: &AccountId,
    ) -> Result<Vec<Coin>, ValidatorClientError> {
        // here also be abci_query land

        let req = QueryAllBalancesRequest {
            address: address.to_string(),
            pagination: None,
        };

        let mut buf = Vec::with_capacity(req.encoded_len());
        req.encode(&mut buf)
            .expect("failed to encode our protobuf request!");

        // "/cosmos.auth.v1beta1.Query/Params"
        // let path = Some("/cosmos/auth/v1beta1/accounts".parse().unwrap());
        let path = Some("/cosmos.bank.v1beta1.Query/AllBalances".parse().unwrap());
        let res = self
            .tm_client
            .abci_query(path, buf, None, false)
            .await
            .unwrap();

        let res_parsed: QueryAllBalancesResponse =
            prost::Message::decode(res.value.as_ref()).unwrap();

        println!("{:?}", res_parsed);

        todo!()
    }

    // TODO: or should it instead take concrete Hash type directly?
    pub async fn get_tx(&self, id: &str) -> Result<TxResponse, ValidatorClientError> {
        let tx_hash = id
            .parse()
            .map_err(|_| ValidatorClientError::InvalidTxHash(id.to_owned()))?;
        Ok(self.tm_client.tx(tx_hash, false).await?)
    }

    pub async fn search_tx(&self, query: Query) -> Result<TxSearchResponse, ValidatorClientError> {
        todo!("need to construct pagination here")
        // self.http_client.tx_search(query, false, )
        /*
        /// `/tx_search`: search for transactions with their results.
        async fn tx_search(
            &self,
            query: Query,
            prove: bool,
            page: u32,
            per_page: u8,
            order: Order,
        ) -> Result<tx_search::Response> {
            self.perform(tx_search::Request::new(query, prove, page, per_page, order))
                .await
        }
         */
    }

    /// Broadcast a transaction, returning immediately.
    pub async fn broadcast_tx_async(
        &self,
        tx: Transaction,
    ) -> Result<broadcast::tx_async::Response, ValidatorClientError> {
        Ok(self.tm_client.broadcast_tx_async(tx).await?)
    }

    /// Broadcast a transaction, returning the response from `CheckTx`.
    pub async fn broadcast_tx_sync(
        &self,
        tx: Transaction,
    ) -> Result<broadcast::tx_sync::Response, ValidatorClientError> {
        Ok(self.tm_client.broadcast_tx_sync(tx).await?)
    }

    /// Broadcast a transaction, returning the response from `DeliverTx`.
    pub async fn broadcast_tx_commit(
        &self,
        tx: Transaction,
    ) -> Result<broadcast::tx_commit::Response, ValidatorClientError> {
        Ok(self.tm_client.broadcast_tx_commit(tx).await?)
    }

    // abci query action!
    // async fn get_codes(&self);
    //
    // async fn get_code_details(&self);
    //
    // async fn get_contracts(&self);
    //
    // async fn get_contract(&self);
    //
    // async fn get_contract_code_history(&self);
    //
    // async fn get_contract_raw(&self);
    //
    // async fn query_contract_smart(&self);

    // getCodes(): Promise<readonly Code[]>;
    // getCodeDetails(codeId: number): Promise<CodeDetails>;
    // getContracts(codeId: number): Promise<readonly string[]>;
    // /**
    //  * Throws an error if no contract was found at the address
    //  */
    // getContract(address: string): Promise<Contract>;
    // /**
    //  * Throws an error if no contract was found at the address
    //  */
    // getContractCodeHistory(address: string): Promise<readonly ContractCodeHistoryEntry[]>;
    // /**
    //  * Returns the data at the key if present (raw contract dependent storage data)
    //  * or null if no data at this key.
    //  *
    //  * Promise is rejected when contract does not exist.
    //  */
    // queryContractRaw(address: string, key: Uint8Array): Promise<Uint8Array | null>;
    // /**
    //  * Makes a smart query on the contract, returns the parsed JSON document.
    //  *
    //  * Promise is rejected when contract does not exist.
    //  * Promise is rejected for invalid query format.
    //  * Promise is rejected for invalid response format.
    //  */
    // queryContractSmart(address: string, queryMsg: Record<string, unknown>): Promise<JsonObject>;
}

// #[async_trait]
// impl QueryCosmWasmClient for Client {}
//
// #[async_trait]
// impl rpc::Client for Client {
//     async fn perform<R>(&self, request: R) -> rpc::Result<R::Response>
//     where
//         R: SimpleRequest,
//     {
//         self.http_client.perform(request).await
//     }
// }
