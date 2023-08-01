// Copyright 2023 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use crate::nyxd::TendermintClient;
use async_trait::async_trait;
use tendermint_rpc::{Error, SimpleRequest};

pub struct WasmRpcClient {
    //
}

#[async_trait]
impl TendermintClient for WasmRpcClient {
    async fn perform<R>(&self, _request: R) -> Result<R::Output, Error>
    where
        R: SimpleRequest,
    {
        todo!()
    }
}
