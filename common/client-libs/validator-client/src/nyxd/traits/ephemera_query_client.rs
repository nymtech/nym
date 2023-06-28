// Copyright 2023 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use crate::nyxd::error::NyxdError;
use crate::nyxd::{CosmWasmClient, NyxdClient};
use async_trait::async_trait;
use nym_ephemera_common::msg::QueryMsg as EphemeraQueryMsg;
use nym_ephemera_common::peers::PagedPeerResponse;
use serde::Deserialize;

#[async_trait]
pub trait EphemeraQueryClient {
    async fn query_ephemera_contract<T>(&self, query: EphemeraQueryMsg) -> Result<T, NyxdError>
    where
        for<'a> T: Deserialize<'a>;

    async fn get_peers_paged(
        &self,
        start_after: Option<String>,
        limit: Option<u32>,
    ) -> Result<PagedPeerResponse, NyxdError> {
        let request = EphemeraQueryMsg::GetPeers { start_after, limit };
        self.query_ephemera_contract(request).await
    }
}

#[async_trait]
impl<C> EphemeraQueryClient for NyxdClient<C>
where
    C: CosmWasmClient + Send + Sync,
{
    async fn query_ephemera_contract<T>(&self, query: EphemeraQueryMsg) -> Result<T, NyxdError>
    where
        for<'a> T: Deserialize<'a>,
    {
        self.client
            .query_contract_smart(self.ephemera_contract_address(), &query)
            .await
    }
}

#[async_trait]
impl<C> EphemeraQueryClient for crate::Client<C>
where
    C: CosmWasmClient + Sync + Send,
{
    async fn query_ephemera_contract<T>(&self, query: EphemeraQueryMsg) -> Result<T, NyxdError>
    where
        for<'a> T: Deserialize<'a>,
    {
        self.nyxd.query_ephemera_contract(query).await
    }
}
