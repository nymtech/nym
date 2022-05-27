// Copyright 2022 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use crate::coconut::error::Result;
use validator_client::nymd::{Fee, TxResponse};

#[async_trait]
pub trait Client {
    async fn get_tx(&self, tx_hash: &str) -> Result<TxResponse>;
    async fn propose_release_funds(
        &self,
        title: String,
        blinded_serial_number: String,
        voucher_value: u128,
        fee: Option<Fee>,
    ) -> Result<u64>;
}
