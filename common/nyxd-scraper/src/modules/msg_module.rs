// Copyright 2023 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use crate::block_processor::types::ParsedTransactionResponse;
use crate::error::ScraperError;
use crate::storage::StorageTransaction;
use async_trait::async_trait;
use cosmrs::Any;

#[async_trait]
pub trait MsgModule {
    async fn handle_msg(
        &mut self,
        index: usize,
        msg: &Any,
        tx: &ParsedTransactionResponse,
        storage_tx: &mut StorageTransaction,
    ) -> Result<(), ScraperError>;
}
