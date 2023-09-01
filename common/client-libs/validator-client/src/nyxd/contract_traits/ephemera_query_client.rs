// Copyright 2022 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use crate::collect_paged;
use crate::nyxd::contract_traits::NymContractsProvider;
use crate::nyxd::error::NyxdError;
use crate::nyxd::CosmWasmClient;
use async_trait::async_trait;
use nym_ephemera_common::msg::QueryMsg as EphemeraQueryMsg;
use nym_ephemera_common::peers::PagedPeerResponse;
use nym_ephemera_common::types::JsonPeerInfo;
use serde::Deserialize;

#[cfg_attr(target_arch = "wasm32", async_trait(?Send))]
#[cfg_attr(not(target_arch = "wasm32"), async_trait)]
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

// extension trait to the query client to deal with the paged queries
// (it didn't feel appropriate to combine it with the existing trait
#[cfg_attr(target_arch = "wasm32", async_trait(?Send))]
#[cfg_attr(not(target_arch = "wasm32"), async_trait)]
pub trait PagedEphemeraQueryClient: EphemeraQueryClient {
    async fn get_all_ephemera_peers(&self) -> Result<Vec<JsonPeerInfo>, NyxdError> {
        collect_paged!(self, get_peers_paged, peers)
    }
}

#[async_trait]
impl<T> PagedEphemeraQueryClient for T where T: EphemeraQueryClient {}

#[cfg_attr(target_arch = "wasm32", async_trait(?Send))]
#[cfg_attr(not(target_arch = "wasm32"), async_trait)]
impl<C> EphemeraQueryClient for C
where
    C: CosmWasmClient + NymContractsProvider + Send + Sync,
{
    async fn query_ephemera_contract<T>(&self, query: EphemeraQueryMsg) -> Result<T, NyxdError>
    where
        for<'a> T: Deserialize<'a>,
    {
        let ephemera_contract_address = &self
            .ephemera_contract_address()
            .ok_or_else(|| NyxdError::unavailable_contract_address("ephemera contract"))?;
        self.query_contract_smart(ephemera_contract_address, &query)
            .await
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::nyxd::contract_traits::tests::IgnoreValue;

    // it's enough that this compiles and clippy is happy about it
    #[allow(dead_code)]
    fn all_query_variants_are_covered<C: EphemeraQueryClient + Send + Sync>(
        client: C,
        msg: EphemeraQueryMsg,
    ) {
        match msg {
            EphemeraQueryMsg::GetPeers { limit, start_after } => {
                client.get_peers_paged(start_after, limit).ignore()
            }
        };
    }
}
