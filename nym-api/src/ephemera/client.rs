// Copyright 2023 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: GPL-3.0-only

use crate::ephemera::error::Result;
use nym_ephemera_common::types::JsonPeerInfo;
use nym_validator_client::nyxd::cosmwasm_client::types::ExecuteResult;

#[async_trait]
pub trait Client {
    async fn get_ephemera_peers(&self) -> Result<Vec<JsonPeerInfo>>;
    async fn register_ephemera_peer(&self, peer_info: JsonPeerInfo) -> Result<ExecuteResult>;
}
