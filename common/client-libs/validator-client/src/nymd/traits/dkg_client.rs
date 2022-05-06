// Copyright 2022 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use crate::nymd::error::NymdError;
use crate::nymd::{NymdClient, SigningCosmWasmClient};
use async_trait::async_trait;
use coconut_dkg_common::msg::QueryMsg as DkgQueryMsg;
use coconut_dkg_common::types::Epoch;

#[async_trait]
pub trait DkgClient {
    async fn get_current_dkg_epoch(&self) -> Result<Epoch, NymdError>;
}

#[async_trait]
impl<C> DkgClient for NymdClient<C>
where
    C: SigningCosmWasmClient + Send + Sync,
{
    async fn get_current_dkg_epoch(&self) -> Result<Epoch, NymdError> {
        let request = DkgQueryMsg::GetCurrentEpoch {};
        self.client
            .query_contract_smart(self.coconut_dkg_contract_address()?, &request)
            .await
    }
}
