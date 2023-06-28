// Copyright 2023 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use crate::nyxd::cosmwasm_client::types::ExecuteResult;
use crate::nyxd::error::NyxdError;
use crate::nyxd::{Fee, NyxdClient, SigningCosmWasmClient};
use async_trait::async_trait;
use nym_ephemera_common::msg::ExecuteMsg as EphemeraExecuteMsg;
use nym_ephemera_common::types::JsonPeerInfo;

#[async_trait]
pub trait EphemeraSigningClient {
    async fn register_as_peer(
        &self,
        peer_info: JsonPeerInfo,
        fee: Option<Fee>,
    ) -> Result<ExecuteResult, NyxdError>;
}

#[async_trait]
impl<C> EphemeraSigningClient for NyxdClient<C>
where
    C: SigningCosmWasmClient + Send + Sync,
{
    async fn register_as_peer(
        &self,
        peer_info: JsonPeerInfo,
        fee: Option<Fee>,
    ) -> Result<ExecuteResult, NyxdError> {
        let req = EphemeraExecuteMsg::RegisterPeer { peer_info };

        self.client
            .execute(
                self.address(),
                self.ephemera_contract_address(),
                &req,
                fee.unwrap_or_default(),
                format!("registering {} as an ephemera peer", self.address()),
                vec![],
            )
            .await
    }
}
