// Copyright 2023 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use crate::nyxd::error::NyxdError;
use crate::nyxd::{CosmWasmClient, NyxdClient};

use nym_group_contract_common::msg::QueryMsg;

use async_trait::async_trait;
use cw4::MemberResponse;

#[async_trait]
pub trait GroupQueryClient {
    async fn member(&self, addr: String) -> Result<MemberResponse, NyxdError>;
}

#[async_trait]
impl<C: CosmWasmClient + Sync + Send> GroupQueryClient for NyxdClient<C> {
    async fn member(&self, addr: String) -> Result<MemberResponse, NyxdError> {
        let request = QueryMsg::Member {
            addr,
            at_height: None,
        };
        self.client
            .query_contract_smart(self.group_contract_address(), &request)
            .await
    }
}
