// Copyright 2022 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use crate::coconut::error::{CoconutError, Result};
use crate::config::DEFAULT_LOCAL_VALIDATOR;

use validator_client::nymd::{tx::Hash, NymdClient, QueryNymdClient, TxResponse};

use async_trait::async_trait;

#[async_trait]
pub trait Client {
    async fn get_tx(&self, tx_hash: &str) -> Result<TxResponse>;
}

pub struct QueryClient {
    inner: NymdClient<QueryNymdClient>,
}

impl QueryClient {
    pub fn new() -> Result<Self> {
        let inner = NymdClient::connect(DEFAULT_LOCAL_VALIDATOR, None, None, None)?;
        Ok(Self { inner })
    }
}

#[async_trait]
impl Client for QueryClient {
    async fn get_tx(&self, tx_hash: &str) -> Result<TxResponse> {
        let tx_hash = tx_hash
            .parse::<Hash>()
            .map_err(|_| CoconutError::TxHashParseError)?;
        Ok(self.inner.get_tx(tx_hash).await?)
    }
}
